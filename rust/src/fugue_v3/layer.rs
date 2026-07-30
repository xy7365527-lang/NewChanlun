//! 层结构数据：`Layer`（每级别一个净仓位，五字段）+ `FugueResult`（报告契约）。
//!
//! 设计来源：用户裁决 2026-06-17 —— Layer **无 CyclePhase**（CyclePhase=硬编码四步相位，被否定）。
//! direction 是 D∞ 坐标 ε 的投影（τ 翻转之），units 是系数模 M（σ-不变 Casimir T48），basis/
//! entry_bar 是会计投影所需（P&L 锚 + trade 行）。一个级别一个方向（相邻级别方向相反 ⟹ 手性
//! 交替无对冲净额）。**无 core/mobile 硬拆分**——核心仓 H⁰ 是涌现（顶层稀疏下沉的多数）。
//!
//! ## 认识论等级
//! 全文件 **L0**（纯数据定义）。

use crate::trading::positional::LayerTrade;
use crate::trading::types::{Polarity, MAX_LADDER};

/// 一个级别的账户（层结构基本单元，五字段）。
///
/// - `ladder`：级别 r（= 数组下标，单一真相源）。
/// - `direction`：当前方向 ε（Long/Short）——D∞ 坐标 ε 的投影，τ 翻转之。
/// - `units`：当前仓位 |units| ≥ 0（系数模 M，σ-不变 Casimir T48）；direction 携带符号。
/// - `basis`：加权入场价（短差 P&L 锚 + 1x 强平基准）。units==0 时为 NaN。
/// - `entry_bar`：当前 chunk 入场 bar（trade 行锚）。units==0 时为 −1。
#[derive(Debug, Clone, Copy)]
pub struct Layer {
    pub ladder: usize,
    pub direction: Polarity,
    pub units: f64,
    pub basis: f64,
    pub entry_bar: i64,
}

impl Layer {
    /// 构造空闲层（无仓位，Long 占位）。
    pub fn idle(ladder: usize) -> Self {
        Layer {
            ladder,
            direction: Polarity::Long,
            units: 0.0,
            basis: f64::NAN,
            entry_bar: -1,
        }
    }

    /// 是否占用（有仓位）。
    pub fn is_active(&self) -> bool {
        self.units > 1e-12
    }
}

// ════════════════════════════ 运行结果（报告契约，ffi 消费）════════════════════════════

/// 赋格引擎 v3 运行结果（trade 行 + 守恒/观测计数器）。批量/流式共享。
///
/// `*_by_ladder` 计数器是引擎纯函数的结构化观测产出（不经 eprintln），供下游分析。
#[derive(Debug, Clone, Default)]
pub struct FugueResult {
    /// trade 行（trade11 契约，复用 `LayerTrade`）。
    pub trades: Vec<LayerTrade>,
    /// **与 `trades` 平行的腿出生途径**（"entry"/"flip"/"sink"）。`trades[i]` 是某条腿的一次
    /// 平仓事件，`trade_origins[i]` 标注该腿当初**如何建立**（途径 a 翻空核心 = "flip" /
    /// 途径 b 次级别 sink 子腿 = "sink" / 入场建仓 = "entry"）。翻转版的 `exit_reason` 不能
    /// 区分途径（flip 既平翻空核心也级联平子腿；reduce 既减核心也减子腿），出生途径必须显式
    /// 标注。仅 T 翻转引擎（`TPositionEngine`）填充；其他引擎留空 vec（长度 0 ⟹ dump fallback "?"）。
    pub trade_origins: Vec<&'static str>,
    /// (bar, nav) 采样（周期点 + 末 bar）。
    pub equity: Vec<(i64, f64)>,
    /// (bar, long_units, short_units) 真实净敞口采样（与 equity 同采样点）。trade 行反推
    /// 净敞口在 units 循环复用（add_at 回补保留 entry_bar）下会虚增量级——此为唯一真值源。
    pub exposure_series: Vec<(i64, f64, f64)>,
    /// 末态 NAV（全清后的现金）。
    pub final_nav: f64,

    // ── 操作计数 ──
    /// F 根入场数（按入场层 = 区间套链顶 source）。
    pub n_entries_by_ladder: [u64; MAX_LADDER],
    /// C 核心仓顶背驰清仓数（按清仓层）。
    pub n_core_clears_by_ladder: [u64; MAX_LADDER],
    /// τ sink 数（σ⁻¹∘τ 下沉，按层）。
    pub n_cycle_opens_by_ladder: [u64; MAX_LADDER],
    /// τ recover 数（σ∘τ 升回，按层；= 完整 sink→recover 闭合计数）。
    pub n_cycle_closes_by_ladder: [u64; MAX_LADDER],
    /// 空头层强平数（按层；1x 逐仓 c≥2×basis）。
    pub n_liquidations_by_ladder: [u64; MAX_LADDER],
    /// σ-不变配额成本门拒绝数（θ<K×friction，按子层）。
    pub n_cost_rejects_by_ladder: [u64; MAX_LADDER],
    /// θ 无参照拒绝数（warm-up/势不可测，按子层）。
    pub n_noref_rejects_by_ladder: [u64; MAX_LADDER],

    // ── 信号层 nest 窗口（向心 confirm 生命周期，复用 spiral 信号层填充）──
    /// candidate 武装数（按层）。
    pub n_arms_by_ladder: [u64; MAX_LADDER],
    /// 卖侧向心 confirm fire 数（按层）。
    pub n_fire_sell_by_ladder: [u64; MAX_LADDER],
    /// 买侧向心 confirm fire 数（按层）。
    pub n_fire_buy_by_ladder: [u64; MAX_LADDER],
    /// 破极值否定数（027:25，按层）。
    pub n_breaks_by_ladder: [u64; MAX_LADDER],

    // ── 短差 P&L 观测 ──
    /// 各层短差累计已实现现金（穿 ε=−1 τ 循环的净降成本，按层）。
    pub mobile_realized_pnl_by_ladder: [f64; MAX_LADDER],

    // ── 持仓三阶段观测（T 引擎填充；其他引擎留 0，见 `recursive_t::t_engine::TStage`）──
    /// 退本金触发次数（cost_basis 穿零、本金全额收回的 campaign 数）。
    pub n_capital_recovered: u64,
    /// 累计退出在险池的本金（"放到安全的地方"，立于不败之地）。
    pub total_withdrawn: f64,
    /// 增股数部署次数（EarningShares 阶段用纯利润买回更多 units 的次数）。
    pub n_earning_deploys: u64,
    /// 诊断：降成本进度峰值 ×1000 = max((entry_cost − core_cost_basis)/entry_cost)（=1000 ⇒ 成本曾归0
    /// 可退本金；<1000 ⇒ 核心 reduce 降成本不及本金——编排者 2026-06-20 裁决后）。
    pub max_core_gain_x1000: u64,
    /// 诊断：campaign 重置次数（翻转/清仓，每次清零 core_cost_basis）——campaign 碎片化指标。
    pub n_campaign_resets: u64,
    /// 短差腿（Short reduce：做空→平空）累计已实现 pnl——**单独核算，不入降成本**（编排者裁决点5）。
    pub short_leg_pnl: f64,
    /// 增股数累计加的 units（EarningShares deploy_earning Σq）——量化增股数对股数的实际效果（任务3）。
    pub earning_units_added: f64,
    /// 增股数累计部署的现金（Σq·c）——增股数贡献 ≈ earning_units_added×末价 − 此（vs 留现金 idle）。
    pub earning_cash_deployed: f64,

    // ── 闭合 + 多声部观测 ──
    /// CrossLevel 闭合数（每次 τ 转移 Δr=−1）。
    pub cross_level_closures: u64,
    /// 自下而上涌现升级数（emergence-upgrade）：核心仓随 T level 涌现 relabel 升级归属（NAV 中性）。
    pub n_emergence_upgrades: u64,
    /// 最大并发活跃空头声部数（多声部 L6 实证）。
    pub max_concurrent_voices: u64,
    /// 最大并发相邻同向占用级别对数（T24 观测：建仓阶段合法；非建仓 regime 高值提示 word 路由 bug）。
    pub max_chiral_same_dir: u64,
    /// 物理多头在场 bar 数。
    pub phys_long_bars: u64,
    /// 物理空头在场 bar 数。
    pub phys_short_bars: u64,
    /// 各层空头做空持有 bar 数。
    pub short_held_bars_by_ladder: [u64; MAX_LADDER],
}

impl FugueResult {
    /// trade 行数（流式 push_bar 切出本 bar 新增）。
    pub fn n_trades(&self) -> usize {
        self.trades.len()
    }
}
