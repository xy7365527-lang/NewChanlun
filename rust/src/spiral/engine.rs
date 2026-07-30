//! 螺旋引擎驱动核心：`step`（每 bar 群作用循环）+ `finish`（eod + 观测）。
//!
//! GUARD-ROLE: legacy-generation-loadbearing-for-fugue-v3——名分：现役（详见
//! `spiral/mod.rs` 头部 GUARD-ROLE 块，#762 C7-E3 核定）。
//!
//! 设计来源：架构 §5（每 bar 优先序 A→C→D→E→F）+ §9（批量/流式共享 step/finish ⇒
//! bit-exact 构造性保证）+ §11 Step 5。
//!
//! ## 每 bar 优先序（架构 §5，无 B 否定扫描——第11环"操作只在买卖点"）
//! ```text
//! A 强平兜底（逐空头 voice，会计终局，非群 gap G2）
//! → C 根 type1 平仓/翻转（同级别卖点，τ=ChiralSeam）
//! → D 回补（子 voice 子级别买点，σ⁻¹ 闭合）
//! → E 降成本 spawn（持仓期次级别卖点，σ⁻¹ Δr=−1）
//! → F 根入场（买点，σ 塔起点）
//! ```
//!
//! ## "操作=群作用"在驱动路径的显形
//! - C 翻转：`GroupAction::ChiralSeam.apply(voice.state)`（φ=0 翻 ε，§5.2）。
//! - E spawn：`accounting::try_spawn_cost_gated` 内 `σ⁻¹∘τ`（Δr=−1 + 方向交替）。
//! - 每次 CrossLevel 闭合经 `prove_cross_level_closure`（Δr=−1 硬断言）。
//!
//! ## 验收 = prove 守卫（非回测，架构 §2.4）
//! 群关系 L0（启动自检）+ N1–N8 + theta_sigma + cross_level + t14/a5/t1 每 bar panic +
//! T50/T56–T59 eod 观测。violation = panic = 验收标准失败。

use super::accounting::{close_voice, make_root, nav, settle, try_spawn_cost_gated};
use super::params::{
    EQUITY_SAMPLE_BARS, FIRST_BSP_LADDER, INITIAL_CAPITAL, MAX_LADDER, PENDING_LO,
};
use super::prove::{
    prove_a5_relabel, prove_all_group_laws, prove_chirality_seam, prove_cross_level_closure,
    prove_n1_forest, prove_n2_per_voice, prove_n4_cost_gate, prove_n7_spawn_self_level,
    prove_n8_conservation, prove_self_level_symmetric, prove_sub_level_symmetric,
    prove_t14_root_flip, prove_t1_no_voluntary_exit, prove_t50_radial_scaling,
    prove_t56_angular_radial_holonomy, prove_t57_chirality_mirror, prove_t58_angular_basic_domain,
    prove_t59_scale_invariance, self_level_counter_fire, sub_level_counter_fire,
};
use super::result::SpiralResult;
use super::signal::{prove_chain, prove_t52_gauge_fix, SignalState};
use super::state::GroupAction;
use super::voice::{SpiralVoice, VoiceStatus};
use crate::buysellpoint::Side;
use crate::stroke::Direction;
use crate::trading::tape::BarSig;
use crate::trading::types::Polarity;

/// 根 voice 涌现归属级别 E*（T5/A5 第20环）：根操作级别随走势向更高级别发展向上
/// 单调生长——会计重组（重新读数），不触发物理交易。
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
    assert!(
        lad >= root_ladder,
        "A5(T30) 违反：根涌现层 {lad} < 入场层 {root_ladder}（爬升应单调）"
    );
    lad
}

/// 流式/批量共享的螺旋引擎核心（架构 §9：push_bar 与批量共享 `step`）。
pub struct SpiralEngineCore {
    res: SpiralResult,
    voices: Vec<SpiralVoice>,
    free: f64,
    n_base: f64,
    signal: SignalState,
    max_children_seen: usize,
    cur_bar: i64,
    last_close: f64,
    finished: bool,
}

impl SpiralEngineCore {
    /// 初始化（零操作参数引擎；`floor_ladder` 仅作结构递归基断言 = FIRST_BSP_LADDER）。
    /// 启动时运行群关系 L0 自检（`prove_all_group_laws`）——群结构是引擎先验。
    pub fn new(floor_ladder: usize) -> Result<Self, String> {
        if floor_ladder != FIRST_BSP_LADDER {
            return Err(format!(
                "螺旋引擎是零操作参数引擎：floor_ladder 仅作结构递归基 = FIRST_BSP_LADDER={FIRST_BSP_LADDER}；得 {floor_ladder}"
            ));
        }
        // L0 群关系自检（h²³=σ / τ²=e / τhτ⁻¹=h⁻¹）——违反即引擎不合法，启动 panic。
        prove_all_group_laws();
        Ok(SpiralEngineCore {
            res: SpiralResult::default(),
            voices: Vec::new(),
            free: INITIAL_CAPITAL,
            n_base: 0.0,
            signal: SignalState::new(),
            max_children_seen: 0,
            cur_bar: 0,
            last_close: f64::NAN,
            finished: false,
        })
    }

    /// 单 bar 推进。`flip_edge` = 本 bar 方向翻转沿。每 bar 优先序 A→F + prove。
    pub fn step(&mut self, sig: &BarSig, flip_edge: &[Option<Direction>; MAX_LADDER]) {
        let bar = self.cur_bar;
        let c = sig.close;
        self.last_close = c;

        // ── 信号层 → 群事件帧（nest/cascade/located/向心 confirm + N3/N5/T52/T53 守卫）──
        let frame = self.signal.process(sig, flip_edge, bar, &mut self.res);
        // located/方向锚 Copy 出（操作期读，F/C 后经 signal.clear_* 清链）。
        let located_sell = self.signal.located_sell;
        let located_buy = self.signal.located_buy;
        let dir_state = self.signal.dir_state;
        let anchor_state = self.signal.anchor_state;
        let nf_sell = frame.nf_sell;
        let nf_buy = frame.nf_buy;
        let max_l = frame.max_l;

        let nav_pre = nav(&self.voices, self.free, c);
        let mut acted_ids: Vec<usize> = Vec::new();
        let was_active = self.voices.iter().any(|v| v.is_active());
        let mut root_liquidated = false;

        // ── A. 强平兜底（逐活跃空头 voice；会计终局，市场被动——非群 gap G2）──
        let n_voices = self.voices.len();
        for id in 0..n_voices {
            if !self.voices[id].is_active() || self.voices[id].dir() != Polarity::Short {
                continue;
            }
            let v = &self.voices[id];
            // 1x 逐仓 capital 耗尽：capital + units×(basis − c) ≤ 0。
            if v.capital + v.units * (v.basis - c) <= 0.0 {
                let lad = v.ladder();
                let is_root = v.parent.is_none();
                // 强平记账价 = 2×basis（SUB_LIQ_FACTOR；trade 行展示，不影响现金守恒）。
                let liq_px = super::params::SUB_LIQ_FACTOR * v.basis;
                close_voice(
                    id,
                    bar,
                    liq_px,
                    c,
                    "liq",
                    false,
                    &mut self.voices,
                    &mut self.free,
                    &mut self.n_base,
                    &mut self.res,
                );
                self.res.n_liquidations_by_ladder[lad] += 1;
                if is_root {
                    root_liquidated = true;
                }
                acted_ids.push(id);
            }
        }

        // ── C. 根清仓/翻转（τ=ChiralSeam；T5/A5 涌现重组 + T14 双向翻转）──
        let mut cleared = false;
        let root_id = self.voices.iter().position(|v| {
            v.is_active() && v.parent.is_none() && v.units > 0.0 && v.acted_bar != bar
        });
        if let Some(rid) = root_id {
            let root_dir = self.voices[rid].dir();
            // T5/A5 会计重组（root_emergent_ladder 向上单调升级，纯 relabel）。
            let re = root_emergent_ladder(
                self.voices[rid].ladder(),
                self.voices[rid].entry_bar,
                root_dir,
                &dir_state,
                &anchor_state,
                max_l,
            );
            if re > self.voices[rid].ladder() {
                let units_pre_re = self.voices[rid].units;
                let nav_pre_re = nav(&self.voices, self.free, c);
                self.voices[rid].state.r = re as u8; // 会计重组：无物理交易
                prove_a5_relabel(
                    self.voices[rid].units,
                    units_pre_re,
                    nav(&self.voices, self.free, c),
                    nav_pre_re,
                    bar,
                );
            }
            let root_ladder = self.voices[rid].ladder();
            let active_count = self.voices.iter().filter(|v| v.is_active()).count();
            let single_root = active_count == 1;
            match root_dir {
                Polarity::Long => {
                    if let Some(s) = frame.sell_source {
                        if s >= root_ladder && sig.sell1.get(s) {
                            prove_chain(&located_sell, Side::Sell, s, bar, "C-flip/clear");
                            prove_t52_gauge_fix(&located_sell, s, bar, "C-flip/clear");
                            if single_root && root_ladder > FIRST_BSP_LADDER {
                                // T14 根翻空：长→空 in-place（τ=ChiralSeam，φ=0）。
                                let m = self.voices[rid].units;
                                settle(&mut self.voices, rid, bar, c, "flip_short", &mut self.res);
                                self.free += m * c;
                                prove_chirality_seam(self.voices[rid].state.phi, bar);
                                self.voices[rid].state = GroupAction::ChiralSeam
                                    .apply(self.voices[rid].state)
                                    .expect("根 φ=0 ⇒ τ 合法");
                                self.voices[rid].basis = c;
                                self.voices[rid].capital = m * c;
                                self.voices[rid].cost_pool = m * c;
                                self.voices[rid].entry_bar = bar;
                                self.voices[rid].acted_bar = bar;
                                self.res.n_root_flips_by_ladder[root_ladder] += 1;
                                self.signal.clear_located_sell();
                                acted_ids.push(rid);
                                prove_t14_root_flip(&self.voices, rid, Polarity::Long, m, bar);
                                cleared = true;
                            } else if single_root {
                                unreachable!(
                                    "C-clear-at-base@bar {bar}：root_ladder={root_ladder}==FIRST_BSP_LADDER \
                                     但 N5 保证 root_ladder≥PENDING_LO>FIRST_BSP"
                                );
                            }
                        }
                    }
                }
                Polarity::Short => {
                    // T14 买点翻多：空→长 in-place（cover + rebuy 守恒）。
                    if let Some(s) = frame.buy_source {
                        if single_root && s >= root_ladder && sig.buy1.get(s) {
                            prove_chain(&located_buy, Side::Buy, s, bar, "C-flipback");
                            prove_t52_gauge_fix(&located_buy, s, bar, "C-flipback");
                            let m = self.voices[rid].units;
                            let cap = self.voices[rid].capital;
                            settle(&mut self.voices, rid, bar, c, "flip_long", &mut self.res);
                            self.free += cap - 2.0 * m * c;
                            prove_chirality_seam(self.voices[rid].state.phi, bar);
                            self.voices[rid].state = GroupAction::ChiralSeam
                                .apply(self.voices[rid].state)
                                .expect("根 φ=0 ⇒ τ 合法");
                            self.voices[rid].basis = c;
                            self.voices[rid].capital = 0.0;
                            self.voices[rid].cost_pool = m * c;
                            self.voices[rid].entry_bar = bar;
                            self.voices[rid].acted_bar = bar;
                            self.res.n_root_flips_by_ladder[root_ladder] += 1;
                            self.signal.clear_located_buy();
                            acted_ids.push(rid);
                            prove_t14_root_flip(&self.voices, rid, Polarity::Short, m, bar);
                            cleared = true;
                        }
                    }
                }
            }
        }

        // ── D. 回补（逐活跃非根 voice：次级别向心确认反向词汇 = 走势完美 ⇒ 隔离平仓返父，
        //        H¹ Δr=−1 向心下沉到 r=k−1；NR-1 否定同级别 σ=e 闭合——"确认在哪个级别，
        //        闭合就在哪个级别"，向心确认已回溯到 k−1，闭合也在 k−1）──
        if !cleared {
            let n = self.voices.len();
            for id in 0..n {
                if !self.voices[id].can_act(bar) || self.voices[id].parent.is_none() {
                    continue;
                }
                let dir = self.voices[id].dir();
                let ladder = self.voices[id].ladder();
                let (perfected, close_lad) = sub_level_counter_fire(dir, ladder, &nf_sell, &nf_buy);
                prove_sub_level_symmetric(
                    dir, ladder, perfected, close_lad, &nf_sell, &nf_buy, bar,
                );
                if perfected {
                    if close_lad < ladder {
                        // 常态：闭合向心下沉到次级别 r=k−1（CrossLevel H¹ 生成元 Δr=−1）。
                        prove_cross_level_closure(ladder, close_lad);
                        self.res.cross_level_closures += 1;
                    } else {
                        // 边界：ladder==FIRST_BSP_LADDER 无次级别 ⇒ 同级别闭合（NR-5 递归基无像）。
                        prove_n7_spawn_self_level(ladder, close_lad, bar);
                    }
                    self.voices[id].acted_bar = bar;
                    close_voice(
                        id,
                        bar,
                        c,
                        c,
                        "recover",
                        true,
                        &mut self.voices,
                        &mut self.free,
                        &mut self.n_base,
                        &mut self.res,
                    );
                    acted_ids.push(id);
                }
            }
        }

        // ── E. 降成本 spawn（N7：逐活跃 voice 自层 counter nf fire ⇒ σ⁻¹ spawn 子 voice @ r−1）──
        if !cleared {
            let n = self.voices.len();
            for id in 0..n {
                if !self.voices[id].can_act(bar) {
                    continue;
                }
                let dir = self.voices[id].dir();
                let ladder = self.voices[id].ladder();
                let is_root = self.voices[id].parent.is_none();
                // T8：根空头是叶节点（MtM ⊥ 子空头 frozen，不嵌套降成本）。
                if is_root && dir == Polarity::Short {
                    continue;
                }
                let e_trigger = self_level_counter_fire(dir, ladder, &nf_sell, &nf_buy);
                prove_self_level_symmetric(dir, ladder, e_trigger, &nf_sell, &nf_buy, bar);
                if e_trigger {
                    prove_n7_spawn_self_level(ladder, ladder, bar);
                    if try_spawn_cost_gated(
                        id,
                        bar,
                        c,
                        &self.signal.depth,
                        &mut self.voices,
                        &mut self.res,
                    )
                    .is_some()
                    {
                        self.voices[id].acted_bar = bar;
                        acted_ids.push(id);
                    }
                }
            }
        }

        // ── F. 根入场（森林空 ∧ 未清仓本 bar）：买链 source ∧ buy1[S] ⇒ S 层满仓开多（σ 塔起点）──
        let any_active = self.voices.iter().any(|v| v.is_active());
        let f_eligible = !cleared
            && !any_active
            && frame.buy_source.is_some_and(|s| sig.buy1.get(s))
            && self.free > 0.0
            && (self.free / c).is_finite();
        if !cleared && !any_active {
            if let Some(s) = frame.buy_source {
                if sig.buy1.get(s) {
                    let units = self.free / c;
                    if units > 0.0 && units.is_finite() {
                        prove_chain(&located_buy, Side::Buy, s, bar, "F-entry");
                        prove_t52_gauge_fix(&located_buy, s, bar, "F-entry");
                        self.voices.push(make_root(s, units, c, bar));
                        self.n_base = units;
                        self.free = 0.0;
                        self.res.n_root_entries_by_ladder[s] += 1;
                        self.res.n_entries_by_ladder[s] += 1;
                        self.signal.clear_located_buy();
                    }
                }
            }
        }

        // ── 必然性运行时证明（每 bar；violation = panic）──
        prove_n2_per_voice(&acted_ids, bar);
        let nav_post = nav(&self.voices, self.free, c);
        prove_n8_conservation(&self.voices, self.n_base, nav_pre, nav_post, bar);
        self.max_children_seen = self
            .max_children_seen
            .max(prove_n1_forest(&self.voices, bar));

        let active_count = self.voices.iter().filter(|v| v.is_active()).count();
        prove_t1_no_voluntary_exit(
            was_active,
            active_count > 0,
            root_liquidated,
            f_eligible && active_count == 0,
            bar,
        );

        // 观测：物理暴露 + 各层持有 bar。
        let mut long_units = 0.0;
        let mut short_units = 0.0;
        for v in self.voices.iter().filter(|v| v.is_active()) {
            match v.dir() {
                Polarity::Long => long_units += v.units,
                Polarity::Short => short_units += v.units,
            }
            self.res.held_bars_by_ladder[v.ladder()] += 1;
            if v.dir() == Polarity::Short {
                self.res.short_held_bars_by_ladder[v.ladder()] += 1;
            }
        }
        if long_units > 0.0 {
            self.res.phys_long_bars += 1;
        }
        if short_units > 0.0 {
            self.res.phys_short_bars += 1;
        }

        if bar % EQUITY_SAMPLE_BARS == 0 {
            self.res.equity.push((bar, nav(&self.voices, self.free, c)));
        }
        self.cur_bar += 1;
    }

    /// 收尾（末 bar equity 补采样 + eod cascade 关根 + N4 反证 + T50/T56–T59 观测）。幂等。
    pub fn finish(&mut self) {
        if self.finished {
            return;
        }
        self.finished = true;
        if self.cur_bar > 0 {
            let last_bar = self.cur_bar - 1;
            let c_last = self.last_close;
            if last_bar % EQUITY_SAMPLE_BARS != 0 {
                self.res
                    .equity
                    .push((last_bar, nav(&self.voices, self.free, c_last)));
            }
            if let Some(root_id) = self
                .voices
                .iter()
                .position(|v| v.is_active() && v.parent.is_none())
            {
                close_voice(
                    root_id,
                    last_bar,
                    c_last,
                    c_last,
                    "eod",
                    false,
                    &mut self.voices,
                    &mut self.free,
                    &mut self.n_base,
                    &mut self.res,
                );
            }
        }
        self.res.final_nav = self.free;
        // N4（成本门，eod 反证）：floor_stop 恒 0。
        prove_n4_cost_gate(&self.res);
        self.res.max_children = self.max_children_seen as u64;
        // T50/T56–T59 eod 观测（T56/T58 含结构 panic，T50/T57/T59 ~观测）。
        self.res.t50_monotone_violations = prove_t50_radial_scaling(&self.res);
        self.res.t56_radial_coverage = prove_t56_angular_radial_holonomy(&self.res);
        self.res.t57_onesided_layers = prove_t57_chirality_mirror(&self.res);
        self.res.t58_active_levels = prove_t58_angular_basic_domain(&self.res);
        self.res.t59_degenerate_layers = prove_t59_scale_invariance(&self.res);
        let _ = (FIRST_BSP_LADDER, PENDING_LO, VoiceStatus::Active); // 锚定 import（防未用警告）
    }

    /// 已累计 trade 数（push_bar 切出本 bar 新增）。
    pub fn n_trades(&self) -> usize {
        self.res.trades.len()
    }

    /// 结果只读。
    pub fn result(&self) -> &SpiralResult {
        &self.res
    }

    /// 状态快照: (cur_bar, nav, long_units, short_units, n_active_voices)。
    pub fn snapshot(&self) -> (i64, f64, f64, f64, usize) {
        let c = if self.last_close.is_finite() {
            self.last_close
        } else {
            0.0
        };
        let navv = nav(&self.voices, self.free, c);
        let mut lu = 0.0;
        let mut su = 0.0;
        let mut n = 0;
        for v in self.voices.iter().filter(|v| v.is_active()) {
            match v.dir() {
                Polarity::Long => lu += v.units,
                Polarity::Short => su += v.units,
            }
            n += 1;
        }
        (self.cur_bar, navv, lu, su, n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trading::tape::BarSig;
    use crate::trading::types::{BspClass, BspEvent, LadderMask, INITIAL_CAPITAL};

    /// 构造一个测试 BarSig。`events` = (ladder, class, confirmed, price)。
    fn mk_bar(
        close: f64,
        buy1: u16,
        sell1: u16,
        max_ladder: u8,
        events: Vec<(usize, BspClass, bool, f64)>,
    ) -> BarSig {
        let mut sig = BarSig {
            close,
            buy1: LadderMask(buy1),
            sell1: LadderMask(sell1),
            max_ladder,
            ..Default::default()
        };
        if !events.is_empty() {
            let mut arr: [Vec<BspEvent>; MAX_LADDER] = Default::default();
            for (lad, class, confirmed, price) in events {
                arr[lad].push(BspEvent {
                    class,
                    seg_idx: lad as i64,
                    confirmed,
                    cs: None,
                    zd: None,
                    zg: None,
                    price,
                });
            }
            sig.bsp_events = Some(Box::new(arr));
        }
        sig
    }

    const NO_FLIP: [Option<Direction>; MAX_LADDER] = [None; MAX_LADDER];

    #[test]
    fn empty_bars_are_noop_conservation_holds() {
        // 全空信号：引擎 no-op，守恒 + 全 prove 零 panic，final_nav = 初始资本。
        let mut core = SpiralEngineCore::new(FIRST_BSP_LADDER).expect("floor=FIRST_BSP");
        for _ in 0..5 {
            core.step(&mk_bar(100.0, 0, 0, 0, vec![]), &NO_FLIP);
        }
        core.finish();
        assert!(core.result().trades.is_empty(), "无事件不应产生 trade");
        assert!(
            (core.result().final_nav - INITIAL_CAPITAL).abs() < 1e-6,
            "无操作 final_nav 应 = 初始资本，得 {}",
            core.result().final_nav
        );
    }

    #[test]
    fn f_entry_then_eod_close_conserves_nav() {
        // F 入场全路径：bar0 武装区间套（segment type1 + move candidate），bar1 向心
        // confirm + buy1@move ⇒ F 满仓开多 @ move(L1)；flat 至 eod cascade 关根。
        // 全程 prove_n8（Σunits=N_base + NAV 中性）/ prove_chain / prove_t52 / N1/N2/T1
        // / prove_n4 / T56/T58 零 panic（守恒 = 验收）。
        let mut core = SpiralEngineCore::new(FIRST_BSP_LADDER).expect("floor=FIRST_BSP");
        // bar0：segment(2) type1 Buy confirmed（记 type1_hist 内圈）+ move(3) type1 Buy
        //       candidate（武装 nest_buy[3]，since=0）。
        core.step(
            &mk_bar(
                100.0,
                0,
                0,
                3,
                vec![
                    (2, BspClass::Buy1, true, 100.0),
                    (3, BspClass::Buy1, false, 100.0),
                ],
            ),
            &NO_FLIP,
        );
        // bar1：since(0)<bar(1) ∧ helix 母线贯通（内圈 segment type1@bar0）⇒ confirm@3
        //       ⇒ 级联 located[2,3] source=3；buy1 bit3 ⇒ F 满仓开多 @ move(L1)。
        core.step(&mk_bar(100.0, 1 << 3, 0, 3, vec![]), &NO_FLIP);
        // F 入场已发生：根 @ ladder 3，满仓 units=1000。
        let (_, nav_after_entry, long_u, short_u, n_voices) = core.snapshot();
        assert_eq!(n_voices, 1, "F 入场后应有 1 个 active 根 voice");
        assert!(
            (long_u - 1000.0).abs() < 1e-6,
            "满仓 units=free/c=1000，得 {long_u}"
        );
        assert_eq!(short_u, 0.0);
        assert!(
            (nav_after_entry - INITIAL_CAPITAL).abs() < 1e-4,
            "F 入场 NAV 中性"
        );
        assert_eq!(
            core.result().n_root_entries_by_ladder[3],
            1,
            "根入场记 @ ladder 3"
        );
        // flat 持仓 + eod cascade 关根（同价 100 ⇒ pnl=0 ⇒ free 返还 100000）。
        core.step(&mk_bar(100.0, 0, 0, 3, vec![]), &NO_FLIP);
        core.step(&mk_bar(100.0, 0, 0, 3, vec![]), &NO_FLIP);
        core.finish();
        assert_eq!(core.result().trades.len(), 1, "eod 关根产生 1 笔 trade");
        let t = &core.result().trades[0];
        assert_eq!(t.exit_reason, "eod");
        assert_eq!(t.ladder, 3);
        assert!((t.shares - 1000.0).abs() < 1e-6);
        assert!(
            (core.result().final_nav - INITIAL_CAPITAL).abs() < 1e-4,
            "同价进出 final_nav 应 = 初始资本（pnl=0），得 {}",
            core.result().final_nav
        );
    }
}
