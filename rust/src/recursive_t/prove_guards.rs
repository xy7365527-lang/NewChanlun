//! **prove 守卫族**（递归 T + flat 共享，编排者裁决 2026-06-21）。
//!
//! 在 `prove_tw_neutral`（总财富守恒，唯一已有守卫）之上补全四个结构/会计不变量守卫。守卫 =
//! 验收标准（非回测指标）：把"走势终完美""带通滤波器独立正贡献"等缠论/架构断言落成可观测的
//! 检查点，让真实数据划定每条不变量的有效域边界。
//!
//! ## 两种守卫模式（沿 `fugue_v3::prove` 先例）
//! - **panic 守卫**（结构必然，违反=停下来）：`prove_nav_neutral` 风格。本模块的
//!   `prove_bsp_triggers_operation`（每个主动操作必有触发源）属此类——架构上不存在无触发源的
//!   操作，违反即"走势跟随残留"bug。
//! - **观测计数守卫**（假设可能被经验否定）：`count_chiral_violations` 风格（"永远交替"假设被
//!   4/8 标的 panic 否定后从 panic 降级为计数器）。`prove_sink_recover_balance` /
//!   `prove_per_level_pnl` / `prove_direction_matches_trend` 属此类——BTC 等强牛标的已知会违反
//!   （诊断显示 recover<sink、深层级别短差失血、核心被低级别 BSP ping-pong flip）。计数违反次数
//!   而非 panic，**让数据告诉我们哪些不变量在哪些 regime 被破坏**。
//!
//! ## 认识论等级（formalization-validity-domain）
//! - `prove_bsp_triggers_operation`：操作触发源可归因 = **L0 结构**（panic）。
//! - 其余三个守卫的「违反计数」：在真实标的上的读数 = **L2/L3**（否定性结果缩小有效域）。
//!
//! ## 谱系
//! - 项目记忆 `project_recursive_t_architecture_v2`（走势终完美保证短差配对）、
//!   `project_t_short_leg_regime_function`（空头腿=亏损唯一来源=regime 函数）、
//!   `project_t_cross_level_coupling_falsified`（深层级别短差灾难失血）。
//! - `fugue_v3::prove::count_chiral_violations`（观测计数模式的先例）。

use crate::trading::types::Polarity;

/// 单个主动操作的触发源（`prove_bsp_triggers_operation`）。`step`/`on_bar` 每个执行段开头设定，
/// 原子操作（enter/sink/recover/drain/ascend/clear_all）执行时验证非 `None`。
///
/// **架构断言**：所有主动交易操作只能经两条路径——`route_bsp`（BSP 驱动）或 `emergence_upgrade`
/// （涌现驱动）；外加边界算子收尾（`Eod`）。强平（价格驱动）不经原子操作函数，故不在此分类内
/// （独立的 liquidation 计数）。`None` 触发的操作 = 走势跟随残留 = bug。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OpTrigger {
    /// 本执行段尚无触发源（on_bar 起始态；原子操作在此态执行 = 残留 bug）。
    #[default]
    None,
    /// BSP 路由（`route_bsp`）：买卖点驱动的 enter/sink/recover/drain/flip/ascend。
    Bsp,
    /// 自下而上涌现升级（`emergence_upgrade`）：emergent_top 驱动的 ascend（合法非 BSP 操作）。
    Emergence,
    /// 收尾（`finish`→`clear_all`）：eod 全平（合法非 BSP 操作）。
    Eod,
}

/// prove 守卫状态机（per 引擎实例一份）。内部基线 + 公开观测计数。
#[derive(Debug, Clone)]
pub struct ProveGuards {
    /// 级别/ladder 数（rec=MAX_LEVEL，flat=MAX_LADDER）。
    n_levels: usize,

    // ── 内部状态 ──
    /// 当前操作段触发源（`prove_bsp_triggers_operation`）。
    cur_trigger: OpTrigger,
    /// 全程 sink/recover 累计（campaign 内 diff 用）。
    total_sinks: u64,
    total_recovers: u64,
    /// 短差腿 per-level realized 累计（recover/drain/强平的空头腿都计入）。
    short_pnl_by_level: Vec<f64>,
    /// 本 campaign 起点的 sink/recover/pnl 基线（campaign_start 设，campaign_end 比较）。
    camp_sink_base: u64,
    camp_recover_base: u64,
    camp_pnl_base: Vec<f64>,
    /// 是否在 campaign 中（防 flip 的 clear_all+enter 双重 campaign_end）。
    in_campaign: bool,

    // ── ① prove_bsp_triggers_operation（panic 守卫；ops_by_trigger 为归因观测）──
    /// 主动操作按触发源计数 [Bsp, Emergence, Eod, (Liq 预留=0，强平不经原子函数)]。
    pub ops_by_trigger: [u64; 4],
    /// 无触发源操作计数（panic 守卫下恒=0；保留为冗余验收读数）。
    pub n_ops_without_trigger: u64,

    // ── ② prove_direction_matches_trend（观测）──
    /// 有 emergent_top 的 bar 数（分母）。
    pub n_dir_checks: u64,
    /// 核心方向 ≠ 最高走势方向 的 bar 数（核心被低级别 BSP flip / 逆涌现未升级）。
    pub n_dir_mismatch: u64,

    // ── ③ prove_sink_recover_balance（观测，campaign 边界）──
    /// 已检查的 campaign 数。
    pub n_campaigns_checked: u64,
    /// 本 campaign 内 sink≠recover 的 campaign 数。
    pub n_sink_recover_imbalance: u64,
    /// 累计 (sink−recover) 净失衡（>0 = 空头累积未配对回补）。
    pub sink_recover_imbalance_total: i64,

    // ── ④ prove_per_level_pnl（观测，campaign 边界）──
    /// 本 campaign 内存在级别短差 pnl<0 的 campaign 数。
    pub n_neg_pnl_campaigns: u64,
    /// per-level 短差 pnl 在某 campaign 内为负的次数（哪个级别 BSP 消费有问题）。
    pub neg_pnl_by_level: Vec<u64>,
}

impl ProveGuards {
    pub fn new(n_levels: usize) -> Self {
        ProveGuards {
            n_levels,
            cur_trigger: OpTrigger::None,
            total_sinks: 0,
            total_recovers: 0,
            short_pnl_by_level: vec![0.0; n_levels],
            camp_sink_base: 0,
            camp_recover_base: 0,
            camp_pnl_base: vec![0.0; n_levels],
            in_campaign: false,
            ops_by_trigger: [0; 4],
            n_ops_without_trigger: 0,
            n_dir_checks: 0,
            n_dir_mismatch: 0,
            n_campaigns_checked: 0,
            n_sink_recover_imbalance: 0,
            sink_recover_imbalance_total: 0,
            n_neg_pnl_campaigns: 0,
            neg_pnl_by_level: vec![0; n_levels],
        }
    }

    // ──────────────── ① prove_bsp_triggers_operation ────────────────

    /// 设定本执行段触发源（on_bar 起始/emergence/route_bsp/finish 各段开头调）。
    pub fn set_trigger(&mut self, t: OpTrigger) {
        self.cur_trigger = t;
    }

    /// **prove_bsp_triggers_operation**（panic 守卫）：每个主动操作必有触发源。
    ///
    /// 架构上 enter/sink/recover/drain/ascend/clear_all 只能由 BSP 路由、涌现升级或收尾触发。
    /// `cur_trigger==None` ⇒ 走势跟随残留（无信号却有操作）⇒ panic 停下来。
    pub fn note_op(&mut self, op: &str) {
        match self.cur_trigger {
            OpTrigger::None => {
                self.n_ops_without_trigger += 1;
                panic!("走势跟随残留：主动操作 `{op}` 无 BSP/emergence/eod 触发源（cur_trigger=None）");
            }
            OpTrigger::Bsp => self.ops_by_trigger[0] += 1,
            OpTrigger::Emergence => self.ops_by_trigger[1] += 1,
            OpTrigger::Eod => self.ops_by_trigger[2] += 1,
        }
    }

    // ──────────────── ② prove_direction_matches_trend ────────────────

    /// **prove_direction_matches_trend**（观测）：核心方向应 = 最高走势类型方向。
    ///
    /// 核心仓骑最高走势 ⇒ 方向必匹配。不匹配 = emergence_upgrade 未工作（逆涌现方向未升级）或
    /// 被后续低级别 BSP flip 破坏。`core_dir=None`（全空）不计 mismatch（无核心可比）。
    pub fn check_direction(&mut self, core_dir: Option<Polarity>, emergent_dir: Polarity) {
        self.n_dir_checks += 1;
        if let Some(d) = core_dir {
            if d != emergent_dir {
                self.n_dir_mismatch += 1;
            }
        }
    }

    // ──────────────── ③/④ sink/recover/pnl 累计 + campaign 边界 ────────────────

    /// sink 发生（计入 campaign sink 总数）。
    pub fn on_sink(&mut self) {
        self.total_sinks += 1;
    }

    /// recover 发生（计入 campaign recover 总数 + 短差腿 per-level pnl）。
    pub fn on_recover(&mut self, level: usize, short_realized: f64) {
        self.total_recovers += 1;
        if level < self.n_levels {
            self.short_pnl_by_level[level] += short_realized;
        }
    }

    /// 短差腿平仓 realized（drain 的同向遗留 / 强平的空头腿）计入 per-level pnl（非 recover，不计
    /// recover 总数）。仅当被平层是短差腿（Short）时调用。
    pub fn on_short_pnl(&mut self, level: usize, realized: f64) {
        if level < self.n_levels {
            self.short_pnl_by_level[level] += realized;
        }
    }

    /// campaign 起点（enter 末尾）：快照 sink/recover/pnl 基线。
    pub fn campaign_start(&mut self) {
        self.camp_sink_base = self.total_sinks;
        self.camp_recover_base = self.total_recovers;
        self.camp_pnl_base.copy_from_slice(&self.short_pnl_by_level);
        self.in_campaign = true;
    }

    /// campaign 终点（reset_campaign，由 flip/eod 的 clear_all 触发）：检查两个 campaign 级不变量。
    ///
    /// - **prove_sink_recover_balance**：本 campaign sink 总数 == recover 总数（走势终完美 ⇒ 每个
    ///   卖点开的短差必有买点回补；sink>recover = 空头累积，被 clear/强平而非 recover 平掉）。
    /// - **prove_per_level_pnl**：本 campaign 每级别短差 pnl 增量 ≥ 0（每级别 = 带通滤波器，应独立
    ///   正贡献；持续负 = 该级别 BSP 消费有问题）。
    ///
    /// 两者皆观测计数（已知 BTC 等强牛违反）。`in_campaign=false`（无实际 campaign）跳过。
    pub fn campaign_end(&mut self) {
        if !self.in_campaign {
            return;
        }
        self.n_campaigns_checked += 1;
        // ③ sink/recover 平衡
        let sink_d = self.total_sinks - self.camp_sink_base;
        let rec_d = self.total_recovers - self.camp_recover_base;
        if sink_d != rec_d {
            self.n_sink_recover_imbalance += 1;
            self.sink_recover_imbalance_total += sink_d as i64 - rec_d as i64;
        }
        // ④ per-level 短差 pnl
        let mut any_neg = false;
        for l in 0..self.n_levels {
            let d = self.short_pnl_by_level[l] - self.camp_pnl_base[l];
            if d < -1e-6 {
                self.neg_pnl_by_level[l] += 1;
                any_neg = true;
            }
        }
        if any_neg {
            self.n_neg_pnl_campaigns += 1;
        }
        self.in_campaign = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bsp_trigger_归因() {
        let mut g = ProveGuards::new(8);
        g.set_trigger(OpTrigger::Bsp);
        g.note_op("sink");
        g.set_trigger(OpTrigger::Emergence);
        g.note_op("ascend");
        g.set_trigger(OpTrigger::Eod);
        g.note_op("clear_all");
        assert_eq!(g.ops_by_trigger, [1, 1, 1, 0]);
        assert_eq!(g.n_ops_without_trigger, 0);
    }

    #[test]
    #[should_panic(expected = "走势跟随残留")]
    fn 无触发源操作_panic() {
        let mut g = ProveGuards::new(8);
        // 未 set_trigger（None 态）下执行操作 = 残留 → panic。
        g.note_op("sink");
    }

    #[test]
    fn sink_recover_平衡_配对() {
        let mut g = ProveGuards::new(8);
        g.campaign_start();
        g.on_sink();
        g.on_recover(2, 5.0);
        g.campaign_end();
        assert_eq!(g.n_sink_recover_imbalance, 0, "1 sink ↔ 1 recover 平衡");
        assert_eq!(g.n_campaigns_checked, 1);
    }

    #[test]
    fn sink_recover_失衡_空头累积() {
        let mut g = ProveGuards::new(8);
        g.campaign_start();
        g.on_sink();
        g.on_sink(); // 2 sink
        g.on_recover(2, 5.0); // 1 recover
        g.campaign_end();
        assert_eq!(g.n_sink_recover_imbalance, 1);
        assert_eq!(g.sink_recover_imbalance_total, 1, "净 1 个空头未配对");
    }

    #[test]
    fn per_level_pnl_负失血() {
        let mut g = ProveGuards::new(8);
        g.campaign_start();
        g.on_recover(3, -10.0); // level 3 短差亏损
        g.campaign_end();
        assert_eq!(g.n_neg_pnl_campaigns, 1);
        assert_eq!(g.neg_pnl_by_level[3], 1);
    }

    #[test]
    fn direction_match_失配计数() {
        let mut g = ProveGuards::new(8);
        g.check_direction(Some(Polarity::Long), Polarity::Long); // 匹配
        g.check_direction(Some(Polarity::Short), Polarity::Long); // 失配
        g.check_direction(None, Polarity::Long); // 全空，不计
        assert_eq!(g.n_dir_checks, 3);
        assert_eq!(g.n_dir_mismatch, 1);
    }
}
