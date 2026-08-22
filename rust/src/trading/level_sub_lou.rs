//! `SubLou`——递归子 LOU（从 `level_operating_unit.rs` 迁出，map #1186 / #1192 C01 刀②）。
//!
//! 迁移仅动文件边界与可见性：`new`/`step`/`cascade_close_with_self` 提升为
//! `pub(super)` 供父模块 `VoiceUnit` 与测试模块调用；判据语义零改动。
//! 消费面经父模块 `pub use level_sub_lou::SubLou;` 重导出保持原路径。

use super::super::center_book::CenterBook;
use super::super::config::{OrganicConfig, SubMode};
use super::super::ledger::{DiffSide, OrganicLedger};
use super::super::types::*;
use super::BarRows;
use crate::buysellpoint::BspKind;
use crate::divergence::DivKind;
use crate::stroke::Direction;

/// 递归子 LOU —— 38课程式在反向走势窗口内的方向镜像实例（递归赋格，2026-06-11）。
///
/// 符号交替塔 (−1)^n（设计报告 §2.4）：奇数深度 = 反弹腿（先买后卖，
/// `DiffSide::Long`——REV 窗口内 k−1 级别反弹段的低买高卖），偶数深度 =
/// 短差（先卖后买，回到父方向）。槽占用与否在账本（单一真相源，osc 先例）；
/// 本结构只持级别/路径坐标 + 子节点。
///
/// 532 双通道在子层的复制（设计报告 §2.5）：开腿谓词（本级别反向段存在
/// 陈述：type1/盘背三岔镜像）与闭腿谓词（段终结陈述：同锚 type1 配对 +
/// type3 逃逸 + 边界触线）是两个独立 match——不共享触发集。
///
/// 三范畴终止条件（设计报告 §4.1）：
/// - 存在论：子级别 < FIRST_BSP_LADDER ⇒ 节点不实例化（调用方守卫）；
/// - 经济：锚中枢振幅 (ZG−ZD)/c < 2×sub_friction_rt ⇒ 拒开（计数）；
/// - 深度预算：path.depth() ≥ cfg.rev_sub_depth ⇒ 不再生成子节点。
#[derive(Debug, Clone)]
pub struct SubLou {
    /// 操作级别（直接父级别 − 1）。
    pub ladder: usize,
    /// 槽路径（父路径 + 本级；SlotKey = rev_path(home, path)）。
    pub path: RevPath,
    /// 嵌套子节点（深度预算内、本腿开放时按 bar 惰性实例化）。
    pub child: Option<Box<SubLou>>,
    /// Fractal 模式翻转检测前值（本级别 D3 方向行上一观测；Zhongshu 不消费）。
    /// 实例化时 None：首个观测只建立基准不触发——开腿恒等待一次完整的
    /// Up→Down 翻转（顶分型确认），不在陈旧方向上行动。
    last_dir: Option<Direction>,
    // ── Sequence38 状态（仅 sub_mode=Sequence38 消费；38课:36 程式）──
    /// 第一段起点低点参照：节点激活/上次买回以来的 close 运行最低。
    /// 开腿（先卖）时冻结进 seg1_low；买回后复位为当下 close 重新累计
    /// （中间循环：程式在 REV 窗口内反复）。
    run_low: f64,
    /// 开腿时冻结的"第一段低点"（38课"不跌破第一段低点"的比较基准）。
    /// None = 未持腿。close 分辨率（low_since_open / RevLeg.low_since_open
    /// 同口径）。
    seg1_low: Option<f64>,
    /// 持腿期 close 运行最低（"不跌破"判据的左操作数；含开腿 bar）。
    low_since_open: f64,
}

impl SubLou {
    pub(super) fn new(ladder: usize, path: RevPath) -> Self {
        SubLou {
            ladder,
            path,
            child: None,
            last_dir: None,
            run_low: f64::INFINITY,
            seg1_low: None,
            low_since_open: f64::INFINITY,
        }
    }

    /// 循环方向 = 深度奇偶（符号交替塔）。
    fn side(&self) -> DiffSide {
        if self.path.depth() % 2 == 1 {
            DiffSide::Long
        } else {
            DiffSide::Short
        }
    }

    /// 开腿三岔镜像（仅震荡型——逃逸型开腿在父层已被数据否证为一致负，
    /// V2o 裁决在子层先验继承；Long = 买侧，Short = 卖侧）。
    fn open_trigger(&self, evs: &[BspEvent], devs: &[DivEvent]) -> bool {
        match self.side() {
            DiffSide::Long => {
                evs.iter().any(|e| {
                    e.confirmed
                        && match e.class {
                            BspClass::Buy1 => true,
                            BspClass::Buy2 | BspClass::Buy3 => false,
                            BspClass::Sell1 | BspClass::Sell2 | BspClass::Sell3 => false,
                        }
                }) || devs
                    .iter()
                    .any(|d| d.kind == DivKind::Consolidation && d.direction == Direction::Down)
            }
            DiffSide::Short => {
                evs.iter().any(|e| {
                    e.confirmed
                        && match e.class {
                            BspClass::Sell1 => true,
                            BspClass::Sell2 | BspClass::Sell3 => false,
                            BspClass::Buy1 | BspClass::Buy2 | BspClass::Buy3 => false,
                        }
                }) || devs
                    .iter()
                    .any(|d| d.kind == DivKind::Consolidation && d.direction == Direction::Up)
            }
        }
    }

    /// 闭腿配对镜像（hard type3 > pre type3 > 同锚 type1 > 边界触线——
    /// step_down_paired 同优先序）。返回是否触发。
    fn close_trigger(
        &self,
        cfg: &OrganicConfig,
        evs: &[BspEvent],
        devs: &[DivEvent],
        anchor_cs: Option<i64>,
        boundary: Option<f64>,
        c: f64,
    ) -> bool {
        let _ = devs; // 背驰事件不在子腿闭腿集（type1 本体配对，盘背买/卖不闭）
        let (t3, t1) = match self.side() {
            DiffSide::Long => (BspClass::Sell3, BspClass::Sell1),
            DiffSide::Short => (BspClass::Buy3, BspClass::Buy1),
        };
        let hard = evs.iter().any(|e| e.class == t3 && e.confirmed);
        let pre = cfg.pre_type3 && evs.iter().any(|e| e.class == t3 && !e.confirmed);
        let t1_paired = evs
            .iter()
            .any(|e| e.confirmed && e.class == t1 && e.cs == anchor_cs);
        let touch = match self.side() {
            DiffSide::Long => boundary.is_some_and(|b| c >= b),
            DiffSide::Short => boundary.is_some_and(|b| c <= b),
        };
        hard || pre || t1_paired || touch
    }

    /// 每 bar 一步（父腿 DownLeg 期间由 VoiceUnit 驱动；递归驱动子树）。
    /// `parent_shares` = 直接父腿当前敞口（预算基，设计报告 §3.3：
    /// "budget(node) = 父层在本节点域内释放的敞口"）。
    #[allow(clippy::too_many_arguments)]
    pub(super) fn step(
        &mut self,
        cfg: &OrganicConfig,
        rows: &BarRows,
        book: &CenterBook,
        c: f64,
        bar: i64,
        ledger: &mut OrganicLedger,
        home: usize,
        parent_shares: f64,
        counters: &mut Counters,
    ) {
        match cfg.sub_mode {
            SubMode::Fractal => {
                self.step_fractal(
                    cfg,
                    rows,
                    book,
                    c,
                    bar,
                    ledger,
                    home,
                    parent_shares,
                    counters,
                );
                return;
            }
            SubMode::Sequence38 => {
                self.step_sequence38(
                    cfg,
                    rows,
                    book,
                    c,
                    bar,
                    ledger,
                    home,
                    parent_shares,
                    counters,
                );
                return;
            }
            SubMode::CounterSeg => {
                self.step_counterseg(
                    cfg,
                    rows,
                    book,
                    c,
                    bar,
                    ledger,
                    home,
                    parent_shares,
                    counters,
                );
                return;
            }
            SubMode::Zhongshu => {}
        }
        let k = self.ladder;
        let key = SlotKey::rev_path(home, self.path);
        let evs = &rows.evs[k];
        let devs = &rows.devs[k];
        let open = ledger.open_slot(key).map(|l| (l.anchor, l.cycle.shares));
        match open {
            Some((anchor, my_shares)) => {
                // 子节点递归（深度预算 ∧ 存在论下限；本腿开放 = 子域存在）
                if self.path.depth() < cfg.rev_sub_depth && k >= 1 && k - 1 >= FIRST_BSP_LADDER {
                    let path = self.path;
                    let child = self
                        .child
                        .get_or_insert_with(|| Box::new(SubLou::new(k - 1, path.child(k - 1))));
                    child.step(cfg, rows, book, c, bar, ledger, home, my_shares, counters);
                }
                let LegAnchor::Center { cs, boundary, .. } = anchor else {
                    unreachable!(
                        "sub 腿恒以 Center 锚开（open_sub 路径唯一）；此 arm 是类型完备性要求"
                    )
                };
                if self.close_trigger(cfg, evs, devs, cs, boundary, c) {
                    // 级联不变式：本腿闭合先平掉全部子树腿（子域随本腿消失）
                    self.cascade_close_children(home, c, bar, ledger, counters);
                    Self::close_one(key, c, bar, ledger, counters);
                    counters.n_sub_close += 1;
                }
            }
            None => {
                if (evs.is_empty() && devs.is_empty()) || !self.open_trigger(evs, devs) {
                    return;
                }
                // 锚域：本级别存活中枢（镜像震荡型的 alive 单一真相源）
                let Some(lc) = book.alive(k) else {
                    counters.n_sub_nocenter_rejects += 1;
                    return;
                };
                if !(lc.zd.is_finite() && lc.zg.is_finite()) {
                    counters.n_sub_nocenter_rejects += 1;
                    return;
                }
                // 经济终止条件（振幅 ≥ 2×往返摩擦）+ 兑现空间（Long 镜像 c>ZD）
                let amp_ok = (lc.zg - lc.zd) / c >= 2.0 * cfg.sub_friction_rt;
                let space_ok = match self.side() {
                    DiffSide::Long => c < lc.zg,
                    DiffSide::Short => c > lc.zd,
                };
                if !(amp_ok && space_ok) {
                    counters.n_sub_amp_rejects += 1;
                    return;
                }
                if ledger.phase.is_earning() {
                    counters.n_sub_earning_rejects += 1;
                    return;
                }
                let boundary = match self.side() {
                    DiffSide::Long => Some(lc.zg),
                    DiffSide::Short => Some(lc.zd),
                };
                if ledger.open_sub(
                    key,
                    parent_shares,
                    c,
                    bar,
                    LegAnchor::Center {
                        cs: Some(lc.seg_start),
                        boundary,
                        zg: None,
                        kind: AnchorKind::Bsp(BspKind::Type1),
                    },
                    self.side(),
                ) {
                    counters.n_sub_open += 1;
                }
            }
        }
    }

    /// Fractal 模式每 bar 一步——38课向下段程式的笔级直读（2026-06-11 任务）。
    ///
    /// 操作锚 = 本级别 D3 方向行翻转（confirmed 笔/move 端点的方向读出）：
    /// - **开腿（先卖）**：Up→Down 翻转 = 顶分型确认（新向下笔/move 出现，
    ///   反弹顶已成立）→ 卖出 parent_shares；
    /// - **闭腿（后买）**：Down→Up 翻转 = 底分型确认 → 买回。
    ///
    /// 与 Zhongshu 模式的三点差异（O_sub0 否证根因的逐点拆除）：
    /// 1. 无"存活中枢"前置——父 REV 腿趋势运行中子域恒可定义（74% 无中枢拒
    ///    在此模式不存在）；
    /// 2. 无 ZG/ZD 边界与振幅经济门——分型即信号，短差幅度由市场给出；
    /// 3. 方向恒 Short（38课"向下段的运作……是先卖后买"——子腿复制父窗口
    ///    的段内韵律，符号交替塔 (−1)^n 是中枢域读法的产物，此处不适用）。
    ///
    /// 同 bar 翻转折叠的诚实声明：方向行是 bar 末状态——若同一 bar 内确认
    /// 多个笔端点（顶+底），中间翻转不可见，该往返短差不被捕获（漏单非错单）。
    #[allow(clippy::too_many_arguments)]
    fn step_fractal(
        &mut self,
        cfg: &OrganicConfig,
        rows: &BarRows,
        book: &CenterBook,
        c: f64,
        bar: i64,
        ledger: &mut OrganicLedger,
        home: usize,
        parent_shares: f64,
        counters: &mut Counters,
    ) {
        let k = self.ladder;
        let key = SlotKey::rev_path(home, self.path);
        let now = rows.dir(k);
        let top_flip = self.last_dir == Some(Direction::Up) && now == Some(Direction::Down);
        let bottom_flip = self.last_dir == Some(Direction::Down) && now == Some(Direction::Up);
        if now.is_some() {
            self.last_dir = now;
        }
        match ledger.open_slot(key).map(|l| l.cycle.shares) {
            Some(my_shares) => {
                // 子节点递归（深度预算 ∧ 存在论下限 = bi 级：k−1 ≥ 1。
                // bi 没有内部分型可用——递归在 bi 级自然终止，构成性例外消除）
                if self.path.depth() < cfg.rev_sub_depth && k >= 2 {
                    let path = self.path;
                    let child = self
                        .child
                        .get_or_insert_with(|| Box::new(SubLou::new(k - 1, path.child(k - 1))));
                    child.step(cfg, rows, book, c, bar, ledger, home, my_shares, counters);
                }
                if bottom_flip {
                    self.cascade_close_children(home, c, bar, ledger, counters);
                    Self::close_one(key, c, bar, ledger, counters);
                    counters.n_sub_close += 1;
                }
            }
            None => {
                if !top_flip {
                    return;
                }
                if ledger.phase.is_earning() {
                    counters.n_sub_earning_rejects += 1;
                    return;
                }
                // ── P1 成本门（35课，sub_cost_gate）：级别可操作性 ──
                // θ_eff = max(θ_q, sub_cost_k × sub_friction_rt)；本级别典型
                // 中枢振幅 θ_q（DepthRef 因果滚动中位数，零前瞻）不够覆盖
                // k 倍往返成本 ⇒ 级别自动关闭。参照不可定义（bi 级无中枢
                // 事件流 / warm-up）⇒ 保守拒绝并独立计数（不静默放行）。
                if cfg.sub_cost_gate {
                    let dr = rows.depth.expect(
                        "sub_cost_gate ⇒ 调用方必提供 DepthRef（capability，runner 恒提供）",
                    );
                    use super::super::config::{SUB_COST_MIN_OBS, SUB_COST_Q};
                    match dr.theta(k, None, SUB_COST_Q, SUB_COST_MIN_OBS) {
                        Some(theta_q) => {
                            let theta_eff = theta_q.max(cfg.sub_cost_k * cfg.sub_friction_rt);
                            if theta_q < theta_eff {
                                counters.n_sub_cost_rejects += 1;
                                return;
                            }
                        }
                        None => {
                            counters.n_sub_cost_noref_rejects += 1;
                            return;
                        }
                    }
                }
                // ── P1 41课门（sub_l41_gate）：父级别走势无衰竭 ⇒ 不做反向 ──
                // 父级别 = 直接上级（k+1，路径每深一层降一级——局部依赖）。
                if cfg.sub_l41_gate {
                    let te = rows.l41.expect(
                        "sub_l41_gate ⇒ 调用方必提供 TrendExhaustion（capability，runner 恒提供）",
                    );
                    if te.down_unexhausted(k + 1) {
                        counters.n_sub_l41_rejects += 1;
                        return;
                    }
                }
                // 段尺度锚：无中枢边界——闭腿纯翻转驱动（+ 父腿级联强闭）
                if ledger.open_sub(
                    key,
                    parent_shares,
                    c,
                    bar,
                    LegAnchor::SegmentScale,
                    DiffSide::Short,
                ) {
                    counters.n_sub_open += 1;
                }
            }
        }
    }

    /// Sequence38 模式每 bar 一步——38课向下段程式的段间盘整背驰严格形式
    /// （2026-06-11 任务；判据原文 38课:36 + 答疑:296，见 SubMode docstring）。
    ///
    /// 状态机（向下段先卖后买，方向恒 Short——同 Fractal 论据）：
    /// - **开腿（先卖）**：本级别 k 或次级别 k−1 的盘整背驰卖点
    ///   （Consolidation × Sell）——第一段（反弹）的盘整背驰结束点；
    ///   冻结 seg1_low = run_low（第一段起点低点的 close 分辨率读数）。
    /// - **闭腿（买回）三岔**（任一为真即买回；计数归因取最强证据）：
    ///   (1) 段间盘整背驰买点（Consolidation × Buy @ k）；
    ///   (2) 不跌破第一段低点：low_since_open > seg1_low ∧ 次级别结构
    ///       确认第二段完成（sub_confirm(k, Buy)——答疑:296 判据本体，
    ///       非纯几何触线）；
    ///   (3) 新的下跌背驰（Trend × Buy ∨ confirmed Buy1 @ k）——观望出口。
    /// - 买回后 run_low 复位为当下 close（中间循环重新累计）。
    ///
    /// 与 Zhongshu 模式的差异：无"存活中枢"前置、无 ZG/ZD 边界与振幅
    /// 经济门——操作域 = 段序列本身（38课程式的域就是同级别分解的段）。
    /// 与 Fractal 模式的差异（近似 vs 严格的唯一轴）：进出信号源 =
    /// div 事件流 + 次级别结构确认，非笔级方向行翻转。
    #[allow(clippy::too_many_arguments)]
    fn step_sequence38(
        &mut self,
        cfg: &OrganicConfig,
        rows: &BarRows,
        book: &CenterBook,
        c: f64,
        bar: i64,
        ledger: &mut OrganicLedger,
        home: usize,
        parent_shares: f64,
        counters: &mut Counters,
    ) {
        use crate::buysellpoint::Side;
        let _ = book; // 无中枢依赖：操作域 = 段序列本身（38课程式）
        let k = self.ladder;
        let key = SlotKey::rev_path(home, self.path);
        // 第一段低点参照：close 运行最低（节点激活/上次买回以来；含本 bar）
        self.run_low = self.run_low.min(c);
        let evs = &rows.evs[k];
        let devs = &rows.devs[k];
        match ledger.open_slot(key).map(|l| l.cycle.shares) {
            Some(my_shares) => {
                self.low_since_open = self.low_since_open.min(c);
                // 子节点递归（深度预算 ∧ 存在论下限 = div 事件流承载下界，
                // 同 Zhongshu：k−1 ≥ FIRST_BSP_LADDER）
                if self.path.depth() < cfg.rev_sub_depth && k >= 1 && k - 1 >= FIRST_BSP_LADDER {
                    let path = self.path;
                    let child = self
                        .child
                        .get_or_insert_with(|| Box::new(SubLou::new(k - 1, path.child(k - 1))));
                    child.step(cfg, rows, book, c, bar, ledger, home, my_shares, counters);
                }
                // (1) 段间盘整背驰买点
                let cons_buy = devs
                    .iter()
                    .any(|d| d.kind == DivKind::Consolidation && d.side() == Side::Buy);
                // (2) 不跌破第一段低点 ∧ 次级别结构确认（答疑:296）
                let nobreak = self.seg1_low.is_some_and(|s1| self.low_since_open > s1)
                    && rows.sub_confirm(k, Side::Buy);
                // (3) 新的下跌背驰（观望出口）
                let new_div = devs
                    .iter()
                    .any(|d| d.kind == DivKind::Trend && d.side() == Side::Buy)
                    || evs.iter().any(|e| e.class == BspClass::Buy1 && e.confirmed);
                if cons_buy || nobreak || new_div {
                    self.cascade_close_children(home, c, bar, ledger, counters);
                    Self::close_one(key, c, bar, ledger, counters);
                    counters.n_sub_close += 1;
                    if cons_buy {
                        counters.n_sub_seq_consbuy_close += 1;
                    } else if nobreak {
                        counters.n_sub_seq_nobreak_close += 1;
                    } else {
                        counters.n_sub_seq_newdiv_close += 1;
                    }
                    // 中间循环复位：新一轮第一段低点从当下重新累计
                    self.seg1_low = None;
                    self.run_low = c;
                    self.low_since_open = f64::INFINITY;
                }
            }
            None => {
                // 开腿：本级别或次级别的盘整背驰卖点
                let cons_sell = |ds: &[DivEvent]| {
                    ds.iter()
                        .any(|d| d.kind == DivKind::Consolidation && d.side() == Side::Sell)
                };
                let here = cons_sell(devs);
                let sub = k >= 1 && cons_sell(&rows.devs[k - 1]);
                if !(here || sub) {
                    return;
                }
                if ledger.phase.is_earning() {
                    counters.n_sub_earning_rejects += 1;
                    return;
                }
                // 段尺度锚：无中枢边界——闭腿纯事件/判据驱动（+ 父腿级联强闭）
                if ledger.open_sub(
                    key,
                    parent_shares,
                    c,
                    bar,
                    LegAnchor::SegmentScale,
                    DiffSide::Short,
                ) {
                    counters.n_sub_open += 1;
                    self.seg1_low = Some(self.run_low);
                    self.low_since_open = c;
                }
            }
        }
    }

    /// CounterSeg 闭腿判据——本级别反向段**终点**的买卖点（开腿的镜像侧，
    /// 恒 confirmed；candidate 不是原文操作点，not6 先例）。
    /// Long 腿（奇数深度，持反弹段）：confirmed Sell1 ∨ 盘背卖 ∨ confirmed
    /// Sell3（中枢向下离开 = 向上反向段终结的更强结构陈述）；Short 镜像。
    fn counterseg_close_trigger(&self, evs: &[BspEvent], devs: &[DivEvent]) -> bool {
        let (t1, t3, div_dir) = match self.side() {
            DiffSide::Long => (BspClass::Sell1, BspClass::Sell3, Direction::Up),
            DiffSide::Short => (BspClass::Buy1, BspClass::Buy3, Direction::Down),
        };
        evs.iter()
            .any(|e| e.confirmed && (e.class == t1 || e.class == t3))
            || devs
                .iter()
                .any(|d| d.kind == DivKind::Consolidation && d.direction == div_dir)
    }

    /// CounterSeg 模式每 bar 一步——嵌套递归并发赋格的严格形式子腿
    /// （2026-06-12 任务；判据见 `SubMode::CounterSeg` docstring）。
    ///
    /// 操作对象 = 父腿反向走势窗口内、本级别的反向走势类型，进出恒用本级别
    /// 买卖点：开腿 = `open_trigger`（Zhongshu 模式同一谓词——confirmed
    /// type1 ∨ 盘背，符号交替塔定方向侧），闭腿 = `counterseg_close_trigger`
    /// （镜像侧 + type3 终结）。与 Zhongshu 模式的唯一拆除 = 中枢域前置
    /// （alive 中枢 / ZG-ZD 边界 / 振幅域门）——O_sub0 否证根因"父腿前提
    /// 否定子腿前提"在此模式无承载位；级别可操作性由 35课成本门
    /// （sub_cost_gate，DepthRef 因果滚动中位数）独立判定。
    /// 段尺度锚：无中枢边界——闭腿纯事件驱动（+ 父腿级联强闭）。
    #[allow(clippy::too_many_arguments)]
    fn step_counterseg(
        &mut self,
        cfg: &OrganicConfig,
        rows: &BarRows,
        book: &CenterBook,
        c: f64,
        bar: i64,
        ledger: &mut OrganicLedger,
        home: usize,
        parent_shares: f64,
        counters: &mut Counters,
    ) {
        let _ = book; // 本模式无中枢依赖（中枢域前置已拆除）；仅透传给子节点
        let k = self.ladder;
        let key = SlotKey::rev_path(home, self.path);
        let evs = &rows.evs[k];
        let devs = &rows.devs[k];
        match ledger.open_slot(key).map(|l| l.cycle.shares) {
            Some(my_shares) => {
                // 子节点递归（深度预算 ∧ 存在论下限：笔 = a0，62/77/78课——
                // k−1 < FIRST_BSP_LADDER 即无中枢/买卖点概念，递归自然终止）
                if self.path.depth() < cfg.rev_sub_depth && k >= 1 && k - 1 >= FIRST_BSP_LADDER {
                    let path = self.path;
                    let child = self
                        .child
                        .get_or_insert_with(|| Box::new(SubLou::new(k - 1, path.child(k - 1))));
                    child.step(cfg, rows, book, c, bar, ledger, home, my_shares, counters);
                }
                if self.counterseg_close_trigger(evs, devs) {
                    // 级联不变式：本腿闭合先平掉全部子树腿（子域随本腿消失）
                    self.cascade_close_children(home, c, bar, ledger, counters);
                    Self::close_one(key, c, bar, ledger, counters);
                    counters.n_sub_close += 1;
                }
            }
            None => {
                if (evs.is_empty() && devs.is_empty()) || !self.open_trigger(evs, devs) {
                    return;
                }
                if ledger.phase.is_earning() {
                    counters.n_sub_earning_rejects += 1;
                    return;
                }
                // ── 35课成本门（递归经济终止）：本级别典型中枢振幅 θ_q
                // （DepthRef 因果滚动中位数，零前瞻）≥ sub_cost_k×friction
                // 才可操作；参照不可定义 ⇒ 保守拒绝（不静默放行）。──
                if cfg.sub_cost_gate {
                    let dr = rows.depth.expect(
                        "sub_cost_gate ⇒ 调用方必提供 DepthRef（capability，runner 恒提供）",
                    );
                    use super::super::config::{SUB_COST_MIN_OBS, SUB_COST_Q};
                    match dr.theta(k, None, SUB_COST_Q, SUB_COST_MIN_OBS) {
                        Some(theta_q) => {
                            if theta_q < cfg.sub_cost_k * cfg.sub_friction_rt {
                                counters.n_sub_cost_rejects += 1;
                                return;
                            }
                        }
                        None => {
                            counters.n_sub_cost_noref_rejects += 1;
                            return;
                        }
                    }
                }
                if ledger.open_sub(
                    key,
                    parent_shares,
                    c,
                    bar,
                    LegAnchor::SegmentScale,
                    self.side(),
                ) {
                    counters.n_sub_open += 1;
                }
            }
        }
    }

    /// 闭一条子腿并按对账面计数（win = profit > 0）。
    fn close_one(
        key: SlotKey,
        c: f64,
        bar: i64,
        ledger: &mut OrganicLedger,
        counters: &mut Counters,
    ) {
        let n0 = ledger.completed.len();
        ledger.close_diff(key, c, bar);
        if ledger.completed.len() > n0 {
            counters.sub_pairs += 1;
            let (_, cyc) = ledger.completed.last().expect("close_diff 刚 push");
            let profit = cyc.profit();
            if profit > 0.0 {
                counters.sub_wins += 1;
            }
            counters.sub_cash += profit;
        }
    }

    /// 级联强闭全部子孙腿（最深优先），并销毁子树。
    fn cascade_close_children(
        &mut self,
        home: usize,
        c: f64,
        bar: i64,
        ledger: &mut OrganicLedger,
        counters: &mut Counters,
    ) {
        if let Some(child) = self.child.as_mut() {
            child.cascade_close_children(home, c, bar, ledger, counters);
            let key = SlotKey::rev_path(home, child.path);
            if ledger.open_slot(key).is_some() {
                Self::close_one(key, c, bar, ledger, counters);
                counters.n_sub_forced_close += 1;
            }
        }
        self.child = None;
    }

    /// 级联强闭含自身（父 REV 腿闭合时由 VoiceUnit 调用）。
    pub(super) fn cascade_close_with_self(
        &mut self,
        home: usize,
        c: f64,
        bar: i64,
        ledger: &mut OrganicLedger,
        counters: &mut Counters,
    ) {
        self.cascade_close_children(home, c, bar, ledger, counters);
        let key = SlotKey::rev_path(home, self.path);
        if ledger.open_slot(key).is_some() {
            Self::close_one(key, c, bar, ledger, counters);
            counters.n_sub_forced_close += 1;
        }
    }
}
