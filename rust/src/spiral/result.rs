//! 螺旋引擎运行结果（trade 行 + 守恒/观测计数器）。
//!
//! GUARD-ROLE: legacy-generation-loadbearing-for-fugue-v3——名分：现役（详见
//! `spiral/mod.rs` 头部 GUARD-ROLE 块，#762 C7-E3 核定）。
//!
//! 设计来源：架构 §7（prove 观测面）+ §9.3（trade11 契约）。复用
//! `trading::positional::LayerTrade` 作 trade 行（trade11 marshal 契约固定）。
//!
//! ## 计数器的认识论位置（formalization-validity-domain）
//! `*_by_ladder` 计数器是引擎纯函数的**结构化观测产出**（不经 eprintln 副作用），
//! 供下游 prove 守卫（T50/T56/T58/T59 eod 观测）+ L2/L3 分析消费。

use crate::trading::positional::LayerTrade;
use crate::trading::types::MAX_LADDER;

/// 螺旋引擎运行结果（批量/流式共享，bit-exact 由构造保证）。
#[derive(Debug, Clone, Default)]
pub struct SpiralResult {
    /// trade 行（trade11 契约：开仓时空、平仓/翻转时配对）。
    pub trades: Vec<LayerTrade>,
    /// (bar, nav) 采样（周期点 + 末 bar）。
    pub equity: Vec<(i64, f64)>,
    /// 末态自由现金（= 全森林 eod cascade 关闭后的 NAV）。
    pub final_nav: f64,

    // ── 操作计数（F/C/D/E/A）──
    /// F 根入场数（按入场层 = 区间套链顶 source）。
    pub n_root_entries_by_ladder: [u64; MAX_LADDER],
    /// 入场数（含 F 根 + E spawn 子，按层）。
    pub n_entries_by_ladder: [u64; MAX_LADDER],
    /// 平仓相数（settle 调用，按层）。
    pub n_exits_by_ladder: [u64; MAX_LADDER],
    /// C 根 τ 翻转数（按翻转层）。
    pub n_root_flips_by_ladder: [u64; MAX_LADDER],
    /// E 降成本 spawn 数（按子层 = parent−1）。
    pub n_spawns_by_ladder: [u64; MAX_LADDER],
    /// 级联回收数（父平仓 ⇒ 子树前提消失，按被回收子层）。
    pub n_cascade_closes_by_ladder: [u64; MAX_LADDER],
    /// A 强平数（按空头 voice 层）。
    pub n_liquidations_by_ladder: [u64; MAX_LADDER],

    // ── N4 成本门（eod 反证 floor_stops 恒 0）──
    /// θ<K×friction 拒 spawn 数（势幅度<成本，按子层）。
    pub n_cost_rejects_by_ladder: [u64; MAX_LADDER],
    /// θ 无参照拒 spawn 数（warm-up/势不可测，按子层）。
    pub n_noref_rejects_by_ladder: [u64; MAX_LADDER],
    /// 固定 floor 终止数（**恒 0**——纯成本门，无 floor 参数；prove_n4 反证）。
    pub n_floor_stops_by_ladder: [u64; MAX_LADDER],

    // ── 信号层 nest 窗口（T56/T58 arm/fire 生命周期）──
    /// candidate 武装数（角向圈起始，按层；T58 arm 起点）。
    pub n_arms_by_ladder: [u64; MAX_LADDER],
    /// 卖侧向心 confirm fire 数（角向圈 φ=0 闭合，按层；T56 覆盖）。
    pub n_fire_sell_by_ladder: [u64; MAX_LADDER],
    /// 买侧向心 confirm fire 数（镜像）。
    pub n_fire_buy_by_ladder: [u64; MAX_LADDER],
    /// 破极值否定数（027:25，按层）。
    pub n_breaks_by_ladder: [u64; MAX_LADDER],

    // ── 会计重定基（A4 N_base 双向）──
    /// earning 累计增仓单位（cost_pool≤0 后纯利润买点买入 Δ，N+Δ）。
    pub earning_units: f64,
    /// 亏损回补缩水累计单位（买不回的单位=亏损物理形式，N−δ）。
    pub shrink_units: f64,
    /// 空头腿已实现净现金（按层）。
    pub short_net_cash_by_ladder: [f64; MAX_LADDER],

    // ── H¹ 闭合（Δr=−1 兑现）──
    /// CrossLevel 闭合数（E spawn / D 子层，每次 Δr=−1）。
    pub cross_level_closures: u64,

    // ── eod 观测（T50/T56–T59 + 森林规模 + 物理暴露）──
    /// 森林最大活跃子数（>1 = 森林实证，栈不可能）。
    pub max_children: u64,
    /// T50 操作频率径向标度律：高层 fire 多于相邻低层的局部单调违反层数。
    pub t50_monotone_violations: u64,
    /// T56 角径全纯 `h²³=σ`：confirm fire 覆盖的径向层数。
    pub t56_radial_coverage: u64,
    /// T57 手性镜像 `τhτ⁻¹=h⁻¹`（~regime 观测）：单边 fire 退化层数。
    pub t57_onesided_layers: u64,
    /// T58 角向基本域（23 环）：arm→fire 生命周期活跃的径向层数。
    pub t58_active_levels: u64,
    /// T59 尺度不变 σ 自相似（~观测）：有 arm 无 fire 的退化层数。
    pub t59_degenerate_layers: u64,
    /// 物理多头在手 bar 数。
    pub phys_long_bars: u64,
    /// 物理空头在手 bar 数。
    pub phys_short_bars: u64,
    /// 各层视图持有 bar 数。
    pub held_bars_by_ladder: [u64; MAX_LADDER],
    /// 各层持空 bar 数。
    pub short_held_bars_by_ladder: [u64; MAX_LADDER],
}

impl SpiralResult {
    /// trade 行数（流式 push_bar 切出本 bar 新增）。
    pub fn n_trades(&self) -> usize {
        self.trades.len()
    }
}
