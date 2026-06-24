//! **递归 T 流式回测器**：真实 OHLC 流 → iterate 真树 → extract_chain → RecDriver.on_view。
//!
//! 与 flat `stream::TFugueStreamCore`（驱动 TPositionEngine）并存——本模块驱动递归 `RecDriver`：
//! 复用同一 orchestrator（产笔/段）+ 段门控重跑 + online MACD 前缀和（口径逐字一致，segments
//! bit-exact），但重跑产物经 `extract_chain` 投影为操作链视图，喂递归 driver（每级别一 T 实例树）。
//!
//! ## 因果性（无 look-ahead）
//! BSP/走势完成直到**段确认那根 bar** 才被算出/投放，`on_view` 在确认时点 close 执行 reconcile
//! ⟹ 因果（除不可消除的确认滞后本身）。与 flat 流式同口径。
//!
//! ## 认识论等级
//! 流式接入 = **L1**（管线正确性，与 flat 共享 orch/MACD/段门控）；回测 alpha + 节点身份跨重跑
//! 稳定性（§8.6 前缀冻结/N4）= **L2/L3 待验证**——这正是本回测器要回答的。

use crate::macd::OnlineMacdState;
use crate::orchestrator::RecursiveOrchestrator;
use crate::segment::SegKind;
use crate::trading::types::INITIAL_CAPITAL;

use super::iterate;
use super::rec_driver::{extract_view, RecDriver};
use super::rec_engine::{LevelView, MAX_LEVEL};
use super::stream::build_a0_fast;
use super::types::{A0Source, BSPKind, PerfectionMode};

/// 诊断：BSPKind → 索引（0=t1buy 1=t1sell 2=t2buy 3=t2sell 4=t3buy 5=t3sell）。
fn bsp_kind_idx(k: crate::recursive_t::types::BSPKind) -> u8 {
    use crate::recursive_t::types::BSPKind::*;
    match k {
        Type1Buy => 0,
        Type1Sell => 1,
        Type2Buy => 2,
        Type2Sell => 3,
        Type3Buy => 4,
        Type3Sell => 5,
    }
}

/// 递归 T 流式核心（orchestrator + 重跑 + extract_chain + RecDriver，全 Rust 内聚）。
pub struct RecStream {
    orch: RecursiveOrchestrator,
    mode: PerfectionMode,
    /// a₀ 来源（线段=默认 bit-exact / 笔=递归底座下移，526号）。决定门控计数口径与 build_a0 过滤。
    a0_source: A0Source,
    driver: RecDriver,
    macd: OnlineMacdState,
    prefix_pos: Vec<f64>,
    prefix_neg: Vec<f64>,
    cur_bar: i64,
    last_close: f64,
    finished: bool,
    last_stroke_n: usize,
    /// 重跑触发 key = (a₀ 单元数, _)。Segment=confirmed&&settled 段数 / Stroke=confirmed 笔数。
    /// 第二位为区间套提前确认预留（当前回退=usize::MAX 固定）。
    last_trigger: (usize, usize),
    /// 重跑次数（性能 + 诊断）。
    pub n_reruns: u64,
    /// 诊断：chain[0]（最高级别走势）方向 bar 加权分布——定位 82% 持空根因（extract_chain vs 操作）。
    last_c0: i8, // 1=up, -1=down, 0=none
    pub c0_up_bars: u64,
    pub c0_down_bars: u64,
    /// 诊断：核心(root)方向 + 净敞口符号 bar 加权（区分核心方向错 vs sink/exposure 主导）。
    pub core_long_bars: u64,
    pub core_short_bars: u64,
    pub net_long_bars: u64,
    pub net_short_bars: u64,
    /// 诊断（编排者排查 2026-06-21）：核心(root highest_active)每 level 停留 bar + emergent_top 每 level
    /// bar——查核心是否到最高涌现级别（net_short 62.7% 疑核心没到最高级别被低级别 BSP flip）。
    pub core_level_bars: [u64; 9],
    pub emergent_level_bars: [u64; 9],
    last_emergent_level: i32, // -1=none
    /// 诊断（编排者 2026-06-21）：最后一次重跑 tree 各级别走势类型分布——查 L4 type1_buy=0
    /// 是否走势结构。[level][0=UpTrend,1=DownTrend,2=Consol-Up,3=Consol-Down]。
    pub tree_trend_stats: [[u64; 4]; 9],
    /// 诊断（编排者 2026-06-21，no_leave 根因）：跨重跑累加，逐级别 ConsolDown 切分性质——
    /// 验证 no_leave 盘整是「末组生长中走势」还是「中间走势被切掉离开段（segment bug）」。
    /// [level]: 0=ConsolDown总数 1=no_leave数 2=其中末组(gi+1==n_groups) 3=其中中间(gi+1<n,=segment bug)
    ///          4=no_leave且completed(被反向终结=apply_t标记) 5=no_leave且completed且无bsp。
    pub consoldn_split: [[u64; 6]; 9],
    /// 每级别**首个** no_leave ConsolDown 的完整结构 dump（units 长度/中枢范围/进入离开段/completed/gi）。
    pub first_no_leave_dump: [Option<String>; 9],
    /// 诊断（编排者回补质询）：全程 type1_buy 买点 (bar, price, level) dedup；
    /// 核心短头 episodes (entry_bar, entry_px, exit_bar, exit_px)——查做空后下跌段有无买点、是否被消费。
    pub t1buy: Vec<(i64, f64, usize)>,
    pub t1sell: Vec<(i64, f64, usize)>,
    pub short_episodes: Vec<(i64, f64, i64, f64)>,
    cur_short_entry: Option<(i64, f64)>,
    prev_core_dir: i8,
    /// 诊断：全 6 类 BSP 产出计数（dedup by bar+level+kind）+ 按级别分布——查引擎消费/未消费哪些。
    /// 索引: 0=t1buy 1=t1sell 2=t2buy 3=t2sell 4=t3buy 5=t3sell。
    pub bsp_counts: [u64; 6],
    pub bsp_by_level: [[u64; 6]; 10], // [level][kind]
    bsp_seen: std::collections::HashSet<(i64, usize, u8)>,
    /// 引擎变体配置（OFF / ANCHOR / NEST）。
    cfg: super::rec_engine::EngineConfig,
    // ── 命题4 读法乙顶层闸门测量（556 解冻判据：背驰段触发频率 vs type1 完成触发频率）──
    /// 重跑中最高级别处于**背驰段**（trend_candidate）的次数（armed reruns）。
    pub n_top_diverge_reruns: u64,
    /// 不同**背驰段窗口**数（未武装→武装跳变 = 大级别进入新背驰段的事件数）。
    pub n_top_diverge_windows: u64,
    /// 最高级别 **type1 完成**触发次数（重跑中 top level t1 fire）= 556 冻结的稀疏触发。
    pub n_top_type1: u64,
    /// 最高有走势级别分布（每重跑，诊断 top 级别随塔生长上移）。
    pub top_level_hist: [u64; 9],
    prev_top_armed: bool,
    /// 顶层背驰段失败归因（每重跑划分 top 走势状态，诊断 W=0 根因）：
    /// 0=无顶层走势 1=盘整 2=趋势<2中枢 3=≥2中枢但无c段 4=c段存在但结构滤网否 5=背驰段(armed)。
    pub nest_diag: [u64; 6],
}

impl RecStream {
    /// a₀ 来源默认 `Segment`（保 bit-exact）。
    pub fn new(mode: PerfectionMode) -> Self {
        Self::new_with_a0(mode, A0Source::Segment)
    }

    /// 显式指定 a₀ 来源（线段=基线 / 笔=递归底座下移，526号）。`new(mode)` 委托此构造默认 Segment。
    /// 引擎配置从 env 读（= 当前 main 行为，OFF 基线）。**rec≡flat bit-exact 守卫 + 合成单测 base**——
    /// **不改默认**（改默认会破 `rec_flat_btc_bit_exact` 等 OFF 守卫）。生产「默认开启 Face A」走
    /// `new_production`（FFI/python 回测入口），见 #113 spec §8.1。
    pub fn new_with_a0(mode: PerfectionMode, a0_source: A0Source) -> Self {
        Self::new_with_config(mode, a0_source, super::rec_engine::EngineConfig::from_env())
    }

    /// **生产/默认回测引擎（做空腿赚 #113，shortleg-profit-spec §8.1「默认开启」）**：纯级别×买卖点
    /// 统一引擎（Face A）为生产/默认配置。`EngineConfig::production()` 默认开启 Face A（无变体 env），
    /// env `T_OFF_BASELINE` ⇒ instances OFF 基线（bit-exact 回归守卫），显式实验变体 env ⇒ 尊重（受控
    /// 实验）。**FFI/python 回测入口走此**（`PyRecStream::new`）⇒ 生产默认开启 Face A。Rust 内部测试/
    /// bit-exact 守卫继续走 `new`/`new_with_a0`（OFF 基线不变）。
    pub fn new_production(mode: PerfectionMode, a0_source: A0Source) -> Self {
        Self::new_with_config(mode, a0_source, super::rec_engine::EngineConfig::production())
    }

    /// 显式引擎配置（OFF / ANCHOR / NEST 受控对照，单进程多变体）。
    pub fn new_with_config(
        mode: PerfectionMode,
        a0_source: A0Source,
        cfg: super::rec_engine::EngineConfig,
    ) -> Self {
        RecStream {
            // orch 配置与 flat stream / backtest_run 逐字一致（segments bit-exact）。
            orch: RecursiveOrchestrator::new(6, "wide", 5, false, 3, false, false, false),
            mode,
            a0_source,
            driver: RecDriver::new_with_config(INITIAL_CAPITAL, cfg),
            macd: OnlineMacdState::new(12, 26, 9),
            prefix_pos: vec![0.0],
            prefix_neg: vec![0.0],
            cur_bar: 0,
            last_close: f64::NAN,
            finished: false,
            last_stroke_n: 0,
            last_trigger: (0, usize::MAX),
            n_reruns: 0,
            last_c0: 0,
            c0_up_bars: 0,
            c0_down_bars: 0,
            core_long_bars: 0,
            core_short_bars: 0,
            net_long_bars: 0,
            net_short_bars: 0,
            core_level_bars: [0; 9],
            emergent_level_bars: [0; 9],
            last_emergent_level: -1,
            tree_trend_stats: [[0; 4]; 9],
            consoldn_split: [[0; 6]; 9],
            first_no_leave_dump: Default::default(),
            t1buy: Vec::new(),
            t1sell: Vec::new(),
            short_episodes: Vec::new(),
            cur_short_entry: None,
            prev_core_dir: 0,
            bsp_counts: [0; 6],
            bsp_by_level: [[0; 6]; 10],
            bsp_seen: std::collections::HashSet::new(),
            cfg,
            n_top_diverge_reruns: 0,
            n_top_diverge_windows: 0,
            n_top_type1: 0,
            top_level_hist: [0; 9],
            prev_top_armed: false,
            nest_diag: [0; 6],
        }
    }

    /// 逐 bar 推送。段门控触发重跑 → extract_view → driver.on_view（确认时点 close）。
    pub fn push_bar(&mut self, o: f64, h: f64, l: f64, c: f64) {
        self.orch.process_bar(o, h, l, c);
        // 增量 MACD + pos/neg 前缀和（区间面积 O(1) 查表基础，与 flat 同源）。
        let (_, _, hist) = self.macd.update(c);
        let lp = *self.prefix_pos.last().unwrap();
        let ln = *self.prefix_neg.last().unwrap();
        self.prefix_pos.push(lp + if hist > 0.0 { hist } else { 0.0 });
        self.prefix_neg.push(ln + if hist < 0.0 { -hist } else { 0.0 });
        let bar = self.cur_bar;
        self.last_close = c;

        // 段门控（区间套提前确认，编排者 2026-06-21）：笔增长 → 触发 key=(settled 段数, 最后 candidate
        // 端点 ep1_i)。candidate 出现/c 段顶部移动 → key 变 → 重跑（**顶部附近**），不等 settled（回调底）。
        // 每 bar 一个信号视图（无重跑=empty；重跑时填 nodes/emergent/fresh BSP）= flat：每 bar step。
        let mut view = LevelView::empty();
        let sc = self.orch.strokes().len();
        if sc > self.last_stroke_n {
            self.last_stroke_n = sc;
            let trig = {
                // a0_source 决定触发计数口径：Segment=confirmed&&settled 段 / Stroke=confirmed 笔。
                // 区间套提前确认试验回退（无效+每笔重跑慢）：第二位固定 usize::MAX。
                let count = match self.a0_source {
                    A0Source::Segment => self
                        .orch
                        .segments()
                        .iter()
                        .filter(|s| s.confirmed && s.kind == SegKind::Settled)
                        .count(),
                    A0Source::Stroke => self.orch.strokes().iter().filter(|s| s.confirmed).count(),
                };
                (count, usize::MAX)
            };
            if trig != self.last_trigger {
                self.last_trigger = trig;
                // 块内借 orch（segs+m2r）→ build_a0 → iterate → extract_chain（owned，块后释放借用）。
                // 诊断：同时捕获本树全 6 类 BSP（回补质询：查产出/消费）。
                let (v, new_bsps, trend_stats, cd_split, cd_dumps, top_div_info, level_div, d_top_arr, zd_arr, zg_arr) = {
                    let segs = self.orch.segments();
                    let strokes = self.orch.strokes();
                    let m2r = self.orch.merged_to_raw();
                    let a0 = build_a0_fast(
                        segs,
                        strokes,
                        m2r,
                        &self.prefix_pos,
                        &self.prefix_neg,
                        self.a0_source,
                        false,
                    );
                    let tree = iterate(a0, self.mode);
                    use crate::recursive_t::types::{Direction, TrendKind};
                    // 诊断（编排者 2026-06-21）：各级别走势类型分布（覆盖=最后一次重跑 tree）。
                    let mut ts = [[0u64; 4]; 9];
                    // no_leave 根因切分（复刻 judge_consolidation_divergence 判据）+ 首例结构 dump。
                    let mut cds = [[0u64; 6]; 9];
                    let mut dumps: [Option<String>; 9] = Default::default();
                    for lvl_out in &tree.levels {
                        let lv = lvl_out.level;
                        if lv >= 9 {
                            continue;
                        }
                        let n_groups = lvl_out.trends.len();
                        for (gi, tr) in lvl_out.trends.iter().enumerate() {
                            match (tr.kind, tr.direction) {
                                (TrendKind::UpTrend, _) => ts[lv][0] += 1,
                                (TrendKind::DownTrend, _) => ts[lv][1] += 1,
                                (TrendKind::Consolidation, Direction::Up) => ts[lv][2] += 1,
                                (TrendKind::Consolidation, Direction::Down) => ts[lv][3] += 1,
                            }
                            if tr.kind != TrendKind::Consolidation || tr.direction != Direction::Down {
                                continue;
                            }
                            cds[lv][0] += 1; // ConsolDown 总数
                            if tr.zhongshus.len() != 1 {
                                continue;
                            }
                            let center = match tr.zhongshus.first() {
                                Some(c) if !c.units.is_empty() => c,
                                _ => continue,
                            };
                            let enter_end = *center.units.last().unwrap();
                            if enter_end + 1 < tr.units.len() {
                                continue; // 有离开段，非 no_leave
                            }
                            cds[lv][1] += 1; // no_leave
                            let is_last = gi + 1 == n_groups;
                            if is_last {
                                cds[lv][2] += 1; // 末组（生长中走势）
                            } else {
                                cds[lv][3] += 1; // 中间走势无离开段（= segment 切分 bug 证据）
                            }
                            if tr.completed {
                                cds[lv][4] += 1; // no_leave 却被标 completed
                                if tr.bsp.is_none() {
                                    cds[lv][5] += 1; // 被反向终结（apply_t line 96）标 completed 无 bsp
                                }
                            }
                            if dumps[lv].is_none() {
                                dumps[lv] = Some(format!(
                                    "units.len={} 中枢units={:?} 进入段[0..={}] 离开段[{}..{}](空={}) completed={} has_bsp={} gi={}/{} 末组={}",
                                    tr.units.len(), center.units, enter_end,
                                    enter_end + 1, tr.units.len(), enter_end + 1 >= tr.units.len(),
                                    tr.completed, tr.bsp.is_some(), gi, n_groups, is_last,
                                ));
                            }
                        }
                    }
                    let bs: Vec<(i64, f64, usize, BSPKind)> = tree
                        .all_bsps()
                        .into_iter()
                        .map(|b| (b.bar, b.price, b.level, b.kind))
                        .collect();
                    // ── 命题4 读法乙：最高级别背驰段闸门（src-prop13 第27课区间套，背驰段⊊走势完成）──
                    //   最高有走势级别的当前（最后）走势：trend_candidate=结构∧MACD 双确认未创新高=背驰段。
                    //   上涨顶背驰段→Short（卖点 close+做空）/ 下跌底背驰段→Long（买点 cover+做多）。
                    let top_div_info = {
                        let top_lvl = tree.levels.iter().rposition(|l| !l.trends.is_empty());
                        match top_lvl.and_then(|tl| tree.levels[tl].trends.last().map(|t| (tl, t))) {
                            Some((tl, top_trend)) => {
                                let lvl = tree.levels[tl].level;
                                let dir = top_trend.direction;
                                let is_div = crate::recursive_t::divergence::trend_diverging_segment(top_trend, self.mode);
                                // 失败归因划分（诊断 W 根因；背驰段含趋势+盘整）。
                                let diag = if is_div {
                                    5 // 背驰段(armed)：趋势背驰段 ∨ 盘整背驰段
                                } else if !matches!(top_trend.kind, TrendKind::UpTrend | TrendKind::DownTrend) {
                                    1 // 盘整但非背驰段（无离开段/力度未衰减）
                                } else if top_trend.zhongshus.len() < 2 {
                                    2 // 趋势<2中枢
                                } else {
                                    let lc = top_trend.zhongshus.last().unwrap();
                                    let cs = lc.units.last().map(|i| i + 1).unwrap_or(usize::MAX);
                                    if cs >= top_trend.units.len() {
                                        3 // ≥2中枢但无c段
                                    } else {
                                        4 // c段存在但结构滤网否
                                    }
                                };
                                let div = if diag == 5 {
                                    Some(match dir {
                                        Direction::Up => crate::trading::types::Polarity::Short,
                                        Direction::Down => crate::trading::types::Polarity::Long,
                                    })
                                } else {
                                    None
                                };
                                (div, Some(dir), lvl, diag)
                            }
                            None => (None, None, 0usize, 0usize),
                        }
                    };
                    // ── 每级别背驰段（任务22 多重赋格 consume + 严格逐级区间套）──
                    //   level_div[level] = 该级别当前走势背驰段操作极性（顶背驰段→Short / 底背驰段→Long）。
                    let level_div = {
                        let mut ld = [None; MAX_LEVEL];
                        for lvl_out in &tree.levels {
                            let lv = lvl_out.level;
                            if lv >= MAX_LEVEL {
                                continue;
                            }
                            if let Some(t) = lvl_out.trends.last() {
                                if crate::recursive_t::divergence::trend_diverging_segment(t, self.mode) {
                                    ld[lv] = Some(match t.direction {
                                        Direction::Up => crate::trading::types::Polarity::Short,
                                        Direction::Down => crate::trading::types::Polarity::Long,
                                    });
                                }
                            }
                        }
                        ld
                    };
                    // ── 每级别 d_top 区间套链贯通（任务18 编排者修正：读法B/读法乙递归每级别独立腿触发器）──
                    //   d_top[k] = 级别 k 走势真顶/真底（区间套链贯通到 a0 = 走势完成真顶 = type1买卖点+区间套nesting全深度）。
                    //   reading_b（单腿 switch 触发器 `g`）+ reading_b_pair（任务69：LegPair **核心多腿 churn 门控**=最深区间套
                    //   确认保护主力骑牛）两路径消费（其余模式 d_top 全 false，bit-exact）。LegPair 次级别开空腿改用 `t3sell`
                    //   （第三类卖点=单层区间套转折，响应回调）⇒ 区间套确认深度按持仓尺度分级（主力 d_top 全深度 / 短差 t3sell 单层）。
                    let d_top_arr = if self.cfg.enable_reading_b || self.cfg.enable_reading_b_pair {
                        let level_trends_all: Vec<&[crate::recursive_t::types::TrendType]> = (0..MAX_LEVEL)
                            .map(|k| tree.levels.get(k).map(|lvl| lvl.trends.as_slice()).unwrap_or(&[]))
                            .collect();
                        let mut dt = [false; MAX_LEVEL];
                        for k in 0..MAX_LEVEL {
                            if let Some(cc_trend) = tree.levels.get(k).and_then(|lvl| lvl.trends.last()) {
                                dt[k] = crate::recursive_t::divergence::d_top(
                                    k, cc_trend, &level_trends_all, cc_trend.direction, self.mode, self.cfg.reading_b_diverge,
                                );
                            }
                        }
                        dt
                    } else {
                        [false; MAX_LEVEL]
                    };
                    // ── 否定线原料（Face B 做空腿赚 #110，§5.3.2/§6.2 567 封顶）：每级别**进场中枢核心区间**
                    //   [ZD,ZG]=末中枢 [low,high]（Zhongshu.low=ZD 核心下沿 / .high=ZG 核心上沿，types.rs:147-150）。
                    //   多腿进场否定线=该级别中枢 ZD（跌破=结构破坏止损）；空腿=ZG（涨破止损）。核心多腿（cc）否定线
                    //   = cc 级别中枢 ZD ⟹ 次级别回调（次级别卖点）不触发核心否定线（次级别 pullback 未破 cc 级 ZD）。
                    //   **仅 enable_uniform_sizing（Face B）填充**——RB_PAIR/OFF/单腿路径 zd/zg 恒 None（pair_stop_loss_step
                    //   死代码不动）⇒ 逐字不变（bit-exact）。删 geom_tower 恒仓必须同步接真否定线否则来源A 杠杆穿仓。
                    let (zd_arr, zg_arr) = if self.cfg.enable_uniform_sizing {
                        let mut zd = [None; MAX_LEVEL];
                        let mut zg = [None; MAX_LEVEL];
                        for k in 0..MAX_LEVEL {
                            if let Some(t) = tree.levels.get(k).and_then(|lvl| lvl.trends.last()) {
                                if let Some(zs) = t.zhongshus.last() {
                                    // ZG>ZD 成立条件（types.rs §1.3）；非法中枢（ZG≤ZD）不作否定线（None=仅靠反向买卖点平）。
                                    if zs.high > zs.low {
                                        zd[k] = Some(zs.low);
                                        zg[k] = Some(zs.high);
                                    }
                                }
                            }
                        }
                        (zd, zg)
                    } else {
                        ([None; MAX_LEVEL], [None; MAX_LEVEL])
                    };
                    (extract_view(&tree), bs, ts, cds, dumps, top_div_info, level_div, d_top_arr, zd_arr, zg_arr)
                };
                view = v;
                // 命题4 读法乙闸门注入 LevelView（engine nest_step 消费；OFF/ANCHOR 忽略 ⇒ bit-exact）。
                let (top_div, top_dir, top_lvl_val, top_diag) = top_div_info;
                view.top_diverge = top_div;
                view.top_trend_dir = top_dir;
                view.level_diverge = level_div;
                view.d_top = d_top_arr;
                view.zd = zd_arr; // Face B 否定线原料（enable_uniform_sizing 才非全 None，否则 bit-exact）。
                view.zg = zg_arr;
                if top_diag < 6 {
                    self.nest_diag[top_diag] += 1;
                }
                // 累加全程（每次重跑 tree 的走势类型 + no_leave 切分），首例结构只记一次。
                for lv in 0..9 {
                    for i in 0..4 {
                        self.tree_trend_stats[lv][i] += trend_stats[lv][i];
                    }
                    for i in 0..6 {
                        self.consoldn_split[lv][i] += cd_split[lv][i];
                    }
                    if self.first_no_leave_dump[lv].is_none() {
                        if let Some(d) = &cd_dumps[lv] {
                            self.first_no_leave_dump[lv] = Some(d.clone());
                        }
                    }
                }
                // **只消费本次重跑新 fire 的 BSP**（fresh）→ LevelView buy/sell per level（= flat
                // TSignalView diff）。根因修复：all_bsps 全历史累积，存在性检查会机械触发；fresh 是本 bar diff。
                for (bb, bp, bl, kind) in new_bsps {
                    let bk = bsp_kind_idx(kind);
                    if self.bsp_seen.insert((bb, bl, bk)) {
                        self.bsp_counts[bk as usize] += 1;
                        if bl < 10 {
                            self.bsp_by_level[bl][bk as usize] += 1;
                        }
                        if bk == 0 {
                            // 记**流式确认时点 bar**（= cur_bar），同 sink/recover 坐标。
                            self.t1buy.push((bar, bp, bl));
                        } else if bk == 1 {
                            self.t1sell.push((bar, bp, bl));
                        }
                        // fresh BSP → buy/sell（buy: bk 偶 0/2/4；sell: bk 奇 1/3/5，不分 type1/2/3）。
                        // 额外：type1（bk=0 买/1 卖 = 底/顶背驰=走势终完美）单独标记，供核心走势完成
                        // 清仓（546号死锁解锁，与 flat stream 对称：同源 fresh 去重子集 ⇒ bit-exact）。
                        if bl < MAX_LEVEL {
                            if bk % 2 == 0 {
                                view.buy[bl] = true;
                            } else {
                                view.sell[bl] = true;
                            }
                            if bk == 0 {
                                view.t1buy[bl] = true;
                            } else if bk == 1 {
                                view.t1sell[bl] = true;
                            } else if bk == 5 {
                                view.t3sell[bl] = true;
                            }
                        }
                    }
                }
                self.n_reruns += 1;
                // ── 命题4 读法乙 556 测量：背驰段触发频率 vs type1 完成触发频率（决定 src-prop13 vs 假消解）──
                let top_armed = view.top_diverge.is_some();
                if top_armed {
                    self.n_top_diverge_reruns += 1;
                    if !self.prev_top_armed {
                        self.n_top_diverge_windows += 1; // 大级别进入新背驰段事件
                    }
                    if top_lvl_val < 9 {
                        self.top_level_hist[top_lvl_val] += 1;
                    }
                }
                self.prev_top_armed = top_armed;
                // 最高级别 type1 完成（556 冻结的稀疏触发）：top level t1 本重跑 fire。
                if top_lvl_val < MAX_LEVEL && (view.t1buy[top_lvl_val] || view.t1sell[top_lvl_val]) {
                    self.n_top_type1 += 1;
                }
                // 诊断：本次最高级别走势方向（emergent_top 极性）。
                self.last_c0 = match view.emergent_top {
                    Some((_, crate::trading::types::Polarity::Long)) => 1,
                    Some((_, crate::trading::types::Polarity::Short)) => -1,
                    None => 0,
                };
                self.last_emergent_level = match view.emergent_top {
                    Some((lvl, _)) => lvl as i32,
                    None => -1,
                };
            }
        }
        // 每 bar 调 on_bar（= flat step 每 bar）：强平每 bar 检查 + emergence/route_bsp 仅在 view 有
        // BSP/emergent（重跑）时触发——empty view（无重跑）仅强平 + 守恒守卫，对照 flat step(empty)。
        self.driver.on_view(&view, c, bar);
        // bar 加权 chain[0] 方向（持续到下次重跑）。
        match self.last_c0 {
            1 => self.c0_up_bars += 1,
            -1 => self.c0_down_bars += 1,
            _ => {}
        }
        if self.last_emergent_level >= 0 && (self.last_emergent_level as usize) < 9 {
            self.emergent_level_bars[self.last_emergent_level as usize] += 1;
        }
        // bar 加权核心(root)方向 + 净敞口符号 + 核心短头 episode 追踪。
        let root = self.driver.root();
        let cur_dir: i8 = match root.highest_active() {
            Some(rs) => {
                if rs < 9 {
                    self.core_level_bars[rs] += 1; // 核心每 level 停留 bar（编排者排查）
                }
                match root.instance(rs).direction {
                    crate::trading::types::Polarity::Long => {
                        self.core_long_bars += 1;
                        1
                    }
                    crate::trading::types::Polarity::Short => {
                        self.core_short_bars += 1;
                        -1
                    }
                }
            }
            None => 0,
        };
        let (lu, su) = root.exposure();
        if lu - su > 1e-9 {
            self.net_long_bars += 1;
        } else if su - lu > 1e-9 {
            self.net_short_bars += 1;
        }
        // 核心短头 episode：进空记 entry，离空记 exit（查做空后下跌段买点产出/消费）。
        if cur_dir == -1 && self.prev_core_dir != -1 {
            self.cur_short_entry = Some((bar, c));
        } else if cur_dir != -1 && self.prev_core_dir == -1 {
            if let Some((eb, ep)) = self.cur_short_entry.take() {
                self.short_episodes.push((eb, ep, bar, c));
            }
        }
        self.prev_core_dir = cur_dir;
        self.cur_bar += 1;
    }

    /// 收尾（全平到现金 + 归还 withdrawn）。返回 final_nav。幂等。
    pub fn finish(&mut self) -> f64 {
        if self.finished {
            return self.driver.root().free();
        }
        self.finished = true;
        let c = if self.last_close.is_finite() { self.last_close } else { 0.0 };
        self.driver.finish(c)
    }

    pub fn final_nav(&self) -> f64 {
        self.driver.root().free()
    }
    pub fn driver(&self) -> &RecDriver {
        &self.driver
    }
    /// 快照: (cur_bar, total_wealth, n_active_instances)。
    pub fn snapshot(&self) -> (i64, f64, usize) {
        (self.cur_bar, self.driver.root().total_wealth(self.last_close), self.driver.root().n_active())
    }

    /// #149 per-element 会计 instrumentation（capture-ratio 验证工具，observation-only）。
    /// LegPair 逐笔账本：(level, entry_bar_raw, exit_bar_raw, entry_px, exit_px, units, is_short, pnl)。
    pub fn leg_trades(&self) -> &[(usize, i64, i64, f64, f64, f64, bool, f64)] {
        &self.driver.root().leg_trades
    }

    /// #149 per-level 多/空腿 realized pnl（pair_long_pnl, pair_short_pnl）——账本完整性自检的对账基准。
    pub fn pair_pnl(&self) -> (Vec<f64>, Vec<f64>) {
        let r = self.driver.root();
        (r.pair_long_pnl.to_vec(), r.pair_short_pnl.to_vec())
    }

    /// #149 per-element 走势段普查（capture-ratio 分母 Σ|Δ| 源，read-only）。
    /// 复刻 `iterate` 主循环，捕获每级别 `current`（apply_t 的输入构成单元 = 该级别走势段）；
    /// 与 Face A 引擎最后一次重跑同 a0（build_a0_fast）同 mode ⇒ 走势段与引擎信号同源。
    /// 返回 (level, raw_start, raw_end, high, low, is_up)。merged→raw 经 merged_to_raw 桥（backtest.rs 同口径）。
    pub fn level_segments(&self) -> Vec<(usize, i64, i64, f64, f64, bool)> {
        use super::types::Direction;
        let segs = self.orch.segments();
        let strokes = self.orch.strokes();
        let m2r = self.orch.merged_to_raw();
        let a0 = build_a0_fast(
            segs,
            strokes,
            m2r,
            &self.prefix_pos,
            &self.prefix_neg,
            self.a0_source,
            false,
        );
        // merged bar → raw bar（start 取 raw_start，end 取 raw_end；越界兜底=原值）。
        let conv = |b: i64, is_end: bool| -> i64 {
            match m2r.get(b as usize) {
                Some(&(rs, re)) => if is_end { re as i64 } else { rs as i64 },
                None => b,
            }
        };
        let mut out: Vec<(usize, i64, i64, f64, f64, bool)> = Vec::new();
        let mut current = a0;
        let mut k = 0usize;
        loop {
            for u in &current {
                out.push((
                    k,
                    conv(u.start_bar, false),
                    conv(u.end_bar, true),
                    u.high,
                    u.low,
                    matches!(u.direction, Direction::Up),
                ));
            }
            let o = super::apply_t(&current, k, self.mode);
            if o.trends.is_empty() {
                break;
            }
            let next = o.next_units.clone();
            if next.len() < 3 {
                break;
            }
            current = next;
            k += 1;
            if k > 64 {
                break;
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 递归流式合成序列零panic() {
        // 锯齿上行→顶→下行：流式喂应零 panic（TW 守恒守卫每操作）+ 产出有限 final_nav。
        let mut s = RecStream::new(PerfectionMode::Structural);
        let mut price = 100.0;
        let mut up = true;
        for i in 0..400 {
            if i % 20 == 0 {
                up = !up;
            }
            price += if up { 1.0 } else { -0.8 };
            let c = price;
            s.push_bar(c - 0.1, c + 0.5, c - 0.5, c);
        }
        let fin = s.finish();
        assert!(fin.is_finite() && fin > 0.0, "final_nav 有限正，得 {fin}");
    }

    #[test]
    fn 递归流式零bar不panic() {
        let mut s = RecStream::new(PerfectionMode::And);
        assert!((s.finish() - INITIAL_CAPITAL).abs() < 1e-6);
    }

    /// **递归流式笔底座零 panic + 有限正 final_nav**（TW 守恒守卫每操作，a0=笔 526号）。
    #[test]
    fn 递归流式笔底座零panic() {
        let mut s = RecStream::new_with_a0(PerfectionMode::Structural, A0Source::Stroke);
        let mut price = 100.0;
        let mut up = true;
        for i in 0..600 {
            if i % 20 == 0 {
                up = !up;
            }
            price += if up { 1.0 } else { -0.8 };
            let c = price;
            s.push_bar(c - 0.1, c + 0.5, c - 0.5, c);
        }
        let fin = s.finish();
        assert!(fin.is_finite() && fin > 0.0, "笔底座 final_nav 有限正，得 {fin}");
    }

    /// **递归流式 new 默认 = Segment 来源**：`new` ⟺ `new_with_a0(Segment)`（bit-exact final_nav + 重跑数）。
    #[test]
    fn 递归流式new默认等价segment来源() {
        fn run(s: &mut RecStream) -> f64 {
            let mut price = 100.0;
            let mut up = true;
            for i in 0..600 {
                if i % 20 == 0 {
                    up = !up;
                }
                price += if up { 1.0 } else { -0.8 };
                let c = price;
                s.push_bar(c - 0.1, c + 0.5, c - 0.5, c);
            }
            s.finish()
        }
        let mut a = RecStream::new(PerfectionMode::And);
        let mut b = RecStream::new_with_a0(PerfectionMode::And, A0Source::Segment);
        let fa = run(&mut a);
        let fb = run(&mut b);
        assert_eq!(fa, fb, "new 默认 Segment ⟺ new_with_a0(Segment)");
        assert_eq!(a.n_reruns, b.n_reruns, "重跑数一致");
    }

    /// **rec≡flat bit-exact（含核心走势完成清仓路径，546号死锁解锁对称改的硬证据）**。
    ///
    /// 死锁修复（核心走势完成 type1背驰 → clear_all，独立于 flip）落在 flat（`step`）与 rec（`on_bar`）
    /// 的**共享操作逻辑**，且触发信号（`t1buy`/`t1sell`）由 flat `stream` 与 rec `rec_stream` **同源
    /// fresh 去重子集**填充。本测试断言：同一合成 OHLC 序列上 flat `TFugueStreamCore` 与 rec
    /// `RecStream` 的 `final_nav` + `n_trades` **逐位一致**——即「绝对行为可变（新增清仓路径），但
    /// rec≡flat 正确性判据保持」（区别于被否定的取向Y=单边改 rec 弃 bit-exact）。
    #[test]
    fn rec_flat_bit_exact_含走势完成清仓路径() {
        use crate::recursive_t::stream::TFugueStreamCore;
        // 强方向锯齿宏观趋势（每 ~700 bar 一上一下大幅反转）+ 中摆动 + 快锯齿 → 多级别塔 + 高层
        // type1 背驰（核心走势完成 → 触发清仓路径）。两引擎共享 `iterate` 树 ⇒ 同结构/BSP/t1 标记，
        // 逐位一致是对称改的判据。宏观三角波制造可完成的高级别趋势（光滑正弦不产高层 type1）。
        fn synth(i: usize) -> (f64, f64, f64, f64) {
            let t = i as f64;
            let period = 700.0;
            let phase = (t % period) / period; // 0..1
            let tri = if phase < 0.5 { phase * 2.0 } else { 2.0 - phase * 2.0 }; // 0→1→0 三角波
            let macro_trend = 70.0 * tri; // 强方向宏观趋势（升 → 降）
            let mid = 12.0 * (t / 47.0).sin();
            let micro = 3.5 * (t / 13.0).sin();
            let c = 100.0 + macro_trend + mid + micro;
            (c - 0.1, c + 0.6, c - 0.6, c)
        }
        for &mode in &[PerfectionMode::Structural, PerfectionMode::And, PerfectionMode::Or] {
            let mut flat = TFugueStreamCore::new(mode);
            let mut rec = RecStream::new(mode);
            for i in 0..4200 {
                let (o, h, l, c) = synth(i);
                flat.push_bar(o, h, l, c);
                rec.push_bar(o, h, l, c);
            }
            flat.finish();
            let rec_nav = rec.finish();
            assert_eq!(
                flat.result().final_nav, rec_nav,
                "rec≡flat final_nav 逐位一致（mode={mode:?}，对称改证据）"
            );
            // 走势完成清仓路径在两引擎触发次数逐位一致（不变量：IF 触发则对称触发）。
            // 合成序列高层 type1 稀疏可能不触发新路径；新路径**确被触发的 bit-exact 硬证据**见
            // `rec_flat_btc_bit_exact`（真实 BTC，clears>0 且 flat==rec）。
            assert_eq!(
                flat.n_trend_done_clears(), rec.driver().root().n_trend_done_clears,
                "rec≡flat 走势完成清仓次数逐位一致（mode={mode:?}，新路径对称触发）"
            );
        }
    }

    /// **rec≡flat bit-exact 真实数据硬证据（走势完成清仓路径 clears>0，546号对称改判据）**。
    ///
    /// 合成序列高层 type1 稀疏不触发新路径；本测试在 BTC 1分钟全史上跑 flat `TFugueStreamCore` 与
    /// rec `RecStream`，断言：(1) `final_nav` 逐位一致；(2) 走势完成清仓次数逐位一致且 **>0**
    /// （新路径确被真实触发 ⇒ 对称改在新行为下仍 rec≡flat，区别于取向Y）。
    /// 跑法：`cargo test --release recursive_t::rec_stream::tests::rec_flat_btc_bit_exact -- --exact --ignored --nocapture`
    #[test]
    #[ignore = "rec≡flat BTC bit-exact（走势完成路径 clears>0），需 btc_1m_full.json"]
    fn rec_flat_btc_bit_exact() {
        use crate::recursive_t::backtest_run::load_clean_ohlc;
        use crate::recursive_t::stream::TFugueStreamCore;
        use std::path::PathBuf;
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("analysis/data_cache/btc_1m_full.json");
        if !path.exists() {
            eprintln!("数据缺失 {path:?}");
            return;
        }
        let (o, h, l, c) = load_clean_ohlc(&path);
        let n = c.len();
        let mut flat = TFugueStreamCore::new(PerfectionMode::Structural);
        let mut rec = RecStream::new(PerfectionMode::Structural);
        for i in 0..n {
            flat.push_bar(o[i], h[i], l[i], c[i]);
            rec.push_bar(o[i], h[i], l[i], c[i]);
        }
        flat.finish();
        let rec_nav = rec.finish();
        let flat_clears = flat.n_trend_done_clears();
        let rec_clears = rec.driver().root().n_trend_done_clears;
        eprintln!(
            "BTC rec≡flat: flat_nav={} rec_nav={} | 走势完成清仓 flat={} rec={}",
            flat.result().final_nav, rec_nav, flat_clears, rec_clears
        );
        assert_eq!(flat.result().final_nav, rec_nav, "rec≡flat final_nav 逐位一致（BTC，新路径活跃）");
        assert_eq!(flat_clears, rec_clears, "rec≡flat 走势完成清仓次数逐位一致（BTC）");
        assert!(flat_clears > 0, "走势完成清仓路径在 BTC 真实触发（clears>0 ⇒ bit-exact 覆盖新路径）");
    }

    /// **核心走势完成清仓单元（546号死锁解锁机制单测）**：构造核心 Long 后，喂该 level 的 type1 卖点
    /// （顶背驰=上涨走势终完美）→ 引擎应 `clear_all` 到现金（highest_active=None），下一买点重建。
    /// 直接驱动 `TRoot.on_bar`（不经 stream），隔离验证触发逻辑。
    #[test]
    fn 核心走势完成_type1卖点_清仓重建() {
        use crate::recursive_t::rec_engine::{LevelView, TRoot, TrendNode};
        use crate::recursive_t::types::Direction;
        use crate::trading::types::Polarity;
        let mut r = TRoot::new(100_000.0);
        // 1) 核心级买点 → enter Long @ level 3。
        let mut v = LevelView::empty();
        v.buy[3] = true;
        v.nodes[3] = Some(TrendNode::new(0, 10, 90.0, 110.0, Direction::Up));
        r.on_bar(&v, 10, 100.0);
        assert_eq!(r.highest_active(), Some(3), "核心建仓 @3");
        assert_eq!(r.instance(3).direction, Polarity::Long);
        let enters0 = r.n_enters;
        // 2) 核心 level 的 type1 卖点（顶背驰）→ 走势完成 → clear_all 到现金。
        let mut v2 = LevelView::empty();
        v2.sell[3] = true;
        v2.t1sell[3] = true; // type1（背驰=走势终完美）
        v2.nodes[3] = Some(TrendNode::new(10, 20, 95.0, 130.0, Direction::Up));
        r.on_bar(&v2, 20, 130.0);
        assert_eq!(r.highest_active(), None, "核心走势完成 → 全平到现金（死锁解锁）");
        assert!(!r.instance(3).is_active(), "level 3 清空");
        assert_eq!(r.n_flips, 0, "走势完成是清仓非 flip（不反向 enter）");
        // 3) 下一个买点 → enter 重建（highest_active 已 None ⇒ enter 路径激活）。
        let mut v3 = LevelView::empty();
        v3.buy[2] = true;
        v3.nodes[2] = Some(TrendNode::new(20, 30, 120.0, 140.0, Direction::Up));
        r.on_bar(&v3, 30, 135.0);
        assert_eq!(r.highest_active(), Some(2), "下一买点 enter 重建 @2");
        assert_eq!(r.n_enters, enters0 + 1, "n_enters 增长（死锁解除：重建路径激活）");
    }

    /// **非 type1 卖点不触发清仓**（走势完成判据严格性：仅 type1 背驰 = 走势终完美）：
    /// 核心 Long 收到该 level 的 type2/3 卖点（非背驰）不应清仓——否则退化为过度交易。
    #[test]
    fn 核心非type1卖点_不清仓() {
        use crate::recursive_t::rec_engine::{LevelView, TRoot, TrendNode};
        use crate::recursive_t::types::Direction;
        let mut r = TRoot::new(100_000.0);
        let mut v = LevelView::empty();
        v.buy[3] = true;
        v.nodes[3] = Some(TrendNode::new(0, 10, 90.0, 110.0, Direction::Up));
        r.on_bar(&v, 10, 100.0);
        // 该 level 卖点但**非 type1**（t1sell 不置位）→ 核心级反向 BSP → flip（既有路径），非走势完成清仓。
        let mut v2 = LevelView::empty();
        v2.sell[3] = true; // sell 但 t1sell=false
        v2.nodes[3] = Some(TrendNode::new(10, 20, 95.0, 130.0, Direction::Up));
        r.on_bar(&v2, 20, 130.0);
        // 走势完成路径未触发（t1sell=false）⇒ 走既有核心级反向 flip（highest_active 仍 Some=翻空后核心）。
        assert!(r.highest_active().is_some(), "非 type1 卖点不走走势完成清仓（走既有 flip 路径）");
        assert_eq!(r.n_flips, 1, "非 type1 反向 → flip（既有路径，未被走势完成清仓抢占）");
    }

    /// **盘整 no_leave 根因诊断**（编排者 2026-06-21）：流式 BTC Structural，逐级别切分 ConsolDown
    /// 的 no_leave 性质——验证「11784 个 L4 ConsolDown 全 no_leave」是 r* 边界生长中走势（末组、
    /// 不标 completed）还是 segment 切掉离开段（中间走势）/被反向终结误标 completed。
    /// 跑法：`cargo test --release recursive_t::rec_stream::tests::rec_btc_consol_no_leave -- --ignored --nocapture`
    #[test]
    #[ignore = "盘整 no_leave 诊断，需 analysis/data_cache/btc_1m_full.json"]
    fn rec_btc_consol_no_leave() {
        use crate::recursive_t::backtest_run::{load_clean_ohlc, SYMBOLS};
        use std::path::PathBuf;

        let data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("analysis/data_cache");
        for (sym, file) in SYMBOLS {
            if sym != "BTC" {
                continue;
            }
            let path = data_dir.join(file);
            if !path.exists() {
                eprintln!("[BTC] 数据缺失 {path:?}，跳过");
                return;
            }
            let (o, h, l, c) = load_clean_ohlc(&path);
            let n = c.len();
            let mut s = RecStream::new(PerfectionMode::Structural);
            for i in 0..n {
                s.push_bar(o[i], h[i], l[i], c[i]);
            }
            s.finish();
            eprintln!(
                "\n===== [BTC] 流式 ConsolDown no_leave 切分（Structural, bars={n} reruns={}）=====",
                s.n_reruns
            );
            eprintln!(
                "{:>5} {:>10} {:>10} {:>9} {:>10} {:>9} {:>12}",
                "level", "ConsolDn", "no_leave", "末组", "中间BUG", "compl", "compl无bsp"
            );
            for lv in 0..6 {
                let r = s.consoldn_split[lv];
                eprintln!(
                    "{:>5} {:>10} {:>10} {:>9} {:>10} {:>9} {:>12}",
                    lv, r[0], r[1], r[2], r[3], r[4], r[5]
                );
            }
            for lv in 0..6 {
                if let Some(d) = &s.first_no_leave_dump[lv] {
                    eprintln!("  L{lv} 首个 no_leave ConsolDown: {d}");
                }
            }
            eprintln!(
                "判读：'中间BUG'>0 ⟺ segment 切掉中间盘整离开段；'compl无bsp'>0 ⟺ no_leave 被反向终结误标 completed。"
            );
        }
    }

    /// 递归引擎 **8 标的 × 3 模式**回测（L3 验证）：vs flat 引擎（t_backtest_8x3）vs BH。
    /// 跑法：`cargo test --release recursive_t::rec_stream::tests::rec_btc -- --ignored --nocapture`
    /// `BT_SYMBOLS=CL,BTC` 过滤标的（默认全 8）。
    /// `T_A0=stroke` 切 a₀=笔（递归底座下移，526号；默认 segment=线段基线）——A/B 对比级别增量。
    #[test]
    #[ignore = "递归引擎全量回测，需 analysis/data_cache/*.json"]
    fn rec_btc() {
        use crate::recursive_t::backtest_run::{load_clean_ohlc, SYMBOLS};
        use std::path::PathBuf;

        let data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("analysis/data_cache");
        let only: Vec<String> = std::env::var("BT_SYMBOLS")
            .ok()
            .map(|s| s.split(',').map(|x| x.trim().to_uppercase()).filter(|x| !x.is_empty()).collect())
            .unwrap_or_default();
        // a₀ 来源 A/B 开关（默认 Segment=bit-exact 基线）。
        let a0_source = match std::env::var("T_A0").ok().as_deref() {
            Some("stroke") | Some("bi") => A0Source::Stroke,
            _ => A0Source::Segment,
        };

        eprintln!(
            "\n========== 递归 T 引擎 8 标的 × 3 模式回测（a₀={:?}）==========",
            a0_source
        );
        // (sym, bh, strat[3], short_pnl[3], net_short%[3], sink[3], freeshort[3])
        type Row = (String, f64, [f64; 3], [f64; 3], [f64; 3], [u64; 3], [u64; 3]);
        let mut rows: Vec<Row> = Vec::new();

        for (sym, file) in SYMBOLS {
            if !only.is_empty() && !only.contains(&sym.to_uppercase()) {
                continue;
            }
            let path = data_dir.join(file);
            if !path.exists() {
                eprintln!("[{sym}] 数据缺失 {path:?}，跳过");
                continue;
            }
            let (o, h, l, c) = load_clean_ohlc(&path);
            let n = c.len();
            let bh = if n > 0 && c[0] > 0.0 { (c[n - 1] / c[0] - 1.0) * 100.0 } else { 0.0 };
            let (mut strat, mut spnl, mut nshort) = ([0.0; 3], [0.0; 3], [0.0; 3]);
            let (mut sinks, mut fshort) = ([0u64; 3], [0u64; 3]);
            for (mi, mode) in [PerfectionMode::Structural, PerfectionMode::And, PerfectionMode::Or]
                .iter()
                .enumerate()
            {
                let t0 = std::time::Instant::now();
                let mut s = RecStream::new_with_a0(*mode, a0_source);
                for i in 0..n {
                    s.push_bar(o[i], h[i], l[i], c[i]);
                }
                let fin = s.finish();
                strat[mi] = (fin / INITIAL_CAPITAL - 1.0) * 100.0;
                let r = s.driver().root();
                spnl[mi] = r.short_leg_pnl;
                nshort[mi] = 100.0 * s.net_short_bars as f64 / n.max(1) as f64;
                sinks[mi] = r.n_sinks;
                fshort[mi] = r.n_flips;
                eprintln!(
                    "[{sym:<5}/{:>10?}] strat={:+.1}% bh={:+.1}% enter={} sink={} recover={} drain={} \
                     flip={} ascend={} short_pnl={:+.0} net_short={:.1}% liq={} | 三阶段 cap_rec={} earn={} \
                     earn_u={:.1} gain={:.1}% ({:.1}s)",
                    mode,
                    strat[mi],
                    bh,
                    r.n_enters,
                    r.n_sinks,
                    r.n_recovers,
                    r.n_drains,
                    r.n_flips,
                    r.n_ascends,
                    r.short_leg_pnl,
                    nshort[mi],
                    r.n_liquidations,
                    r.n_capital_recovered,
                    r.n_earning_deploys,
                    r.earning_units_added,
                    r.max_core_gain_x1000 as f64 / 10.0,
                    t0.elapsed().as_secs_f64()
                );
                // per-level 短差 P&L 验收（编排者方向 3：哪个级别短差赚/亏）。
                let spnl_lvl: Vec<i64> = (0..6).map(|k| r.short_pnl_by_level[k] as i64).collect();
                eprintln!("  [{sym:<5}/{mode:?}] short_pnl_by_level={:?}", spnl_lvl);
                // ── prove 守卫族读数（L2：让数据告诉我们哪些不变量被违反，编排者 2026-06-21）──
                // no_trigger=0 是 panic 守卫保证的（>0 早已 panic）；其余三项为观测计数。
                let g = r.guards();
                eprintln!(
                    "  [{sym:<5}/{mode:?}] PROVE ops(bsp/emrg/eod)={:?} no_trigger={} | dir_mismatch={}/{} \
                     | sink≠recover campaigns={}/{} 净失衡={} | neg_pnl campaigns={} by_level={:?}",
                    &g.ops_by_trigger[..3],
                    g.n_ops_without_trigger,
                    g.n_dir_mismatch,
                    g.n_dir_checks,
                    g.n_sink_recover_imbalance,
                    g.n_campaigns_checked,
                    g.sink_recover_imbalance_total,
                    g.n_neg_pnl_campaigns,
                    &g.neg_pnl_by_level[..6.min(g.neg_pnl_by_level.len())],
                );
                if sym == "BTC" {
                    use crate::trading::types::Polarity as Pol;
                    // 编排者排查 2026-06-21：核心是否到最高涌现级别 + emergence_upgrade 调用/成功/跳过。
                    eprintln!(
                        "  [{sym}/{mode:?}] emergence: attempts={} upgrades={} skipped_dir={} | n_flips={} n_ascends={}",
                        r.n_emergence_attempts, r.n_emergence_upgrades, r.n_emergence_skipped_dir, r.n_flips, r.n_ascends
                    );
                    let core_lvl: Vec<u64> = (0..9).map(|k| s.core_level_bars[k]).collect();
                    let emg_lvl: Vec<u64> = (0..9).map(|k| s.emergent_level_bars[k]).collect();
                    eprintln!("  [{sym}/{mode:?}] core_level_bars(核心停留)={core_lvl:?}");
                    eprintln!("  [{sym}/{mode:?}] emergent_level_bars(涌现级别)={emg_lvl:?}");
                    let trend_s: Vec<[u64; 4]> = (0..6).map(|k| s.tree_trend_stats[k]).collect();
                    eprintln!("  [{sym}/{mode:?}] tree_trend_stats[Up,Down,ConsolUp,ConsolDown]/lvl={trend_s:?}");
                    let consol_diag: Vec<[u64; 4]> = crate::recursive_t::divergence::CONSOL_DOWN_DIAG
                        .with(|d| (0..6).map(|k| d.borrow()[k]).collect());
                    eprintln!("  [{sym}/{mode:?}] ConsolDown判定[no_new_low,no_force,produced,no_leave]/lvl={consol_diag:?}");
                    crate::recursive_t::divergence::CONSOL_DOWN_DIAG.with(|d| *d.borrow_mut() = [[0; 4]; 9]);
                    eprintln!(
                        "  [{sym}/{mode:?}] core_long_bars={} core_short_bars={} c0_up={} c0_down={}",
                        s.core_long_bars, s.core_short_bars, s.c0_up_bars, s.c0_down_bars
                    );
                    // 核心 flip 序列：查低级别 ping-pong。
                    let n_l2s = r.flip_log.iter().filter(|(_, f, t, _, _)| *f == Pol::Long && *t == Pol::Short).count();
                    let n_s2l = r.flip_log.iter().filter(|(_, f, t, _, _)| *f == Pol::Short && *t == Pol::Long).count();
                    let flip_by_lvl: Vec<usize> = r.flip_log.iter().map(|(_, _, _, j, _)| *j).collect();
                    eprintln!(
                        "  [{sym}/{mode:?}] flips={} (Long→Short={n_l2s} Short→Long={n_s2l}) 触发级别={flip_by_lvl:?}",
                        r.flip_log.len()
                    );
                    let head: Vec<(i64, i8, usize)> = r.flip_log.iter().take(40)
                        .map(|(b, _, t, j, _)| (*b, if *t == Pol::Long { 1i8 } else { -1 }, *j)).collect();
                    eprintln!("  [{sym}/{mode:?}] flip序列前40 (bar,到向[1=L/-1=S],触发级别)={head:?}");
                    // sink/recover 路由对称性（编排者排查：为什么 recover<sink）。
                    let sink_lvl: Vec<u64> = (0..6).map(|k| r.sink_by_level[k]).collect();
                    let recover_lvl: Vec<u64> = (0..6).map(|k| r.recover_by_level[k]).collect();
                    eprintln!("  [{sym}/{mode:?}] sink_by_level={sink_lvl:?} recover_by_level={recover_lvl:?}");
                    eprintln!(
                        "  [{sym}/{mode:?}] 买点路由分类: recover={} noop(无短差浪费)={} sink(父空)={} core(核心级)={}",
                        r.buy_recover, r.buy_noop, r.buy_sink, r.buy_core
                    );
                    let sells: Vec<u64> = (0..6).map(|k| s.bsp_by_level[k][1]).collect();
                    let t1buy_lvl: Vec<u64> = (0..6).map(|k| s.bsp_by_level[k][0]).collect();
                    eprintln!(
                        "  [{sym}/{mode:?}] reruns={} t1sell_by_level={:?} t1buy_by_level={:?}",
                        s.n_reruns, sells, t1buy_lvl
                    );
                    // 编排者 2026-06-21：type2/type3 买卖点（UpTrend 自身产买点 → 可 recover 短差空头，
                    // 不需 DownTrend）。验收最高级别走势类型上的 type2/3 buy 数量。
                    let t2buy: Vec<u64> = (0..6).map(|k| s.bsp_by_level[k][2]).collect();
                    let t3buy: Vec<u64> = (0..6).map(|k| s.bsp_by_level[k][4]).collect();
                    let t2sell: Vec<u64> = (0..6).map(|k| s.bsp_by_level[k][3]).collect();
                    let t3sell: Vec<u64> = (0..6).map(|k| s.bsp_by_level[k][5]).collect();
                    eprintln!(
                        "  [{sym}/{mode:?}] t2buy_by_level={t2buy:?} t3buy_by_level={t3buy:?} | t2sell={t2sell:?} t3sell={t3sell:?}"
                    );
                    // 强平三阶段快照（编排者：查降成本/退本金保护 + 全仓 vs 逐仓）。
                    eprintln!(
                        "  [{sym}/{mode:?}] 强平三阶段快照(stage[0CostRed/1CapRec/2Earn],cost_basis,withdrawn,notional_in,nav)={:?}",
                        r.liq_snapshot
                    );
                    // 强平 episode 表（编排者：查回补失败=空头没匹配买点被强平）。
                    eprintln!(
                        "  [{sym}/{mode:?}] 强平 episode（{}个）lv|开空bar|开空价|强平bar|强平价|空?|期间任意级别买点",
                        r.liq_log.len()
                    );
                    for &(lv, eb, ep, lb, lp, short) in r.liq_log.iter() {
                        // 期间 (开空bar, 强平bar] 任意级别 type1_buy（流式确认 bar 坐标）。
                        let buys: Vec<(i64, f64, usize)> =
                            s.t1buy.iter().filter(|(b, _, _)| *b > eb && *b <= lb).cloned().collect();
                        let buy_lvls: Vec<usize> = buys.iter().map(|(_, _, l)| *l).collect();
                        // 同级别买点（lv）是否 fire（高级别空头需同级别买点 recover）。
                        let same_lvl = buys.iter().filter(|(_, _, l)| *l == lv).count();
                        eprintln!(
                            "    L{lv}|{eb}|{ep:.0}|{lb}|{lp:.0}|{}|{}个(同级别{})levels={:?}",
                            if short { "空" } else { "多" },
                            buys.len(),
                            same_lvl,
                            buy_lvls
                        );
                    }
                    // 编排者诊断：L2/L3/L4 type1_buy 的 bar 分布（确认高级别空头持仓期间有无同级别买点）。
                    for lvl in [2usize, 3, 4] {
                        let buys: Vec<i64> =
                            s.t1buy.iter().filter(|(_, _, l)| *l == lvl).map(|(b, _, _)| *b).collect();
                        let first = buys.first().copied().unwrap_or(-1);
                        let last = buys.last().copied().unwrap_or(-1);
                        let head: Vec<i64> = buys.iter().take(8).copied().collect();
                        eprintln!(
                            "    L{lvl} type1_buy={}个 最早bar={} 最晚bar={} 前8={:?}",
                            buys.len(),
                            first,
                            last,
                            head
                        );
                    }
                }
                assert!(fin.is_finite(), "[{sym}] final_nav 有限（NaN/Inf=会计 bug）");
            }
            rows.push((sym.to_string(), bh, strat, spnl, nshort, sinks, fshort));
        }

        // ── 汇总矩阵 ──
        println!("\n===== 递归 T 8×3 strat_pct 矩阵（区间套递归归还修复后）=====");
        println!("{:<6} {:>12} {:>12} {:>12} {:>11}", "标的", "Structural", "AND", "OR", "BH");
        for (sym, bh, strat, _, _, _, _) in &rows {
            let mark = |x: f64| if x > *bh { "*" } else { " " }; // * = 超 BH
            println!(
                "{:<6} {:>+10.1}%{} {:>+10.1}%{} {:>+10.1}%{} {:>+10.1}%",
                sym,
                strat[0],
                mark(strat[0]),
                strat[1],
                mark(strat[1]),
                strat[2],
                mark(strat[2]),
                bh
            );
        }
        println!("--- 做空腿 short_leg_pnl（S/A/O）+ net_short%（S）+ flip(S）---");
        for (sym, _, _, spnl, nshort, sinks, fshort) in &rows {
            println!(
                "{:<6} short_pnl S={:+.0} A={:+.0} O={:+.0} | net_short(S)={:.0}% sink(S)={} flip(S)={}",
                sym, spnl[0], spnl[1], spnl[2], nshort[0], sinks[0], fshort[0]
            );
        }
        println!("======================================================\n");
        assert!(!rows.is_empty(), "至少跑出一个标的");
    }

    /// **命题4 读法乙 L3 验证：OFF / ANCHOR / NEST 8 标的对照**（src-prop13 决定性测量）。
    ///
    /// 三变体（PerfectionMode::Structural 固定，受控对照单进程多变体）：
    /// - **OFF** = 当前 main 独立腿（cc 锚 + trend_done_clear）。
    /// - **ANCHOR** = OFF + 趋势底仓（552号 HOLD_ANCHOR）。
    /// - **NEST** = 命题4 读法乙（旁路 cc 锚，大级别背驰段闸门 a0 区间套定位翻转）。
    ///
    /// 决定性测量（556 是否解冻）：顶层**背驰段触发频率**（W=n_top_diverge_windows）vs
    /// **type1 完成触发频率**（T=n_top_type1）。W≫T ⇒ src-prop13（背驰段≠完成，真解冻）；
    /// W≈T ⇒ recursive-bsp（假消解，背驰段与完成同等稀疏）。
    ///
    /// 跑法：`cargo test --release recursive_t::rec_stream::tests::prop4_nest_l3 -- --exact --ignored --nocapture`
    /// `BT_SYMBOLS=CL,BTC` 过滤标的。
    #[test]
    #[ignore = "命题4 读法乙 L3 全量回测，需 analysis/data_cache/*.json"]
    fn prop4_nest_l3() {
        use super::super::rec_engine::EngineConfig;
        use crate::recursive_t::backtest_run::{load_clean_ohlc, SYMBOLS};
        use std::path::PathBuf;

        let data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("analysis/data_cache");
        let only: Vec<String> = std::env::var("BT_SYMBOLS")
            .ok()
            .map(|s| s.split(',').map(|x| x.trim().to_uppercase()).filter(|x| !x.is_empty()).collect())
            .unwrap_or_default();

        eprintln!("\n========== 命题4 读法乙 L3：OFF / ANCHOR / NEST（PerfectionMode::Structural）==========");
        // (sym, bh, strat[OFF,ANCHOR,NEST], W, T, armed_reruns, reruns, nest_flips, nest_short_pnl, off_short_pnl)
        struct Row {
            sym: String,
            bh: f64,
            strat: [f64; 3],
            w: u64,
            t: u64,
            armed: u64,
            reruns: u64,
            nest_flips: u64,
            nest_short_n: usize,
            nest_short_pnl: f64,
            nest_short_hold_avg: f64,
            nest_long_n: usize,
            nest_short_leg_pnl: f64,
            off_short_pnl: f64,
        }
        let mut rows: Vec<Row> = Vec::new();
        let variants = [("OFF", EngineConfig::off()), ("ANCHOR", EngineConfig::anchor()), ("NEST", EngineConfig::nest())];

        for (sym, file) in SYMBOLS {
            if !only.is_empty() && !only.contains(&sym.to_uppercase()) {
                continue;
            }
            let path = data_dir.join(file);
            if !path.exists() {
                eprintln!("[{sym}] 数据缺失 {path:?}，跳过");
                continue;
            }
            let (o, h, l, c) = load_clean_ohlc(&path);
            let n = c.len();
            let bh = if n > 0 && c[0] > 0.0 { (c[n - 1] / c[0] - 1.0) * 100.0 } else { 0.0 };
            let mut strat = [0.0f64; 3];
            let mut row = Row {
                sym: sym.to_string(), bh, strat: [0.0; 3], w: 0, t: 0, armed: 0, reruns: 0,
                nest_flips: 0, nest_short_n: 0, nest_short_pnl: 0.0, nest_short_hold_avg: 0.0,
                nest_long_n: 0, nest_short_leg_pnl: 0.0, off_short_pnl: 0.0,
            };

            for (vi, (vname, cfg)) in variants.iter().enumerate() {
                let t0 = std::time::Instant::now();
                let mut s = RecStream::new_with_config(PerfectionMode::Structural, A0Source::Segment, *cfg);
                for i in 0..n {
                    s.push_bar(o[i], h[i], l[i], c[i]);
                }
                let fin = s.finish();
                strat[vi] = (fin / INITIAL_CAPITAL - 1.0) * 100.0;
                let r = s.driver().root();
                assert!(fin.is_finite(), "[{sym}/{vname}] final_nav 非有限（会计 bug）");
                eprintln!(
                    "[{sym:<5}/{vname:<6}] strat={:+.1}% bh={:+.1}% enter={} flip={} nest_flip={} short_pnl={:+.0} \
                     liq={} reruns={} ({:.1}s)",
                    strat[vi], bh, r.n_enters, r.n_flips, r.n_nest_flips, r.short_leg_pnl,
                    r.n_liquidations, s.n_reruns, t0.elapsed().as_secs_f64()
                );
                if *vname == "OFF" {
                    row.off_short_pnl = r.short_leg_pnl;
                }
                if *vname == "NEST" {
                    row.w = s.n_top_diverge_windows;
                    row.t = s.n_top_type1;
                    row.armed = s.n_top_diverge_reruns;
                    row.reruns = s.n_reruns;
                    row.nest_flips = r.n_nest_flips;
                    row.nest_short_leg_pnl = r.short_leg_pnl; // 综合做空腿 P&L（含强平腿，539 主指标）
                    // NEST 逐笔做空腿（539 验收：长持死扣 vs 高频短持小亏）。
                    let shorts: Vec<&(bool, i64, f64, i64, f64, f64)> =
                        r.nest_trades.iter().filter(|t| t.0).collect();
                    let longs = r.nest_trades.iter().filter(|t| !t.0).count();
                    row.nest_short_n = shorts.len();
                    row.nest_short_pnl = shorts.iter().map(|t| t.5).sum();
                    row.nest_long_n = longs;
                    let hold_sum: f64 = shorts.iter().map(|t| (t.3 - t.1) as f64).sum();
                    row.nest_short_hold_avg = if shorts.is_empty() { 0.0 } else { hold_sum / shorts.len() as f64 };
                    let top_hist: Vec<u64> = (0..9).map(|k| s.top_level_hist[k]).collect();
                    let armed_pct = 100.0 * s.n_top_diverge_reruns as f64 / s.n_reruns.max(1) as f64;
                    eprintln!(
                        "  [{sym}/NEST] ★顶层闸门: 背驰段窗口W={} type1完成T={} | armed_reruns={}/{} (armed%={:.1}) | top_level_hist={:?}",
                        s.n_top_diverge_windows, s.n_top_type1, s.n_top_diverge_reruns, s.n_reruns, armed_pct, top_hist
                    );
                    eprintln!(
                        "  [{sym}/NEST] 顶层状态划分[0无走势,1盘整,2趋势<2中枢,3≥2中枢无c段,4c段存在结构否,5背驰段]={:?}",
                        s.nest_diag
                    );
                    // NEST 做空腿逐笔分布（持仓 bar 直方）。
                    let short_pnls: Vec<i64> = shorts.iter().map(|t| t.5 as i64).take(12).collect();
                    let short_holds: Vec<i64> = shorts.iter().map(|t| t.3 - t.1).take(12).collect();
                    eprintln!(
                        "  [{sym}/NEST] 做空腿: n={} 总pnl={:+.0} 平均持仓bar={:.0} | 多头腿 n={} | 前12短pnl={:?} 前12持bar={:?}",
                        shorts.len(), row.nest_short_pnl, row.nest_short_hold_avg, longs, short_pnls, short_holds
                    );
                }
            }
            row.strat = strat;
            rows.push(row);
        }

        // ── 汇总矩阵 ──
        println!("\n===== 命题4 读法乙 L3 矩阵（strat% OFF/ANCHOR/NEST vs BH，* = 超 BH）=====");
        println!("{:<6} {:>12} {:>12} {:>12} {:>11}", "标的", "OFF", "ANCHOR", "NEST", "BH");
        for r in &rows {
            let mk = |x: f64| if x > r.bh { "*" } else { " " };
            println!(
                "{:<6} {:>+10.1}%{} {:>+10.1}%{} {:>+10.1}%{} {:>+10.1}%",
                r.sym, r.strat[0], mk(r.strat[0]), r.strat[1], mk(r.strat[1]), r.strat[2], mk(r.strat[2]), r.bh
            );
        }
        // ── 556 解冻判据矩阵：W（背驰段窗口）vs T（type1完成）──
        println!("\n----- 556 顶层触发频率：W=背驰段窗口 vs T=type1完成（W≫T⇒src-prop13真解冻 / W≈T⇒假消解）-----");
        println!("{:<6} {:>8} {:>8} {:>10} {:>8} {:>10} {:>10}", "标的", "W", "T", "W/T", "nest_flip", "armed%", "reruns");
        for r in &rows {
            let wt = if r.t > 0 { format!("{:.2}", r.w as f64 / r.t as f64) } else { "∞/NaN".to_string() };
            let armed_pct = 100.0 * r.armed as f64 / r.reruns.max(1) as f64;
            println!(
                "{:<6} {:>8} {:>8} {:>10} {:>8} {:>9.1}% {:>10}",
                r.sym, r.w, r.t, wt, r.nest_flips, armed_pct, r.reruns
            );
        }
        // ── vs 539：NEST 做空腿逐笔 vs OFF 做空腿 ──
        println!("\n----- vs 539 做空腿：NEST（高频短持小亏？千刀凌迟？长持死扣？）vs OFF -----");
        println!("{:<6} {:>12} {:>10} {:>12} {:>10} {:>14}", "标的", "NEST空腿pnl(全)", "翻转n", "逐笔空n", "平均持bar", "OFF short_pnl");
        for r in &rows {
            println!(
                "{:<6} {:>+12.0} {:>10} {:>12} {:>12.0} {:>+14.0}",
                r.sym, r.nest_short_leg_pnl, r.nest_flips, r.nest_short_n, r.nest_short_hold_avg, r.off_short_pnl
            );
        }
        println!("====================================================================\n");
        assert!(!rows.is_empty(), "至少跑出一个标的");
    }

    /// **命题4 读法乙双向 consume平空 + 严格逐级区间套 L3（任务22）**：OFF / NEST / NEST_CONSUME /
    /// NEST_CONSUME_STRICT 4 变体。核心问题：**次级别买点平空（consume侧）是否解 prop4-nest 整仓长持
    /// 死扣穿仓（缩短持仓减强牛穿仓）+ 收益 + 解556保持？**
    ///
    /// - NEST = prop4-nest 基线（整仓骑到 top 反转才平，1-2 年死扣）。
    /// - NEST_CONSUME = + 次级别反核心向背驰段平核心仓 1/3（缩短持仓）。
    /// - NEST_CONSUME_STRICT = + 严格逐级区间套（主翻转定位点须 loc..top 逐级背驰段一致）。
    ///
    /// 跑法：`cargo test --release recursive_t::rec_stream::tests::prop4_consume_l3 -- --exact --ignored --nocapture`
    /// `BT_SYMBOLS=OKLO,DX,CL,BTC` 过滤（默认全 8）。
    #[test]
    #[ignore = "命题4 读法乙 consume L3，需 analysis/data_cache/*.json"]
    fn prop4_consume_l3() {
        use super::super::rec_engine::EngineConfig;
        use crate::recursive_t::backtest_run::{load_clean_ohlc, SYMBOLS};
        use std::path::PathBuf;

        let data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("analysis/data_cache");
        let only: Vec<String> = std::env::var("BT_SYMBOLS")
            .ok()
            .map(|s| s.split(',').map(|x| x.trim().to_uppercase()).filter(|x| !x.is_empty()).collect())
            .unwrap_or_default();

        eprintln!("\n===== 命题4 读法乙 consume平空 + 严格逐级 L3（任务22，Structural）=====");
        struct Row {
            sym: String,
            bh: f64,
            strat: [f64; 4], // OFF, NEST, NEST_CONSUME, NEST_CONSUME_STRICT
            flips: [u64; 4],
            consumes: [u64; 4],
            short_pnl: [f64; 4],
            hold_avg: [f64; 4], // NEST 系列做空腿平均持仓 bar（缩短持仓验收）
        }
        let mut rows: Vec<Row> = Vec::new();
        let variants = [
            ("OFF", EngineConfig::off()),
            ("NEST", EngineConfig::nest()),
            ("NEST_STRICT", EngineConfig::nest_strict()),
            ("NEST_CS_STR", EngineConfig::nest_consume_strict()),
        ];

        for (sym, file) in SYMBOLS {
            if !only.is_empty() && !only.contains(&sym.to_uppercase()) {
                continue;
            }
            let path = data_dir.join(file);
            if !path.exists() {
                eprintln!("[{sym}] 数据缺失，跳过");
                continue;
            }
            let (o, h, l, c) = load_clean_ohlc(&path);
            let n = c.len();
            let bh = if n > 0 && c[0] > 0.0 { (c[n - 1] / c[0] - 1.0) * 100.0 } else { 0.0 };
            let mut row = Row {
                sym: sym.to_string(), bh, strat: [0.0; 4], flips: [0; 4], consumes: [0; 4],
                short_pnl: [0.0; 4], hold_avg: [0.0; 4],
            };
            for (vi, (vname, cfg)) in variants.iter().enumerate() {
                let t0 = std::time::Instant::now();
                let mut s = RecStream::new_with_config(PerfectionMode::Structural, A0Source::Segment, *cfg);
                for i in 0..n {
                    s.push_bar(o[i], h[i], l[i], c[i]);
                }
                let fin = s.finish();
                assert!(fin.is_finite(), "[{sym}/{vname}] final_nav 非有限");
                let r = s.driver().root();
                row.strat[vi] = (fin / INITIAL_CAPITAL - 1.0) * 100.0;
                row.flips[vi] = r.n_nest_flips;
                row.consumes[vi] = r.n_nest_consumes;
                row.short_pnl[vi] = r.short_leg_pnl;
                let shorts: Vec<&(bool, i64, f64, i64, f64, f64)> = r.nest_trades.iter().filter(|t| t.0).collect();
                let hold_sum: f64 = shorts.iter().map(|t| (t.3 - t.1) as f64).sum();
                row.hold_avg[vi] = if shorts.is_empty() { 0.0 } else { hold_sum / shorts.len() as f64 };
                eprintln!(
                    "[{sym:<5}/{vname:<11}] strat={:+.1}% bh={:+.1}% flip={} consume={} consume_pnl={:+.0} short_pnl={:+.0} liq={} ({:.1}s)",
                    row.strat[vi], bh, r.n_nest_flips, r.n_nest_consumes, r.nest_consume_pnl, r.short_leg_pnl, r.n_liquidations, t0.elapsed().as_secs_f64()
                );
            }
            rows.push(row);
        }

        println!("\n===== 任务22 矩阵：strat% OFF/NEST/NEST_STRICT(开放轴C)/NEST_CS_STRICT vs BH（* 超 BH）=====");
        println!("{:<6} {:>10} {:>10} {:>13} {:>13} {:>10}", "标的", "OFF", "NEST", "NEST_STRICT", "+CONS_STRICT", "BH");
        for r in &rows {
            let mk = |x: f64| if x > r.bh { "*" } else { " " };
            println!(
                "{:<6} {:>+9.1}%{} {:>+9.1}%{} {:>+11.1}%{} {:>+12.1}%{} {:>+9.1}%",
                r.sym, r.strat[0], mk(r.strat[0]), r.strat[1], mk(r.strat[1]),
                r.strat[2], mk(r.strat[2]), r.strat[3], mk(r.strat[3]), r.bh
            );
        }
        println!("\n----- 归因：strict(开放轴C) vs consume 各自贡献 + 空腿失血 -----");
        println!("{:<6} {:>12} {:>13} {:>14} {:>12} {:>14}", "标的", "NEST short_pnl", "STRICT short_pnl", "CS_STR short_pnl", "STR flip", "CS consume数");
        for r in &rows {
            println!(
                "{:<6} {:>+12.0} {:>+13.0} {:>+14.0} {:>12} {:>14}",
                r.sym, r.short_pnl[1], r.short_pnl[2], r.short_pnl[3], r.flips[2], r.consumes[3]
            );
        }
        println!("==================================================================\n");
        assert!(!rows.is_empty(), "至少跑出一个标的");
    }

    /// **命题4 读法乙递归 L3（任务18 编排者修正）：每级别自相似递归多重赋格，触发器=背驰段替代走势完成**。
    ///
    /// 对照（隔离「换触发器」纯效果，结构=每级别独立腿不变）：
    /// - **OFF** = 当前 main 独立腿（cc 锚）。
    /// - **READING_B_DTOP** = 556 读法B（每级别独立腿 + **走势完成链** d_top 触发）→ 顶层腿冻结（完成跨年稀疏）。
    /// - **READING_B_DIVERGE** = 读法乙递归（每级别独立腿 + **背驰段链** 触发）→ 背驰段⊊完成更频繁，可能解冻顶层腿。
    ///
    /// 核心判据：每级别 leg_switches（背驰段 vs 完成触发频率，顶层应解冻）+ 收益 + per-level pnl。
    /// 跑法：`cargo test --release recursive_t::rec_stream::tests::prop4_reading_b_recursive_l3 -- --exact --ignored --nocapture`
    #[test]
    #[ignore = "命题4 读法乙递归 L3，需 analysis/data_cache/*.json"]
    fn prop4_reading_b_recursive_l3() {
        use super::super::rec_engine::EngineConfig;
        use crate::recursive_t::backtest_run::{load_clean_ohlc, SYMBOLS};
        use std::path::PathBuf;

        let data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("analysis/data_cache");
        let only: Vec<String> = std::env::var("BT_SYMBOLS").ok()
            .map(|s| s.split(',').map(|x| x.trim().to_uppercase()).filter(|x| !x.is_empty()).collect())
            .unwrap_or_default();

        eprintln!("\n===== 命题4 读法乙递归 L3：OFF / READING_B(走势完成) / READING_B(背驰段)（Structural）=====");
        struct Row {
            sym: String, bh: f64, strat: [f64; 3],
            // [DTOP, DIVERGE] 的 per-level switch（顶层解冻测量）+ 总 switch + liq。
            switch_by_lvl: [[u64; 6]; 2], total_switch: [u64; 2], liq: [u64; 2], net_pnl: [f64; 2],
        }
        let mut rows: Vec<Row> = Vec::new();
        let variants = [
            ("OFF", EngineConfig::off()),
            ("RB_DTOP", EngineConfig::reading_b_dtop()),
            ("RB_DIVERGE", EngineConfig::reading_b_diverge()),
        ];

        for (sym, file) in SYMBOLS {
            if !only.is_empty() && !only.contains(&sym.to_uppercase()) { continue; }
            let path = data_dir.join(file);
            if !path.exists() { eprintln!("[{sym}] 数据缺失，跳过"); continue; }
            let (o, h, l, c) = load_clean_ohlc(&path);
            let n = c.len();
            let bh = if n > 0 && c[0] > 0.0 { (c[n-1]/c[0]-1.0)*100.0 } else { 0.0 };
            let mut row = Row { sym: sym.to_string(), bh, strat: [0.0;3], switch_by_lvl: [[0;6];2], total_switch: [0;2], liq: [0;2], net_pnl: [0.0;2] };
            for (vi, (vname, cfg)) in variants.iter().enumerate() {
                let t0 = std::time::Instant::now();
                let mut s = RecStream::new_with_config(PerfectionMode::Structural, A0Source::Segment, *cfg);
                for i in 0..n { s.push_bar(o[i], h[i], l[i], c[i]); }
                let fin = s.finish();
                assert!(fin.is_finite(), "[{sym}/{vname}] final_nav 非有限");
                row.strat[vi] = (fin/INITIAL_CAPITAL - 1.0)*100.0;
                let r = s.driver().root();
                if vi >= 1 {
                    let bi = vi - 1; // DTOP=0, DIVERGE=1
                    let sw: Vec<u64> = (0..6).map(|k| r.leg_switches_by_level[k]).collect();
                    let total: u64 = (0..MAX_LEVEL).map(|k| r.leg_switches_by_level[k]).sum();
                    let npnl: f64 = (0..MAX_LEVEL).map(|k| r.per_level_long_pnl[k] + r.per_level_short_pnl[k]).sum();
                    for k in 0..6 { row.switch_by_lvl[bi][k] = sw[k]; }
                    row.total_switch[bi] = total;
                    row.liq[bi] = r.n_liquidations;
                    row.net_pnl[bi] = npnl;
                    eprintln!(
                        "[{sym:<5}/{vname:<11}] strat={:+.1}% bh={:+.1}% total_switch={} switch_by_lvl(0..6)={:?} liq={} net_pnl={:+.0} | max_gross={:.2}× max_net={:.2}× ({:.1}s)",
                        row.strat[vi], bh, total, sw, r.n_liquidations, npnl,
                        r.max_gross_exp_x100 as f64 / 100.0, r.max_net_exp_x100 as f64 / 100.0, t0.elapsed().as_secs_f64()
                    );
                } else {
                    eprintln!("[{sym:<5}/{vname:<11}] strat={:+.1}% bh={:+.1}% ({:.1}s)", row.strat[vi], bh, t0.elapsed().as_secs_f64());
                }
            }
            rows.push(row);
        }

        println!("\n===== 任务18 矩阵：strat% OFF/RB_DTOP(完成)/RB_DIVERGE(背驰段) vs BH（* 超 BH）=====");
        println!("{:<6} {:>10} {:>13} {:>14} {:>10}", "标的", "OFF", "RB_DTOP", "RB_DIVERGE", "BH");
        for r in &rows {
            let mk = |x: f64| if x > r.bh { "*" } else { " " };
            println!("{:<6} {:>+9.1}%{} {:>+12.1}%{} {:>+13.1}%{} {:>+9.1}%",
                r.sym, r.strat[0], mk(r.strat[0]), r.strat[1], mk(r.strat[1]), r.strat[2], mk(r.strat[2]), r.bh);
        }
        // ── ★556 解冻判据：顶层 leg 切换频率 完成 vs 背驰段 ──
        println!("\n----- ★556 解冻：每级别 leg 切换数 DTOP(走势完成) vs DIVERGE(背驰段)（背驰段≫完成⇒顶层解冻）-----");
        println!("{:<6} {:>16} {:>16} {:>16} {:>16}", "标的", "DTOP总switch", "DIVERGE总switch", "DTOP by_lvl", "DIVERGE by_lvl");
        for r in &rows {
            println!("{:<6} {:>16} {:>16} {:>16?} {:>16?}",
                r.sym, r.total_switch[0], r.total_switch[1], r.switch_by_lvl[0], r.switch_by_lvl[1]);
        }
        println!("==================================================================\n");
        assert!(!rows.is_empty(), "至少跑出一个标的");
    }

    /// **任务57=53.1：读法B 一对多空腿（LegPair）L3 验收**（编排者重写）。
    ///
    /// 模型：每级别 LegPair（多腿+空腿同时在场）。开=该级别买卖点（第17课）/ 平=反向买卖点 /
    /// 止损=否定线（进场中枢 ZG/ZD 破坏）/ 链破坏 churn 门控（第27课区间套：链完整=回调不动核心，链破坏=转折动核心）。
    ///
    /// 验收（编排者交付契约）：CL 跑通新腿模型——零 panic、**零强平**（否定线止损先于 NAV≤0 生效 ⇒ n_liquidations==0）、
    /// 守恒零违反（prove_tw_neutral 每 op 不 panic = 通过）、final_nav 有限正。
    /// 跑法：`BT_SYMBOLS=CL cargo test --release recursive_t::rec_stream::tests::prop4_reading_b_pair_l3 -- --exact --ignored --nocapture`
    #[test]
    #[ignore = "任务57 读法B LegPair L3，需 analysis/data_cache/*.json"]
    fn prop4_reading_b_pair_l3() {
        use super::super::rec_engine::EngineConfig;
        use crate::recursive_t::backtest_run::{load_clean_ohlc, SYMBOLS};
        use std::path::PathBuf;

        let data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("analysis/data_cache");
        let only: Vec<String> = std::env::var("BT_SYMBOLS").ok()
            .map(|s| s.split(',').map(|x| x.trim().to_uppercase()).filter(|x| !x.is_empty()).collect())
            .unwrap_or_default();

        eprintln!("\n===== 任务57=53.1：读法B 一对多空腿（LegPair）L3：OFF / READING_B_PAIR（Structural）=====");
        // leverage-accept（任务 leverage-accept，2026-06-23）：+RB_PAIR_T3 变体（=enable_pair_core_short_open=
        //   放开 below_core_long 门控，t3sell 核心/高级别开空 ⇒ 涌现 max_gross>1×）在 net-up 8标的对比基线。
        //   T3 是「接受杠杆」的**已存在**涌现机制（bear-validate 已 L3 测 bear 窗 1.27×/liq=0）；本测试在 net-up
        //   全量数据观测其 4 读数（做空腿净盈亏/liq/max_gross/vs BH）。无新硬编码倍数（杠杆=多腿叠加+空头收益
        //   膨胀 free 涌现，567 否定线 zg[k] 封顶单笔）。OFF/RB_PAIR 两 flag 默认 false ⇒ 路径逐字不变（bit-exact）。
        let variants = [
            ("OFF", EngineConfig::off()),
            ("RB_PAIR", EngineConfig::reading_b_pair()),
            ("RB_PAIR_T3", EngineConfig::reading_b_pair_coreshort_t3()),
        ];
        let mut any = false;
        for (sym, file) in SYMBOLS {
            if !only.is_empty() && !only.contains(&sym.to_uppercase()) { continue; }
            let path = data_dir.join(file);
            if !path.exists() { eprintln!("[{sym}] 数据缺失，跳过"); continue; }
            let (o, h, l, c) = load_clean_ohlc(&path);
            let n = c.len();
            let bh = if n > 0 && c[0] > 0.0 { (c[n-1]/c[0]-1.0)*100.0 } else { 0.0 };
            for (vname, cfg) in variants.iter() {
                let t0 = std::time::Instant::now();
                let mut s = RecStream::new_with_config(PerfectionMode::Structural, A0Source::Segment, *cfg);
                for i in 0..n { s.push_bar(o[i], h[i], l[i], c[i]); }
                let fin = s.finish();
                assert!(fin.is_finite(), "[{sym}/{vname}] final_nav 非有限");
                let strat = (fin/INITIAL_CAPITAL - 1.0)*100.0;
                let r = s.driver().root();
                if vname.starts_with("RB_PAIR") {
                    if *vname == "RB_PAIR" { any = true; }
                    let long_pnl: f64 = (0..MAX_LEVEL).map(|k| r.pair_long_pnl[k]).sum();
                    let short_pnl: f64 = (0..MAX_LEVEL).map(|k| r.pair_short_pnl[k]).sum();
                    let lopens: u64 = (0..MAX_LEVEL).map(|k| r.pair_long_opens[k]).sum();
                    let sopens: u64 = (0..MAX_LEVEL).map(|k| r.pair_short_opens[k]).sum();
                    eprintln!(
                        "[{sym:<5}/{vname:<7}] strat={strat:+.1}% bh={bh:+.1}% | long_opens={lopens} short_opens={sopens} \
                         long_pnl={long_pnl:+.0} short_pnl={short_pnl:+.0} | long_stops={} short_stops={} core_churns={} \
                         liq={} | max_gross={:.2}× max_net={:.2}× ({:.1}s)",
                        r.pair_long_stops, r.pair_short_stops, r.pair_core_churns, r.n_liquidations,
                        r.max_gross_exp_x100 as f64 / 100.0, r.max_net_exp_x100 as f64 / 100.0, t0.elapsed().as_secs_f64()
                    );
                    // 任务69 诊断（纯观测）：per-level 空腿 pnl/opens（判定「做空失血是级别问题还是 539 regime」）。
                    let sp_lvl: Vec<String> = (0..MAX_LEVEL)
                        .filter(|&k| r.pair_short_opens[k] > 0)
                        .map(|k| format!("L{k}:pnl={:+.0}/op={}", r.pair_short_pnl[k], r.pair_short_opens[k]))
                        .collect();
                    eprintln!("        [{sym}] short_per_level: {}", sp_lvl.join(" "));
                    // ── 验收硬断言（编排者交付契约）──
                    assert!(fin.is_finite() && fin > 0.0, "[{sym}/{vname}] final_nav 须有限正，得 {fin}");
                    // 零强平契约仅约束 baseline RB_PAIR（恒仓<1×）。RB_PAIR_T3（涌现杠杆 max_gross>1×）的 liq
                    // 是**被观测量**（leverage-accept 4 读数之一：否定线在 gross>1× 下是否仍封顶单笔），不硬断言。
                    if *vname == "RB_PAIR" {
                        assert_eq!(r.n_liquidations, 0, "[{sym}/RB_PAIR] 零强平违反（否定线止损应先于 NAV≤0）：liq={}", r.n_liquidations);
                    }
                    // 守恒零违反 = prove_tw_neutral 每 op 未 panic（运行到此即通过，无显式断言可加）。
                } else {
                    eprintln!("[{sym:<5}/{vname:<7}] strat={strat:+.1}% bh={bh:+.1}% ({:.1}s)", t0.elapsed().as_secs_f64());
                }
            }
        }
        assert!(any, "至少跑出一个标的的 RB_PAIR 变体");
    }

    /// **Face B（做空腿赚 #110 implB）L2 验证：核心不僵死 = 删 geom_tower 均匀定仓 + 真否定线**。
    ///
    /// 存在论位置（shortleg-profit-spec §5.3/§8.2 + mid-scale §14）：post-#69 中间级别失血 = **压制**
    /// （geom_tower 恒仓归一化把 max_gross 钳到 <1× ⟹ 中间级别敞口塌缩不吃自身|涨跌幅|）。Face B 删
    /// geom_tower → 均匀基准单元（来源A 杠杆涌现）+ 真否定线 [ZD,ZG]（RB_PAIR 下 zd/zg=None 死代码）封顶
    /// 每腿（liq=0）。**判据（§8.2，看符号非看 BH）**：① 做空腿 short_pnl 符号（vs RB_PAIR 压制基线）；
    /// ② liq=0（否定线真生效）；③ max_gross>1×（来源A 解压制）；④ per-level 敞口非塌缩（entries 非 30→3）。
    ///
    /// L2 等级（formalization-validity-domain）：单标的（BTC，默认）假设检验，可否证。否定性优先（§8.3）：
    /// liq>0 / short_pnl 仍负 / max_gross 仍<1× ⟹ 如实报告（缩小有效域 > 确认性主张），不硬断言掩盖。
    ///
    /// 跑法：`BT_SYMBOLS=BTC cargo test --release recursive_t::rec_stream::tests::faceb_l2_validate -- --exact --ignored --nocapture`
    #[test]
    #[ignore = "Face B L2（做空腿赚 #110），需 analysis/data_cache/*.json"]
    fn faceb_l2_validate() {
        use super::super::rec_engine::EngineConfig;
        use crate::recursive_t::backtest_run::{load_clean_ohlc, SYMBOLS};
        use std::path::PathBuf;

        let data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("analysis/data_cache");
        // 默认 BTC（L2 单标的锚，shortleg-profit-spec §8.2 / 死终止判据 2022 吃跌幅）。可 BT_SYMBOLS 覆盖。
        let only: Vec<String> = std::env::var("BT_SYMBOLS").ok()
            .map(|s| s.split(',').map(|x| x.trim().to_uppercase()).filter(|x| !x.is_empty()).collect())
            .unwrap_or_else(|| vec!["BTC".to_string()]);

        eprintln!("\n===== Face B（#110 implB）L2：OFF(instances基线) / RB_PAIR(geom_tower 压制) / FACE_B(均匀定仓+真否定线) =====");
        eprintln!("（判据：short_pnl 符号 + liq=0 + max_gross>1×解压制 + per-level 敞口非塌缩。L2 否定性优先）");
        // OFF=instances/sink 路径（mid-scale 8/8 失血 short_leg_pnl 基线）；RB_PAIR=LegPair geom_tower（post-#69 压制）；
        // FACE_B=删 geom_tower 均匀定仓 + 真否定线（解压制 + liq=0 同时成立）。
        let variants = [
            ("OFF",    EngineConfig::off()),
            ("RB_PAIR", EngineConfig::reading_b_pair()),
            ("FACE_B", EngineConfig::face_b()),
        ];
        let mut any = false;
        for (sym, file) in SYMBOLS {
            if !only.is_empty() && !only.contains(&sym.to_uppercase()) { continue; }
            let path = data_dir.join(file);
            if !path.exists() { eprintln!("[{sym}] 数据缺失，跳过"); continue; }
            let (o, h, l, c) = load_clean_ohlc(&path);
            let n = c.len();
            let bh = if n > 0 && c[0] > 0.0 { (c[n-1]/c[0]-1.0)*100.0 } else { 0.0 };
            for (vname, cfg) in variants.iter() {
                let t0 = std::time::Instant::now();
                let mut s = RecStream::new_with_config(PerfectionMode::Structural, A0Source::Segment, *cfg);
                for i in 0..n { s.push_bar(o[i], h[i], l[i], c[i]); }
                let fin = s.finish();
                assert!(fin.is_finite() && fin > 0.0, "[{sym}/{vname}] final_nav 须有限正，得 {fin}");
                let strat = (fin/INITIAL_CAPITAL - 1.0)*100.0;
                let r = s.driver().root();
                if *vname == "OFF" {
                    // OFF=instances/sink 路径：mid-scale 失血基线 short_leg_pnl + per-level short_pnl_by_level。
                    let sl: f64 = r.short_leg_pnl;
                    let spl: Vec<String> = (0..MAX_LEVEL).filter(|&k| r.short_pnl_by_level[k].abs() > 1.0)
                        .map(|k| format!("L{k}={:+.0}", r.short_pnl_by_level[k])).collect();
                    eprintln!("[{sym:<4}/OFF    ] strat={strat:+.1}% bh={bh:+.1}% | short_leg_pnl(sink路径)={sl:+.0} liq={} | per-lvl:[{}] ({:.0}s)",
                        r.n_liquidations, spl.join(" "), t0.elapsed().as_secs_f64());
                } else {
                    if *vname == "FACE_B" { any = true; }
                    let long_pnl: f64 = (0..MAX_LEVEL).map(|k| r.pair_long_pnl[k]).sum();
                    let short_pnl: f64 = (0..MAX_LEVEL).map(|k| r.pair_short_pnl[k]).sum();
                    let lopens: u64 = (0..MAX_LEVEL).map(|k| r.pair_long_opens[k]).sum();
                    let sopens: u64 = (0..MAX_LEVEL).map(|k| r.pair_short_opens[k]).sum();
                    eprintln!(
                        "[{sym:<4}/{vname:<7}] strat={strat:+.1}% bh={bh:+.1}% | l_op={lopens} s_op={sopens} \
                         long_pnl={long_pnl:+.0} short_pnl={short_pnl:+.0} | l_stop={} s_stop={} churn={} \
                         liq={} | max_gross={:.2}× net={:.2}× ({:.0}s)",
                        r.pair_long_stops, r.pair_short_stops, r.pair_core_churns, r.n_liquidations,
                        r.max_gross_exp_x100 as f64 / 100.0, r.max_net_exp_x100 as f64 / 100.0, t0.elapsed().as_secs_f64()
                    );
                    // per-level 空腿（中间级别解压制透镜，#108 口径：是否吃自身|涨跌幅|转正）。
                    let sp_lvl: Vec<String> = (0..MAX_LEVEL).filter(|&k| r.pair_short_opens[k] > 0)
                        .map(|k| format!("L{k}:sp={:+.0}/op={}", r.pair_short_pnl[k], r.pair_short_opens[k])).collect();
                    let lp_lvl: Vec<String> = (0..MAX_LEVEL).filter(|&k| r.pair_long_opens[k] > 0)
                        .map(|k| format!("L{k}:lp={:+.0}/op={}", r.pair_long_pnl[k], r.pair_long_opens[k])).collect();
                    eprintln!("        [{sym}/{vname}] short_per_lvl:[{}] long_per_lvl:[{}]", sp_lvl.join(" "), lp_lvl.join(" "));
                    // 否定性优先（§8.3）：FACE_B 的 liq=0 是判据但 L2 否定性如实报告，不硬断言掩盖其余读数。
                    if *vname == "FACE_B" && r.n_liquidations > 0 {
                        eprintln!("        [{sym}/FACE_B] ⚠否定性 L2：liq={}>0（否定线未封死全部穿仓，§2.4 定仓精化触发）", r.n_liquidations);
                    }
                }
            }
        }
        assert!(any, "至少跑出一个标的的 FACE_B 变体（默认 BTC）");
    }

    /// **Face B L2 死终止判据：2022 吃到跌幅（编排者「2022 吃到跌幅=成功」）**。
    ///
    /// BTC 全史（强牛）是次级别空腿的**最差窗口**（574 确认滞后税：浅回调涨回才平，short_pnl 微负=真539）。
    /// Face B「吃中间级别回调跌」的成功窗口 = 真 bear 子窗（中间级别独立空腿吃下跌）。本测试用真 bear 2022
    /// 窗口（ES 标普熊 −27% / BRN −40% / CL）验证 FACE_B 次级别独立空腿是否吃到跌幅（short_pnl 转正/改善）
    /// vs RB_PAIR（geom_tower 压制基线）。**判据**：FACE_B short_pnl > RB_PAIR（解压制吃跌）∧ liq=0。
    /// 注：本测试**不开 enable_pair_core_short**（核心翻空=Face A #113）——纯 Face B 次级别独立空腿吃回调。
    ///
    /// 跑法：`cargo test --release recursive_t::rec_stream::tests::faceb_l2_bear_window -- --exact --ignored --nocapture`
    #[test]
    #[ignore = "Face B L2 bear 窗（做空腿吃跌幅 #110），需 analysis/data_cache/*_databento_10y.json"]
    fn faceb_l2_bear_window() {
        use super::super::rec_engine::EngineConfig;
        use crate::recursive_t::backtest_run::load_clean_ohlc_window;
        use std::path::PathBuf;

        let data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("analysis/data_cache");
        let bear_windows: [(&str, &str, &str, &str); 3] = [
            ("ES_2022标普熊",  "es_1m_databento_10y.json",  "2022-01-03", "2022-10-13"), // 4800→3500 −27%
            ("BRN_2022H2跌",   "brn_1m_databento_10y.json", "2022-06-08", "2022-12-09"), // $125→$76 −40%
            ("CL_2014-16油崩", "cl_1m_databento_10y.json",  "2014-06-20", "2016-02-11"), // $107→$26 −75%
        ];

        eprintln!("\n===== Face B L2 死终止判据：2022 吃到跌幅（RB_PAIR 压制 vs FACE_B 均匀+真否定线）=====");
        let variants = [("RB_PAIR", EngineConfig::reading_b_pair()), ("FACE_B", EngineConfig::face_b())];
        let mut any = false;
        for (label, file, d0, d1) in bear_windows {
            let path = data_dir.join(file);
            if !path.exists() { eprintln!("[{label}] 数据缺失，跳过"); continue; }
            let (o, h, l, c) = load_clean_ohlc_window(&path, d0, d1);
            let n = c.len();
            if n == 0 { eprintln!("[{label}] 窗口空，跳过"); continue; }
            let bh = if c[0] > 0.0 { (c[n-1]/c[0]-1.0)*100.0 } else { 0.0 };
            for (vname, cfg) in variants.iter() {
                let mut s = RecStream::new_with_config(PerfectionMode::Structural, A0Source::Segment, *cfg);
                for i in 0..n { s.push_bar(o[i], h[i], l[i], c[i]); }
                let fin = s.finish();
                assert!(fin.is_finite() && fin > 0.0, "[{label}/{vname}] final_nav 须有限正");
                let strat = (fin/INITIAL_CAPITAL - 1.0)*100.0;
                let r = s.driver().root();
                if *vname == "FACE_B" { any = true; }
                let short_pnl: f64 = (0..MAX_LEVEL).map(|k| r.pair_short_pnl[k]).sum();
                let long_pnl: f64 = (0..MAX_LEVEL).map(|k| r.pair_short_pnl[k] + r.pair_long_pnl[k]).sum::<f64>() - short_pnl;
                let sopens: u64 = (0..MAX_LEVEL).map(|k| r.pair_short_opens[k]).sum();
                eprintln!(
                    "[{label:<14}/{vname:<7}] strat={strat:+.1}% bh={bh:+.1}% | short_pnl={short_pnl:+.0} long_pnl={long_pnl:+.0} \
                     s_op={sopens} | l_stop={} s_stop={} liq={} max_gross={:.2}×",
                    r.pair_long_stops, r.pair_short_stops, r.n_liquidations, r.max_gross_exp_x100 as f64 / 100.0
                );
                let sp_lvl: Vec<String> = (0..MAX_LEVEL).filter(|&k| r.pair_short_opens[k] > 0)
                    .map(|k| format!("L{k}:sp={:+.0}/op={}", r.pair_short_pnl[k], r.pair_short_opens[k])).collect();
                eprintln!("        [{label}/{vname}] short_per_lvl:[{}]", sp_lvl.join(" "));
            }
        }
        assert!(any, "至少跑出一个 bear 窗的 FACE_B 变体");
    }

    /// **Face A（做空腿赚 #113 implA）L2 验证：核心能动 = cc 走势完成翻空 + 默认开启**。
    ///
    /// 存在论位置（shortleg-profit-spec §5.2/§七/§八.2）：Face A = Face B 基座 + 核心翻转吃熊
    /// （`enable_pair_core_short`：核心多腿在 cc 走势完成 d_top 翻空镜像，区间套级联减滞后）。两 regime 由
    /// 「哪级别走势完成」自动整合（零 if regime）。**判据（§8.2，看符号 + 自动 regime）**：
    /// ① BTC 强牛（最差窗）：核心翻转闸门**不灾难误开**（churn 稀疏，short_pnl 非 −10万量级灾难，liq=0）=
    ///    §5.2.4 自动 regime（net-up cc 走势未完成 ⇒ 不翻空，无假顶翻空打主升浪 leverage-accept −106256）；
    /// ② 真 bear 窗：核心翻空吃熊（pair_core_churns>0 ∧ core short 大额 ∧ short_pnl 改善 vs FACE_B），liq=0。
    /// 增量 = FACE_A − FACE_B（唯一差 = 核心翻转，§集成契约）。
    ///
    /// L2 等级（formalization-validity-domain）：单标的（BTC + 3 bear 窗）假设检验，可否证。否定性优先
    /// （§8.3）：BTC 强牛 short_pnl 微负 = 574 确认滞后税（非 bug）；若核心翻转在 net-up 灾难误开（churn 暴增
    /// + short_pnl −10万量级）⟹ 如实报告（闸门失效，§5.2.4 翻转），不掩盖。做空腿全 regime 赚=L3（C #117）。
    ///
    /// 跑法：`BT_SYMBOLS=BTC cargo test --release recursive_t::rec_stream::tests::facea_l2_validate -- --exact --ignored --nocapture`
    #[test]
    #[ignore = "Face A L2（做空腿赚 #113），需 analysis/data_cache/*.json"]
    fn facea_l2_validate() {
        use super::super::rec_engine::EngineConfig;
        use crate::recursive_t::backtest_run::{load_clean_ohlc, load_clean_ohlc_window, SYMBOLS};
        use std::path::PathBuf;

        let data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("analysis/data_cache");
        let only: Vec<String> = std::env::var("BT_SYMBOLS").ok()
            .map(|s| s.split(',').map(|x| x.trim().to_uppercase()).filter(|x| !x.is_empty()).collect())
            .unwrap_or_else(|| vec!["BTC".to_string()]);

        // 逐变体跑一遍并打印 LegPair 读数（OFF=instances 基线参照）。
        let run = |label: &str, vname: &str, cfg: EngineConfig, o: &[f64], h: &[f64], l: &[f64], c: &[f64], bh: f64| {
            let n = c.len();
            let mut s = RecStream::new_with_config(PerfectionMode::Structural, A0Source::Segment, cfg);
            for i in 0..n { s.push_bar(o[i], h[i], l[i], c[i]); }
            let fin = s.finish();
            assert!(fin.is_finite() && fin > 0.0, "[{label}/{vname}] final_nav 须有限正，得 {fin}");
            let strat = (fin / INITIAL_CAPITAL - 1.0) * 100.0;
            let r = s.driver().root();
            let short_pnl: f64 = (0..MAX_LEVEL).map(|k| r.pair_short_pnl[k]).sum();
            let long_pnl: f64 = (0..MAX_LEVEL).map(|k| r.pair_long_pnl[k]).sum();
            let sopens: u64 = (0..MAX_LEVEL).map(|k| r.pair_short_opens[k]).sum();
            eprintln!(
                "[{label:<14}/{vname:<7}] strat={strat:+.1}% bh={bh:+.1}% | short_pnl={short_pnl:+.0} long_pnl={long_pnl:+.0} \
                 s_op={sopens} core_churn={} | l_stop={} s_stop={} liq={} max_gross={:.2}×",
                r.pair_core_churns, r.pair_long_stops, r.pair_short_stops, r.n_liquidations,
                r.max_gross_exp_x100 as f64 / 100.0
            );
            (strat, short_pnl, r.pair_core_churns, r.n_liquidations)
        };

        // ── ① BTC 全史（强牛，核心翻转闸门最差窗——验证自动 regime 不灾难误开）──
        eprintln!("\n===== Face A（#113 implA）L2 ①：BTC 强牛自动 regime（OFF / FACE_B / FACE_A 核心翻转增量）=====");
        let mut any = false;
        for (sym, file) in SYMBOLS {
            if !only.is_empty() && !only.contains(&sym.to_uppercase()) { continue; }
            let path = data_dir.join(file);
            if !path.exists() { eprintln!("[{sym}] 数据缺失，跳过"); continue; }
            let (o, h, l, c) = load_clean_ohlc(&path);
            let n = c.len();
            let bh = if n > 0 && c[0] > 0.0 { (c[n-1]/c[0]-1.0)*100.0 } else { 0.0 };
            run(sym, "OFF", EngineConfig::off(), &o, &h, &l, &c, bh);
            run(sym, "FACE_B", EngineConfig::face_b(), &o, &h, &l, &c, bh);
            let (_, sp_a, churn_a, liq_a) = run(sym, "FACE_A", EngineConfig::face_a(), &o, &h, &l, &c, bh);
            any = true;
            // §5.2.4 自动 regime：net-up 核心翻转闸门不灾难（短pnl 非 −10万量级 + liq=0）。否定性如实报告。
            if sp_a < -50_000.0 || liq_a > 0 {
                eprintln!("        [{sym}/FACE_A] ⚠否定性 L2：核心翻转在 net-up 可能误开（short_pnl={sp_a:+.0} churn={churn_a} liq={liq_a}，§5.2.4 闸门审查）");
            }
        }
        assert!(any, "至少跑出一个标的的 FACE_A 变体（默认 BTC）");

        // ── ② 真 bear 窗（核心翻空吃熊——验证 churn>0 + short_pnl 改善 vs FACE_B）──
        eprintln!("\n===== Face A（#113 implA）L2 ②：真 bear 核心翻空吃熊（FACE_B vs FACE_A）=====");
        let bear_windows: [(&str, &str, &str, &str); 3] = [
            ("ES_2022标普熊",  "es_1m_databento_10y.json",  "2022-01-03", "2022-10-13"),
            ("BRN_2022H2跌",   "brn_1m_databento_10y.json", "2022-06-08", "2022-12-09"),
            ("CL_2014-16油崩", "cl_1m_databento_10y.json",  "2014-06-20", "2016-02-11"),
        ];
        for (label, file, d0, d1) in bear_windows {
            let path = data_dir.join(file);
            if !path.exists() { eprintln!("[{label}] 数据缺失，跳过"); continue; }
            let (o, h, l, c) = load_clean_ohlc_window(&path, d0, d1);
            let n = c.len();
            if n == 0 { eprintln!("[{label}] 窗口空，跳过"); continue; }
            let bh = if c[0] > 0.0 { (c[n-1]/c[0]-1.0)*100.0 } else { 0.0 };
            run(label, "FACE_B", EngineConfig::face_b(), &o, &h, &l, &c, bh);
            run(label, "FACE_A", EngineConfig::face_a(), &o, &h, &l, &c, bh);
        }
    }

    /// **任务 bear-validate：大额吃熊牛熊对称 L3 验证**（编排者：入库 bear 标的验证核心做空镜像）。
    ///
    /// 存在论位置：#69 判决「大额做空（max_gross>1×）吃熊有效域 ⊂ 真 bear regime，8 net-up 标的
    /// 无法证实⇒结构性禁止」（shortleg-alpha §五，231号 formalization-validity-domain）。本测试**不入库
    /// 新标的**——bear regime 已作为现有 8 标的的子窗口存在（CL 2014-2016 油崩 −75% / CL 2020 COVID 崩 /
    /// ES 2022 标普熊 −27% / BRN 2020 COVID 崩 −76%），用 `load_clean_ohlc_window`（dates 列切片）取真 bear
    /// 窗口，同格式同管线，**比入库新标的更严格**（零格式失配风险，L2 形式化有效域：定义域=切片合法的全部
    /// 日期窗，有效域=真 bear 子窗）。
    ///
    /// 核心测试：放开 #69 `k<核心` 禁令（`enable_pair_core_short`，最高活跃级别走势完成→核心翻空镜像），
    /// 对照 committed #69（RB_PAIR，k<核心 门控）。验证 bear regime 下核心做空是否**大额吃熊赚**
    /// （max_gross>1× ∧ short_pnl 大额正，对照 net-up 标的的灾难 CL L4 −40928）。
    ///
    /// 跑法：`cargo test --release recursive_t::rec_stream::tests::bear_validate_core_short_l3 -- --exact --ignored --nocapture`
    #[test]
    #[ignore = "任务 bear-validate 大额吃熊 L3，需 analysis/data_cache/*.json（dates 列窗切片）"]
    fn bear_validate_core_short_l3() {
        use super::super::rec_engine::EngineConfig;
        use crate::recursive_t::backtest_run::load_clean_ohlc_window;
        use std::path::PathBuf;

        let data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("analysis/data_cache");

        // bear 窗口（label, 文件, day_start, day_end）：真 bear regime 子窗（dates 列切片，闭区间）。
        // CL/ES/BRN 文件均 parallel-array + dates（已核验）。BTC1m 无 dates 列⇒无法日期切片（fail-loud 声明，
        // 不入此表；1s 床位=观测分辨率非操作床位，[[project_cl_1s_a0_verdict]]，不混入）。
        let bear_windows: [(&str, &str, &str, &str); 5] = [
            ("CL_2014-16油崩",  "cl_1m_databento_10y.json",  "2014-06-20", "2016-02-11"), // $107→$26 −75%
            ("CL_2020COVID崩",  "cl_1m_databento_10y.json",  "2020-01-06", "2020-04-30"), // $63→$16 急崩
            ("ES_2022标普熊",   "es_1m_databento_10y.json",  "2022-01-03", "2022-10-13"), // 4800→3500 −27%
            ("BRN_2020COVID崩", "brn_1m_databento_10y.json", "2020-01-06", "2020-04-30"), // $68→$16 −76%
            ("BRN_2022H2跌",    "brn_1m_databento_10y.json", "2022-06-08", "2022-12-09"), // $125→$76 −40%
        ];

        eprintln!("\n===== 任务 bear-validate：大额吃熊核心翻空 L3（OFF / RB_PAIR(#69 k<核心) / RB_PAIR_CS(放开核心翻空)）=====");
        eprintln!("（认识论 L3：真 bear regime 数据，可否证。核心判据：core_short 是否 max_gross>1× ∧ short_pnl 大额正）");
        // OFF=基线（base operate 路径，无 leg）/ RB_PAIR=committed #69（k<核心 门控）/
        // RB_PAIR_CS=核心走势完成翻空镜像（d_top）/ RB_PAIR_T3=放开 k<核心 t3sell 大额做空（变体1 机制，max_gross>1×）。
        let variants = [
            ("OFF",         EngineConfig::off()),
            ("RB_PAIR",     EngineConfig::reading_b_pair()),
            ("RB_PAIR_CS",  EngineConfig::reading_b_pair_coreshort()),
            ("RB_PAIR_T3",  EngineConfig::reading_b_pair_coreshort_t3()),
        ];
        let mut any = false;
        for (label, file, d0, d1) in bear_windows {
            let path = data_dir.join(file);
            if !path.exists() { eprintln!("[{label}] 数据缺失 {file}，跳过"); continue; }
            let (o, h, l, c) = load_clean_ohlc_window(&path, d0, d1);
            let n = c.len();
            let bh = if n > 0 && c[0] > 0.0 { (c[n-1]/c[0]-1.0)*100.0 } else { 0.0 };
            eprintln!("\n── {label}  [{d0}..{d1}]  bars={n}  BH={bh:+.1}%  close[{:.2}→{:.2}] ──", c[0], c[n-1]);
            for (vname, cfg) in variants.iter() {
                let t0 = std::time::Instant::now();
                let mut s = RecStream::new_with_config(PerfectionMode::Structural, A0Source::Segment, *cfg);
                for i in 0..n { s.push_bar(o[i], h[i], l[i], c[i]); }
                let fin = s.finish();
                assert!(fin.is_finite(), "[{label}/{vname}] final_nav 非有限");
                let strat = (fin/INITIAL_CAPITAL - 1.0)*100.0;
                let r = s.driver().root();
                if *vname == "OFF" {
                    eprintln!("  [{vname:<11}] strat={strat:+8.1}% (BH {bh:+.1}%) ({:.1}s)", t0.elapsed().as_secs_f64());
                    continue;
                }
                any = true;
                let long_pnl: f64 = (0..MAX_LEVEL).map(|k| r.pair_long_pnl[k]).sum();
                let short_pnl: f64 = (0..MAX_LEVEL).map(|k| r.pair_short_pnl[k]).sum();
                let lopens: u64 = (0..MAX_LEVEL).map(|k| r.pair_long_opens[k]).sum();
                let sopens: u64 = (0..MAX_LEVEL).map(|k| r.pair_short_opens[k]).sum();
                eprintln!(
                    "  [{vname:<11}] strat={strat:+8.1}% (BH {bh:+.1}%) | Lopen={lopens} Sopen={sopens} \
                     long_pnl={long_pnl:+.0} short_pnl={short_pnl:+.0} | Lstop={} Sstop={} churn={} liq={} \
                     | max_gross={:.2}× max_net={:.2}× ({:.1}s)",
                    r.pair_long_stops, r.pair_short_stops, r.pair_core_churns, r.n_liquidations,
                    r.max_gross_exp_x100 as f64 / 100.0, r.max_net_exp_x100 as f64 / 100.0, t0.elapsed().as_secs_f64()
                );
                // per-level 空腿 pnl/opens（判定大额吃熊在哪些级别赚/亏：核心级别=深层小 k=大配额）。
                let sp_lvl: Vec<String> = (0..MAX_LEVEL)
                    .filter(|&k| r.pair_short_opens[k] > 0)
                    .map(|k| format!("L{k}:pnl={:+.0}/op={}", r.pair_short_pnl[k], r.pair_short_opens[k]))
                    .collect();
                eprintln!("        short_per_level: {}", sp_lvl.join(" "));
                // 守恒：final_nav 有限正（所有变体）。零强平硬断言仅施于 committed #69（RB_PAIR 交付契约）；
                // 实验变体（CS/T3）liq>0 是**发现**（大额核心做空穿仓）非契约违反 ⇒ 报告不断言。
                assert!(fin.is_finite() && fin > 0.0, "[{label}/{vname}] final_nav 须有限正，得 {fin}");
                if *vname == "RB_PAIR" {
                    assert_eq!(r.n_liquidations, 0, "[{label}/RB_PAIR] 零强平违反（#69 契约）：liq={}", r.n_liquidations);
                }
            }
        }
        assert!(any, "至少跑出一个 bear 窗口的 RB_PAIR 变体");
    }

    /// **BTC 牛熊段收益归因**（编排者 2026-06-21）：zigzag(40%反转)分牛熊段，逐段算引擎 TW 收益（MtM）
    /// vs BH，段内短差 pnl + 强平落点（牛/熊段）。Structural 模式。诊断纯观测（不改引擎/不影响 bit-exact）。
    /// 跑法：`cargo test --release recursive_t::rec_stream::tests::btc_bull_bear_segments -- --exact --ignored --nocapture`
    #[test]
    #[ignore = "BTC 牛熊段归因诊断，需 btc_1m_full.json"]
    fn btc_bull_bear_segments() {
        use crate::recursive_t::backtest_run::load_clean_ohlc;
        use std::path::PathBuf;
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("analysis/data_cache/btc_1m_full.json");
        if !path.exists() {
            eprintln!("数据缺失 {path:?}");
            return;
        }
        let (o, h, l, c) = load_clean_ohlc(&path);
        let n = c.len();

        // ── zigzag 分段（反转阈值 env ZZ，默认 0.40，BTC 巨幅摆动）→ 交替 峰/谷 pivot ──
        let rev: f64 = std::env::var("ZZ").ok().and_then(|s| s.parse().ok()).unwrap_or(0.40);
        let mut pivots: Vec<(usize, f64, bool)> = Vec::new(); // (bar, price, is_peak)
        let (mut ext_bar, mut ext_price) = (0usize, c[0]);
        let mut up = true; // BTC 2015 低位起步 → 先找峰
        for i in 1..n {
            if up {
                if c[i] > ext_price {
                    ext_price = c[i];
                    ext_bar = i;
                } else if c[i] < ext_price * (1.0 - rev) {
                    pivots.push((ext_bar, ext_price, true));
                    up = false;
                    ext_price = c[i];
                    ext_bar = i;
                }
            } else if c[i] < ext_price {
                ext_price = c[i];
                ext_bar = i;
            } else if c[i] > ext_price * (1.0 + rev) {
                pivots.push((ext_bar, ext_price, false));
                up = true;
                ext_price = c[i];
                ext_bar = i;
            }
        }
        // 段边界 = [0, pivot_bars..., n-1]
        let mut bounds: Vec<usize> = vec![0];
        for &(b, _, _) in &pivots {
            bounds.push(b);
        }
        bounds.push(n - 1);
        bounds.dedup();

        // ── 跑 RecStream Structural，逐 bar 累计核心方向，边界记录快照（核心 vs 次级别分离）──
        use crate::trading::types::Polarity;
        struct Snap {
            bar: usize,
            tw: f64,
            spl: [f64; 6], // short_pnl_by_level 累计（次级别短差腿 per-level）
            cl: u64,       // 累计核心 Long bar（highest_active=Long）
            cs: u64,       // 累计核心 Short bar（主力做空 = 无 parent flip 到 Short）
            lv: f64,       // 边界处 多头市值 = long_units * c（核心+次级别多）
            free: f64,     // 自由现金
            wd: f64,       // withdrawn（退本金，锁定不在险）
        }
        let mut s = RecStream::new(PerfectionMode::Structural);
        let mut snaps: Vec<Snap> = Vec::new();
        let (mut cl, mut cs) = (0u64, 0u64);
        let mut bi = 0usize;
        for i in 0..n {
            s.push_bar(o[i], h[i], l[i], c[i]);
            // 逐 bar 核心方向（highest_active 的 direction）：区分核心 Long/Short（主力方向）。
            {
                let root = s.driver().root();
                if let Some(k) = root.highest_active() {
                    match root.instance(k).direction {
                        Polarity::Long => cl += 1,
                        Polarity::Short => cs += 1,
                    }
                }
            }
            while bi < bounds.len() && bounds[bi] == i {
                let root = s.driver().root();
                let mut spl = [0.0; 6];
                for (j, v) in spl.iter_mut().enumerate() {
                    *v = root.short_pnl_by_level[j];
                }
                let (lu, _) = root.exposure();
                snaps.push(Snap {
                    bar: i,
                    tw: root.total_wealth(c[i]),
                    spl,
                    cl,
                    cs,
                    lv: lu * c[i],
                    free: root.free(),
                    wd: root.withdrawn_total(),
                });
                bi += 1;
            }
        }
        let _ = s.finish();
        let liq = s.driver().root().liq_log.clone();
        let flips = s.driver().root().flip_log.clone(); // (bar, from, to, j_level, is_buy)

        // ── 表1：核心方向（主力）vs 次级别短差总和 ──
        eprintln!(
            "\n===== BTC 牛熊段 核心(主力)vs次级别(短差) 归因（Structural, zigzag {:.0}%）=====",
            rev * 100.0
        );
        eprintln!(
            "{:>4} {:>8} {:>8} {:>9} {:>9} {:>8} {:>8} {:>7} {:>11} {:>5}",
            "段", "起价", "止价", "引擎TW%", "BH%", "核心多%", "核心空%", "flip→S", "次级短差Σ", "强平"
        );
        for w in snaps.windows(2) {
            let (a, b) = (&w[0], &w[1]);
            let is_bull = c[b.bar] >= c[a.bar];
            let eng = (b.tw / a.tw - 1.0) * 100.0;
            let bh = (c[b.bar] / c[a.bar] - 1.0) * 100.0;
            let segbars = ((b.cl + b.cs) - (a.cl + a.cs)).max(1);
            let clp = 100.0 * (b.cl - a.cl) as f64 / segbars as f64;
            let csp = 100.0 * (b.cs - a.cs) as f64 / segbars as f64;
            let flip_s = flips
                .iter()
                .filter(|(fb, _, t, _, _)| {
                    *fb > a.bar as i64 && *fb <= b.bar as i64 && *t == Polarity::Short
                })
                .count();
            let seg_short: f64 = (0..6).map(|j| b.spl[j] - a.spl[j]).sum();
            let nliq =
                liq.iter().filter(|(_, _, _, lb, _, _)| *lb > a.bar as i64 && *lb <= b.bar as i64).count();
            eprintln!(
                "{:>4} {:>8.0} {:>8.0} {:>+8.1}% {:>+8.1}% {:>7.1}% {:>7.1}% {:>7} {:>+11.0} {:>5}",
                if is_bull { "牛" } else { "熊" },
                c[a.bar],
                c[b.bar],
                eng,
                bh,
                clp,
                csp,
                flip_s,
                seg_short,
                nliq
            );
        }

        // ── 表2：各段 次级别短差 per-level P&L（sink/recover 空头腿 L0..L5）──
        eprintln!("\n--- 各段 次级别短差 per-level（L0..L5，正=该级别短差段内赚）---");
        eprintln!(
            "{:>4} {:>8} {:>8} {:>9} {:>9} {:>9} {:>9} {:>9} {:>9}",
            "段", "起价", "止价", "L0", "L1", "L2", "L3", "L4", "L5"
        );
        for w in snaps.windows(2) {
            let (a, b) = (&w[0], &w[1]);
            let is_bull = c[b.bar] >= c[a.bar];
            let d: Vec<i64> = (0..6).map(|j| (b.spl[j] - a.spl[j]) as i64).collect();
            eprintln!(
                "{:>4} {:>8.0} {:>8.0} {:>9} {:>9} {:>9} {:>9} {:>9} {:>9}",
                if is_bull { "牛" } else { "熊" },
                c[a.bar],
                c[b.bar],
                d[0],
                d[1],
                d[2],
                d[3],
                d[4],
                d[5]
            );
        }

        // ── 表3：各段 资本结构（段首→段尾 暴露率，揭示熊段靠减仓避损还是主力做空）──
        eprintln!("\n--- 各段 资本结构（多头市值/TW · 现金/TW · withdrawn退本金/TW，段首→段尾）---");
        eprintln!(
            "{:>4} {:>8} {:>8} {:>17} {:>17} {:>17}",
            "段", "起价", "止价", "多头暴露%首→尾", "现金%首→尾", "退本金%首→尾"
        );
        for w in snaps.windows(2) {
            let (a, b) = (&w[0], &w[1]);
            let is_bull = c[b.bar] >= c[a.bar];
            let pct = |v: f64, tw: f64| if tw.abs() > 1.0 { 100.0 * v / tw } else { 0.0 };
            eprintln!(
                "{:>4} {:>8.0} {:>8.0}  {:>6.1}→{:>6.1}  {:>6.1}→{:>6.1}  {:>6.1}→{:>6.1}",
                if is_bull { "牛" } else { "熊" },
                c[a.bar],
                c[b.bar],
                pct(a.lv, a.tw),
                pct(b.lv, b.tw),
                pct(a.free, a.tw),
                pct(b.free, b.tw),
                pct(a.wd, a.tw),
                pct(b.wd, b.tw)
            );
        }

        let bh_total = (c[n - 1] / c[0] - 1.0) * 100.0;
        let eng_total = (snaps.last().unwrap().tw / snaps.first().unwrap().tw - 1.0) * 100.0;
        let flip_s_total = flips.iter().filter(|(_, _, t, _, _)| *t == Polarity::Short).count();
        eprintln!(
            "\n全程 BH={:+.1}% 引擎TW={:+.1}% | 核心flip总数={}(→Short={}) 触发级别={:?} | 强平={}",
            bh_total,
            eng_total,
            flips.len(),
            flip_s_total,
            flips.iter().filter(|(_, _, t, _, _)| *t == Polarity::Short).map(|(b, _, _, j, _)| (*b, *j)).collect::<Vec<_>>(),
            liq.len()
        );
        assert!(!snaps.is_empty());
    }

    /// **prove 守卫尺度不变性 L3 交叉验证 + L4 触发探测**（编排者 2026-06-22）。
    ///
    /// ## 任务（观测分辨率维度，非操作床位）
    /// 缠论级别递归 ≅ 多尺度滤波器组（小波 MRA）。prove 守卫 = 滤波器组不变量的可执行形式。
    /// 方向1 已证 **L2**（CL 577k + BTC 1.2M 各 1 窗零 fire + 滤波器深度 +1）。本测试扩展三维度：
    /// 1. **多标的（宽度）**：CL/BTC 加 **ES/BRN**——4 panic 守卫 + radial_scaling 每标的 1s **零 fire**。
    /// 2. **多窗口（同标的稳定性）**：同 1s 文件切 ≥2 个 disjoint 日期窗——守卫跨窗口稳定 =
    ///    regime 无关的结构断言（不是某段行情的偶然性质）。
    /// 3. **L4 触发探测（核心突破）**：用更长窗口/更多行情反复的 1s（_1y 文件），看 `highest_active`
    ///    是否涌现 **L4**。若 L4 涌现 → 验证 4 panic 守卫在 L4 仍零 fire（突破方向1 的 L3 有效域上界）。
    ///    层数取决于行情反复次数非 bar 数（[[project_recursive_level_emergence]]）。
    ///
    /// ## 4 panic 守卫尺度不变性
    /// sink_descends / sigma_quota / relabel_invariant / bsp_triggers_operation 已接入 rec_engine
    /// 操作热路径，违反即 panic 终止。进程跑完零 panic = 这四个守卫在该尺度/窗口/层成立。
    /// 若某守卫在 1s 深层 fire = 滤波器自相似的有效域边界（formalization-validity-domain，
    /// **否定性结果比确认性结果更有价值**）——如实报告是哪个守卫/哪标的/哪窗口/哪层。
    ///
    /// ## 认识论等级
    /// 多标的×多窗口零 fire = **L3**（标的维度 × 窗口维度交叉）。L4 涌现且守卫成立 = 有效域上界突破。
    /// **不评估 1s 交易收益/alpha**——操作床位维度已否证（[[project_cl_1s_a0_verdict]]：0.58bps<taker 1.45bps）。
    ///
    /// ## 跑法（worktree：CHANLUN_DATA_DIR 指主仓库 data_cache 绝对路径）
    /// 内存：大数据放最后，先中小。逐标的跑（避免一次性加载多个 GB 级文件）：
    /// ```text
    /// BT_1S_SYMBOLS=CL  cargo test --release scale_invariance_1s -- --ignored --nocapture  # 中小，含多窗
    /// BT_1S_SYMBOLS=BTC cargo test --release scale_invariance_1s -- --ignored --nocapture  # 1.2M
    /// BT_1S_SYMBOLS=BRN cargo test --release scale_invariance_1s -- --ignored --nocapture  # 262MB，L4 探测
    /// BT_1S_SYMBOLS=ES  cargo test --release scale_invariance_1s -- --ignored --nocapture  # 657MB，最后跑
    /// ```
    /// L4 探测窗口大小由 `L4_DAYS`（默认 90 天，0=整文件全量）控制——长窗口更可能触发 L4，
    /// 但 _1y 全量是 GB 级 + 数百万 bar，按需放大。
    #[test]
    #[ignore = "1s prove 守卫尺度不变性 L3 交叉验证，需 analysis/data_cache 1s 文件 + CHANLUN_DATA_DIR"]
    fn scale_invariance_1s() {
        use crate::recursive_t::backtest_run::{
            load_clean_ohlc, load_clean_ohlc_window, load_clean_ohlc_window_ns,
        };
        use std::path::PathBuf;
        // 数据目录：默认 `<repo>/analysis/data_cache`；worktree 隔离运行时大 JSON 被 gitignore
        // 不在 worktree 内，用 CHANLUN_DATA_DIR 指向主仓库 data_cache（绝对路径）。
        let data_dir = match std::env::var("CHANLUN_DATA_DIR") {
            Ok(d) => PathBuf::from(d),
            Err(_) => PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("analysis/data_cache"),
        };
        let which = std::env::var("BT_1S_SYMBOLS").unwrap_or_else(|_| "CL".to_string()).to_uppercase();

        // ── 单个验证运行（= 1 标的 × 1 尺度 × 1 窗口）──
        // 切窗方式三态：FullFile（整文件，1s 小数据）/ WinNs（timestamps_ns 切窗，databento）/
        // WinDates（dates 字符串切窗，btc_1m_full / 旧 1min 对照）。1min 对照仍用同窗对比深度。
        enum Slice {
            FullFile,
            WinNs(&'static str, &'static str), // [day_start, day_end] 闭区间，基于 timestamps_ns
            WinDates(&'static str, &'static str), // 基于 dates 列
        }
        struct Run {
            label: String,
            path_1s: PathBuf,
            slice: Slice,
            /// 1min 同窗对照（滤波器深度 Δ 对照）：(文件, day_start, day_end, dates?)。
            cmp_1min: Option<(PathBuf, &'static str, &'static str, bool)>,
        }

        // L4 探测窗口开关：默认用各标的静态 ~3 个月窗（更多行情反复 → 更可能 L4）；
        // L4_DAYS=0 → 整文件全量（_1y 是 GB 级 + 数百万 bar，按需）。静态端点 = 可审计。
        let l4_full: bool = std::env::var("L4_DAYS").ok().and_then(|s| s.parse::<i64>().ok()) == Some(0);

        let runs: Vec<Run> = match which.as_str() {
            // ── CL：1mo 文件（577k/2025-04 整月）vs 1min 同窗 + 同文件切 2 子窗（多窗稳定性）──
            "CL" => vec![
                Run { label: "CL 1s 1mo 全月".into(),
                    path_1s: data_dir.join("cl_1s_databento_1mo.json"), slice: Slice::FullFile,
                    cmp_1min: Some((data_dir.join("cl_1m_databento_10y.json"), "2025-04-01", "2025-04-30", true)) },
                Run { label: "CL 1s 窗A(04-01..04-10)".into(),
                    path_1s: data_dir.join("cl_1s_databento_1mo.json"),
                    slice: Slice::WinNs("2025-04-01", "2025-04-10"), cmp_1min: None },
                Run { label: "CL 1s 窗B(04-21..04-30)".into(),
                    path_1s: data_dir.join("cl_1s_databento_1mo.json"),
                    slice: Slice::WinNs("2025-04-21", "2025-04-30"), cmp_1min: None },
                // L4 探测：CL 1y（5.96M，2024-06-02..2025-05-30）。默认 ~3 月窗，L4_DAYS=0 全量。
                Run { label: format!("CL 1s 1y L4探测({})", if l4_full {"全量5.96M".into()} else {"窗2024-06-02..08-30".to_string()}),
                    path_1s: data_dir.join("cl_1s_databento_1y.json"),
                    slice: if l4_full { Slice::FullFile } else { Slice::WinNs("2024-06-02", "2024-08-30") },
                    cmp_1min: None },
            ],
            // ── BTC：2 周文件（1.2M）vs 1min 同窗 + 同文件切 2 子窗 ──
            "BTC" => vec![
                Run { label: "BTC 1s 2w 全".into(),
                    path_1s: data_dir.join("btc_1s_2week.json"), slice: Slice::FullFile,
                    cmp_1min: Some((data_dir.join("btc_1m_full.json"), "2026-05-29", "2026-06-11", true)) },
                Run { label: "BTC 1s 窗A(05-29..06-04)".into(),
                    path_1s: data_dir.join("btc_1s_2week.json"),
                    slice: Slice::WinDates("2026-05-29", "2026-06-04"), cmp_1min: None },
                Run { label: "BTC 1s 窗B(06-05..06-11)".into(),
                    path_1s: data_dir.join("btc_1s_2week.json"),
                    slice: Slice::WinDates("2026-06-05", "2026-06-11"), cmp_1min: None },
            ],
            // ── ES：1y（11.76M/657MB，最大）。多窗 + L4 探测。整文件全量需 GB 级内存——放最后跑。──
            "ES" => vec![
                Run { label: "ES 1s 窗A(2025-06-12..06-30)".into(),
                    path_1s: data_dir.join("es_1s_databento_1y.json"),
                    slice: Slice::WinNs("2025-06-12", "2025-06-30"), cmp_1min: None },
                Run { label: "ES 1s 窗B(2026-05-12..05-30)".into(),
                    path_1s: data_dir.join("es_1s_databento_1y.json"),
                    slice: Slice::WinNs("2026-05-12", "2026-05-30"), cmp_1min: None },
                Run { label: format!("ES 1s 1y L4探测({})", if l4_full {"全量11.76M".into()} else {"窗2025-06-12..09-10".to_string()}),
                    path_1s: data_dir.join("es_1s_databento_1y.json"),
                    slice: if l4_full { Slice::FullFile } else { Slice::WinNs("2025-06-12", "2025-09-10") },
                    cmp_1min: None },
            ],
            // ── BRN：1y（5.3M/262MB）。多窗 + L4 探测。──
            "BRN" => vec![
                Run { label: "BRN 1s 窗A(2024-06-02..06-30)".into(),
                    path_1s: data_dir.join("brn_1s_databento_1y.json"),
                    slice: Slice::WinNs("2024-06-02", "2024-06-30"), cmp_1min: None },
                Run { label: "BRN 1s 窗B(2025-05-01..05-30)".into(),
                    path_1s: data_dir.join("brn_1s_databento_1y.json"),
                    slice: Slice::WinNs("2025-05-01", "2025-05-30"), cmp_1min: None },
                Run { label: format!("BRN 1s 1y L4探测({})", if l4_full {"全量5.3M".into()} else {"窗2024-06-02..08-30".to_string()}),
                    path_1s: data_dir.join("brn_1s_databento_1y.json"),
                    slice: if l4_full { Slice::FullFile } else { Slice::WinNs("2024-06-02", "2024-08-30") },
                    cmp_1min: None },
            ],
            other => panic!("BT_1S_SYMBOLS={other} 未知（支持 CL / BTC / ES / BRN）"),
        };

        // 汇总：每个 run 的 (label, n_bars, max_active_level, radial_viol_sink, radial_viol_bsp,
        //                      n_ops_without_trigger) → L3 矩阵 + L4 判定。
        let mut summary: Vec<(String, usize, usize, u64, u64, u64)> = Vec::new();

        for run in &runs {
            if !run.path_1s.exists() {
                eprintln!("[{}] 1s 数据缺失 {:?}，跳过（no silent cap：明确报告跳了什么）", run.label, run.path_1s);
                continue;
            }
            // ── 加载 1s（按 slice 切窗）──
            let (o, h, l, c) = match &run.slice {
                Slice::FullFile => load_clean_ohlc(&run.path_1s),
                Slice::WinNs(d0, d1) => load_clean_ohlc_window_ns(&run.path_1s, d0, d1),
                Slice::WinDates(d0, d1) => load_clean_ohlc_window(&run.path_1s, d0, d1),
            };
            let n = c.len();
            let t0 = std::time::Instant::now();
            let r_1s = run_one_scale(&o, &h, &l, &c);
            // 释放该 run 的大 OHLC（GB 级文件，下个 run 前回收）。
            drop((o, h, l, c));
            eprintln!(
                "\n========== prove 守卫尺度不变性：{}（Structural, bars={n}, {:.1}s）==========",
                run.label, t0.elapsed().as_secs_f64()
            );
            report_scale(&format!("{} [1s 采样]", run.label), n, &r_1s);
            // L4 判定（核心突破探测）。
            if r_1s.max_active_level >= 4 {
                eprintln!(
                    "  ★★ L4 涌现：最深活跃滤波器层={} ≥ 4 ⇒ 4 panic 守卫在 L4 仍零 fire（进程未 panic）\
                     = 突破方向1 L2 的 L3 有效域上界 ★★",
                    r_1s.max_active_level
                );
            } else {
                eprintln!(
                    "  · 最深活跃层={}（<4）：本窗行情反复不足以涌现 L4（层数~反复次数非 bar 数，546号/recursive_level_emergence）",
                    r_1s.max_active_level
                );
            }
            summary.push((
                run.label.clone(), n, r_1s.max_active_level,
                r_1s.radial_viol_sink, r_1s.radial_viol_bsp, r_1s.n_ops_without_trigger,
            ));

            // ── 1min 同窗对照（滤波器深度 Δ；仅全月/全文件 run 有）──
            if let Some((p_1m, d0, d1, by_dates)) = &run.cmp_1min {
                if !p_1m.exists() {
                    eprintln!("[{}] 1min 对照缺失 {:?}，仅 1s", run.label, p_1m);
                } else {
                    let (o2, h2, l2, c2) = if *by_dates {
                        load_clean_ohlc_window(p_1m, d0, d1)
                    } else {
                        load_clean_ohlc_window_ns(p_1m, d0, d1)
                    };
                    let n2 = c2.len();
                    let r_1m = run_one_scale(&o2, &h2, &l2, &c2);
                    drop((o2, h2, l2, c2));
                    report_scale(&format!("{} [1min 采样 窗={d0}..{d1}]", run.label), n2, &r_1m);
                    eprintln!(
                        "\n--- 滤波器组深度对照（{}）：1s vs 1min ---\n\
                         采样比 1s/1min bar = {:.1}× | 最深活跃层 1s={} 1min={}（Δ={}）",
                        run.label, n as f64 / n2.max(1) as f64,
                        r_1s.max_active_level, r_1m.max_active_level,
                        r_1s.max_active_level as i64 - r_1m.max_active_level as i64,
                    );
                    eprintln!("{:>6} {:>14} {:>14} {:>10}", "level", "1s 通带(走势组)", "1min 通带", "1s 多出");
                    for lv in 0..MAX_LEVEL {
                        let p1s = r_1s.passbands[lv];
                        let p1m = r_1m.passbands[lv];
                        if p1s == 0 && p1m == 0 {
                            continue;
                        }
                        eprintln!("{:>6} {:>14} {:>14} {:>+10}", lv, p1s, p1m, p1s as i64 - p1m as i64);
                    }
                }
            }
        }

        // ── L3 交叉验证汇总矩阵（标的×窗口；零 fire 即 L3 成立）──
        eprintln!("\n========== L3 交叉验证汇总（{which}）：标的×窗口 prove 守卫读数 ==========");
        eprintln!(
            "{:<34} {:>9} {:>9} {:>12} {:>12} {:>12}",
            "run", "bars", "最深层", "radial违(sink)", "radial违(bsp)", "no_trigger"
        );
        let mut any_fire = false;
        let mut l4_seen = false;
        for (label, n, maxlv, rvs, rvb, nt) in &summary {
            if *rvs > 0 || *rvb > 0 || *nt > 0 {
                any_fire = true;
            }
            if *maxlv >= 4 {
                l4_seen = true;
            }
            eprintln!("{label:<34} {n:>9} {maxlv:>9} {rvs:>12} {rvb:>12} {nt:>12}");
        }
        eprintln!(
            "\n判定（{which}）：4 panic 守卫零 fire = 进程跑完未 panic（隐式）；radial_scaling 违反层数 + \
             no_trigger 见上表。\n  observation-count 守卫 fire⟺ 上表 radial违/no_trigger >0：{}；\
             L4 涌现（最深层≥4）：{}。",
            if any_fire { "★有 fire（有效域边界！如实报告）" } else { "无（尺度不变性成立）" },
            if l4_seen { "★是（突破 L3 上界）" } else { "否（本批窗口塔深止于 L3）" },
        );
    }

    /// 单标的单尺度跑通的 prove 守卫读数（[`scale_invariance_1s`] 汇总单元）。
    struct ScaleResult {
        /// 全程 highest_active() 峰值（= 最深活跃滤波器层；滤波器组深度）。
        max_active_level: usize,
        /// 各级别走势组累计数（≈ 滤波器通带：tree_trend_stats[Up+Down+ConsolUp+ConsolDown] 汇总）。
        passbands: [u64; MAX_LEVEL],
        /// sink_by_level（径向标度律 radial_scaling 的 per_level 输入）。
        sink_by_level: [u64; MAX_LEVEL],
        /// recover_by_level（对照）。
        recover_by_level: [u64; MAX_LEVEL],
        /// t1buy/t1sell per-level（BSP 频率随级别分布，radial_scaling 第二输入）。
        bsp_by_level: [u64; MAX_LEVEL],
        /// radial_scaling 局部违反层数（per_level[k]>per_level[k-1]）on sink_by_level。
        radial_viol_sink: u64,
        /// radial_scaling 局部违反层数 on bsp_by_level。
        radial_viol_bsp: u64,
        n_reruns: u64,
        n_sinks: u64,
        n_recovers: u64,
        // ── 观测计数守卫（非 panic；L2 regime 读数）──
        n_dir_mismatch: u64,
        n_dir_checks: u64,
        n_sink_recover_imbalance: u64,
        n_campaigns_checked: u64,
        n_neg_pnl_campaigns: u64,
        ops_bsp: u64,
        ops_emergence: u64,
        ops_eod: u64,
        n_ops_without_trigger: u64,
    }

    /// 喂全序列 → 跑完（4 panic 守卫零 fire 是隐式前提：fire 则此函数 panic 终止）→ 抽读数。
    fn run_one_scale(o: &[f64], h: &[f64], l: &[f64], c: &[f64]) -> ScaleResult {
        let n = c.len();
        let mut s = RecStream::new(PerfectionMode::Structural);
        let mut max_active = 0usize;
        for i in 0..n {
            s.push_bar(o[i], h[i], l[i], c[i]);
            // 全程追踪最深活跃层（滤波器组深度）。
            if let Some(k) = s.driver().root().highest_active() {
                if k > max_active {
                    max_active = k;
                }
            }
        }
        s.finish();
        let r = s.driver().root();
        let g = r.guards();

        let mut passbands = [0u64; MAX_LEVEL];
        for (lv, pb) in passbands.iter_mut().enumerate() {
            // 走势组总数（Up+Down+ConsolUp+ConsolDown）= 该级别滤波器通带数。
            *pb = s.tree_trend_stats[lv].iter().sum();
        }
        let mut sink_by_level = [0u64; MAX_LEVEL];
        let mut recover_by_level = [0u64; MAX_LEVEL];
        let mut bsp_by_level = [0u64; MAX_LEVEL];
        for lv in 0..MAX_LEVEL {
            sink_by_level[lv] = r.sink_by_level[lv];
            recover_by_level[lv] = r.recover_by_level[lv];
            // t1buy + t1sell（buy=idx0, sell=idx1）= 该级别 type1 BSP 频率。
            bsp_by_level[lv] = s.bsp_by_level[lv][0] + s.bsp_by_level[lv][1];
        }
        ScaleResult {
            max_active_level: max_active,
            passbands,
            radial_viol_sink: crate::recursive_t::prove_guards::count_radial_scaling_violations(
                &sink_by_level,
            ),
            radial_viol_bsp: crate::recursive_t::prove_guards::count_radial_scaling_violations(
                &bsp_by_level,
            ),
            sink_by_level,
            recover_by_level,
            bsp_by_level,
            n_reruns: s.n_reruns,
            n_sinks: r.n_sinks,
            n_recovers: r.n_recovers,
            n_dir_mismatch: g.n_dir_mismatch,
            n_dir_checks: g.n_dir_checks,
            n_sink_recover_imbalance: g.n_sink_recover_imbalance,
            n_campaigns_checked: g.n_campaigns_checked,
            n_neg_pnl_campaigns: g.n_neg_pnl_campaigns,
            ops_bsp: g.ops_by_trigger[0],
            ops_emergence: g.ops_by_trigger[1],
            ops_eod: g.ops_by_trigger[2],
            n_ops_without_trigger: g.n_ops_without_trigger,
        }
    }

    /// 打印单尺度 prove 守卫读数（尺度不变性证据）。
    fn report_scale(tag: &str, n_bars: usize, r: &ScaleResult) {
        eprintln!(
            "\n[{tag}] bars={n_bars} reruns={} | 最深活跃滤波器层={} sink={} recover={}",
            r.n_reruns, r.max_active_level, r.n_sinks, r.n_recovers
        );
        // ① 4 panic 守卫：跑到这里 = sink_descends/sigma_quota/relabel_invariant 零 fire；
        //    bsp_triggers_operation 由 n_ops_without_trigger==0 确认（panic 守卫下恒 0）。
        eprintln!(
            "  [PANIC 守卫尺度不变性] 进程零 panic ⇒ sink_descends/sigma_quota/relabel_invariant 全程成立 \
             | bsp_triggers_operation: ops(bsp={} emrg={} eod={}) no_trigger={}（=0 即成立）",
            r.ops_bsp, r.ops_emergence, r.ops_eod, r.n_ops_without_trigger
        );
        // ② radial_scaling（f∝λ⁻ᵏ）：违反层数（0=全程频率随级别非增，几何递减成立）。
        eprintln!(
            "  [radial_scaling f∝λ⁻ᵏ] sink_by_level 违反层数={} | type1_bsp_by_level 违反层数={}",
            r.radial_viol_sink, r.radial_viol_bsp
        );
        let sbl: Vec<u64> = r.sink_by_level.to_vec();
        let rbl: Vec<u64> = r.recover_by_level.to_vec();
        let bbl: Vec<u64> = r.bsp_by_level.to_vec();
        eprintln!("     sink_by_level    ={sbl:?}");
        eprintln!("     recover_by_level ={rbl:?}");
        eprintln!("     type1bsp_by_level={bbl:?}");
        let pb: Vec<u64> = r.passbands.to_vec();
        eprintln!("     滤波器通带(走势组)/lvl={pb:?}");
        // ③ 观测计数守卫（L2 regime 读数，非 panic）。
        eprintln!(
            "  [观测计数 L2] dir_mismatch={}/{} sink≠recover campaigns={}/{} neg_pnl campaigns={}",
            r.n_dir_mismatch, r.n_dir_checks, r.n_sink_recover_imbalance,
            r.n_campaigns_checked, r.n_neg_pnl_campaigns
        );
    }

    /// **2019后不再入场根因追踪**（编排者 2026-06-22）：采样核心 units 衰减 + free/withdrawn/stage +
    /// enter/ascend/flip 累计 + buy 路由分类，定位「2018 做空赚完后引擎为何不再 enter 重新建仓」。
    /// 关键判据：enter 仅在 highest_active()==None（全塔空仓）触发；若核心 units 几何衰减但永不 ≤EPS
    /// → highest_active 恒 Some → enter 恒不触发 → free 闲置。
    /// 跑法：`cargo test --release recursive_t::rec_stream::tests::btc_no_reentry_trace -- --exact --ignored --nocapture`
    #[test]
    #[ignore = "2019后不入场追踪，需 btc_1m_full.json"]
    fn btc_no_reentry_trace() {
        use crate::recursive_t::backtest_run::load_clean_ohlc;
        use std::path::PathBuf;
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("analysis/data_cache/btc_1m_full.json");
        if !path.exists() {
            eprintln!("数据缺失 {path:?}");
            return;
        }
        let (o, h, l, c) = load_clean_ohlc(&path);
        let n = c.len();
        // 关键 bar：熊1底 248736 + 2019 各点 + 各周期边界。
        let key: std::collections::HashSet<usize> =
            [248736usize, 250000, 267706, 300000, 400000, 694639, 971781, 1345277, 2218375]
                .into_iter()
                .collect();

        let mut s = RecStream::new(PerfectionMode::Structural);
        eprintln!("\n===== BTC 2019后不再入场 状态追踪（Structural）=====");
        eprintln!(
            "{:>9} {:>8} {:>4} {:>12} {:>4} {:>11} {:>11} {:>5} {:>4} {:>4} {:>4} {:>7} {:>6} {:>6} {:>8}",
            "bar", "价", "核层", "核units", "活层", "free", "withdrawn", "stg", "ent", "asc",
            "flp", "buyCore", "sink", "recov", "BSP累"
        );
        let mut units_dump: Vec<(usize, Vec<f64>)> = Vec::new();
        for i in 0..n {
            s.push_bar(o[i], h[i], l[i], c[i]);
            if i % 200_000 == 0 || i == n - 1 || key.contains(&i) {
                let r = s.driver().root();
                let (hl, hu) = match r.highest_active() {
                    Some(k) => (k as i64, r.instance(k).units),
                    None => (-1, 0.0),
                };
                let stg = match r.stage().as_u8() {
                    0 => "Cost",
                    1 => "CapR",
                    _ => "Earn",
                };
                eprintln!(
                    "{:>9} {:>8.0} {:>4} {:>12.6} {:>4} {:>11.0} {:>11.0} {:>5} {:>4} {:>4} {:>4} {:>7} {:>6} {:>6} {:>8}",
                    i, c[i], hl, hu, r.n_active(), r.free(), r.withdrawn_total(), stg,
                    r.n_enters, r.n_ascends, r.n_flips, r.buy_core, r.n_sinks, r.n_recovers,
                    s.bsp_counts.iter().sum::<u64>()
                );
                if key.contains(&i) {
                    units_dump.push((i, (0..MAX_LEVEL).map(|k| r.instance(k).units).collect()));
                }
            }
        }
        let _ = s.finish();

        // ── 关键 bar 各 level units（看核心几何衰减是否到 0 / 是否恢复）──
        eprintln!("\n--- 关键 bar 各 level units（核心几何衰减，>EPS=1e-12 即'活着'阻止 enter）---");
        eprintln!(
            "{:>9} {:>8} {:>13} {:>13} {:>13} {:>13} {:>13} {:>13}",
            "bar", "价", "L0", "L1", "L2", "L3", "L4", "L5"
        );
        for (b, u) in &units_dump {
            eprintln!(
                "{:>9} {:>8.0} {:>13.9} {:>13.9} {:>13.9} {:>13.9} {:>13.9} {:>13.9}",
                b, c[*b], u[0], u[1], u[2], u[3], u[4], u[5]
            );
        }
        eprintln!(
            "\n判读：ent列在 248736 后是否不变=2018后零enter；核units衰减但>EPS=highest_active恒Some阻止enter；\
             buyCore增长但ent不变=核心级买点走ascend/flip非enter；free大但锁不进市场。"
        );
    }
}
