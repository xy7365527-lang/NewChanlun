//! Θ_level + Θ_signal 子模块（reference-theta-v0.md:27-37）。
//!
//! ## 契约重锚（legacy Strict/* → Origin canonical，task #127 A′ Phase2）
//!
//! 递归级别构造 + R6 态 + 买卖点 bit-vector + 背驰度量 + 区间套。给定 Θ_level/Θ_signal ⟹ R6 态 +
//! BSP 证书唯一。契约锚点从 legacy `Strict/{LevelState,Center,BSP,Trend,Nest,Recursive}.lean`
//! **重锚到 Origin canonical**：`Origin.RecursiveLevelSystem` / `Origin.CenterStates` /
//! `Origin.BspClassification` / `Origin.TrendCompleteClassification` / `Origin.SubLevelDescent`。
//!
//! ## 子模块拓扑（各子模块契约锚 Origin def）
//!
//! - [`center`]：完整中枢判据（方向交替+第三段贯穿）+ 关系/位置三态。对齐
//!   `Origin.CenterComplete.CenterConfirmedComplete` / `Origin.CenterConstruction.centersOf` /
//!   `Origin.CenterStates.{classifyDevelopment,classifyPosition}`。
//! - [`level`]：递归级别走势裁决（`classifyMove` 全链同向）。对齐
//!   `Origin.TrendCompleteClassification.{TrendClass,chooseTrend}` + `Origin.RecursiveLevelSystem`。
//! - [`level_state`]：R6 位置态 + LevelState 三元组。对齐 `Origin.CenterStates.CenterPosition` +
//!   `Origin.RecursiveLevelSystem`。
//! - [`bsp`]：买卖点 bit-vector 判据（三类结构谓词，非互斥）。对齐
//!   `Origin.BspClassification.{BspEndpoint,IsType1,IsType2,IsType3Buy,IsType3Sell}`。
//! - [`divergence`]：背驰 MACD 度量（浮点域隔离 + 同向段面积严格变小）。对齐
//!   `Origin.Divergence.{Force,IsDivergence}` + `Origin.ForceInterface.ForceMeasure`（reference:37）。
//! - [`nest`]：区间套有限递归证书 χ（`Sel_Θ` 选择器 + 终端确认）。对齐
//!   `Origin.SubLevelDescent.{descend,subLevelHasBrokenCenter}`。
//! - [`turn_class`]：小转大显式分类分支（旁挂联合分类 `NestTurnClass` 四类 partition，
//!   纯只读派生）。p118 施工图形态 D；Lean 侧 `Origin.NestTurnClass` 列 formal-chain 遗留
//!   （rust 领先 Origin，T3 L1 先例登记漂移）。
//!
//! ## 递归级别（reference-theta-v0.md:29-30；契约锚 `Origin.RecursiveLevelSystem`）
//!
//! `L0=1分钟线段账本`（parser segments）；`L(k+1)` 只由 `Lk` 已完成走势/Move 构造（对齐
//! `Origin.RecursiveLevelSystem.lift` + `chanRecursiveLevelSystem` 的 `composeStep`：Lk 走势单元 →
//! 连续三段窗口中枢 → 中枢序列裁决走势 → L(k+1) 输入单元）。禁跳级混级。
//! 某层无 ≥`config.level.min_parts_per_level` 完成部件则自然终止；上界 `config.level.l_max`。
//!
//! ## 认识论（formalization-validity-domain）
//!
//! 实装本身 = L1（bit-exact 一致性：Rust 与 Lean spec 输出对齐 = 验证管线正确，不验证
//! Θ 在市场上有效）。各子模块的判定函数是 Lean 纯函数的镜像（L0 给定 Θ 后）；golden/
//! property 测试是 L1（管线正确性，零信息增量）。L2/L3 有效域检验是 Phase 3-4，本模块不声称。
//!
//! ## 铁律（编排者硬指令）
//!
//! 只实装已冻结 Θ v0；遇 spec 漏洞/与 Lean 冲突 → change request，不静默改语义。
//! 不可变：构造新对象，不原地修改。不可交易/退化情况显式处理不静默吞。

use super::config::ThetaConfig;
use super::parser::ParseLayer;
use super::types::{Center, Direction, Segment, Tick};
use divergence::MacdState;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub mod center;
pub mod ref_v1;
pub mod decompose;
pub mod level_state;
pub mod bsp;
pub mod divergence;
pub mod force_conformance;
pub mod descend;
pub mod rmove_compose;
pub mod recursive_tower;
pub mod nest;
/// 账本内核（票 #573 T1）：per-key 注册/首建/append-only 修订/倒退拒绝/终态吸收/钟首写/
/// 增量返回/身份迁移/只读枚举/不变量骨架的对象无关泛型承载体（四组类型参数）。
pub mod ledger_kernel;
/// V3 活假设状态机：NestLifecycleBook sidecar 注册表（三态 + 五钟；#231 重建，spec #232）。
pub mod nest_lifecycle;
/// #92/#93 证书索引：确认事件 → typed 证书（身份主键；构建口径 B + CWindow）。
pub mod nest_index;
/// p118 关④ 小转大显式分类分支：旁挂联合分类 `NestTurnClass`（四类 partition，纯只读派生）。
pub mod turn_class;
pub use turn_class::{
    classify_certificate_turn, classify_nest_turns, is_defer_orphan_event, CertKey, NestTurnClass,
    XzdEvidence,
};
pub mod signal;
/// #110 投影层骨架 + 级别身份标签（SPEC #109 expand 第一票）。默认门关零开销。
pub mod projection;
pub mod six_state;
pub mod voice_eat;
pub mod cand_predicate;
/// C2 走势消费 seam：显式 exact-three 投影、D3 方向绑定与 D2 A/C provider。
pub mod level_view;
/// D7 firstRetrace 只读复核：严格 CompletedMove pair 映射与对象重启事件语义。
/// 不接生产订单路径；只消费 C2 view，默认关闭的 seam 不受影响。
pub mod first_retrace_replay;
/// 买卖点身份账本 S1（票 #621，#465 裁定 A 之 T3 首环）：观察适配器 → 三态状态机 →
/// append-only 修订日志（JSONL 外化 + 重放折叠恢复）→ 成立档门户；全部经 [`ledger_kernel`] 表达。
pub mod retrace_ledger;
/// C2 CompletedFreeze 的正式 append-only event-store adapter。
pub mod level_view_store;

/// #345：自持缓冲区增量分类器变体（Nautilus 流式适配，无条件编译——见模块头）。
pub mod streaming;

/// P52 全量增量重放专用的 frontier 只读计数器。
///
/// 默认关闭；只有诊断 bin 显式 [`enable`] 后，分类器在既有 pop/recompose 与 dirty 依赖门处
/// 累加旁路计数。计数不参与任何分类、交易、订单或风控分支。
pub mod cp_replay_diagnostics;
/// 阶段计时插桩（profile-only，#106 真热点定位）：thread_local 累加器，
/// env `THETA_PROFILE_STAGES=1` 时启用。只测时间不改逻辑（bit-exact 安全）。
pub mod stage_profile;
/// ★A3 证书 oracle 探针（仅 test 构建）：记录证书热路径分支命中，供 always-run oracle 断言
/// 「fixture 确实触发了 had_emitted_window pop（T==1/T>1）与两处早停缓存血缘失效」——防止
/// 「always-run 但覆盖为零」的陷阱（codex 审计第6条根修）。release/非 test 构建完全不编译。
#[cfg(test)]
pub mod oracle_probe;

use bsp::BspPoint;
use center::UnitRange;
use decompose::{decompose, decompose_resume, MoveBlock};
use recursive_tower::{
    compose_level, descend_leveled, index_of_in, map_src_to_close_idx, project_to_units,
    CpScanOwnership, ElementId, LeveledMove, WinMeta,
};
use super::types::Side;

/// Θ_level + Θ_signal 分类主管线（级别态 / 分类输出 / 单元⇄线段规约 / 中枢扫描 / 递归构造）。
mod pipeline;
pub use pipeline::{classify, classify_with_tower, Classification, LevelState};
/// P1/P52/P53 Cand^δ 只读驱动器（纯增量只读层，不改 `classify` 行为）。
mod cand_delta;
use pipeline::unit_to_segment;
pub use cand_delta::{
    cand_delta_entry_tower, cand_delta_tower, cand_delta_tower_cached, cp_recall_upper_bound_audit,
};

/// 增量塔缓存与 MACD 增量递推（task #93：per-bar substrate 塔构造 O(n²) → O(n)）——
/// [`TowerCache`] / `LevelCache` / area-memo / closes·MACD 增量序列。见模块头有效域声明。
mod tower_cache;
pub use tower_cache::TowerCache;
use recursive_tower::{compose_level_resume, WindowScanCursor};

/// 增量塔入口（task #93）：塔构造中枢扫描走增量 resume，bit-exact 等价 [`classify_with_tower`]。
mod incremental;
pub use incremental::classify_with_tower_incremental;
/// 递归组装层第二类（B2/S2）提取与次级别背驰力度（§10.2 买卖点定律一）。
mod sublevel;


#[cfg(test)]
mod tests {
    use super::super::types::MoveKind;
    use super::cand_delta::cache_series_ok;
    use super::pipeline::segment_to_unit;
    use super::tower_cache::{compute_macd_hist_incremental, update_closes_cache};
    use super::pipeline::{classify_level, extract_first_third_for_level};
    use super::*;
    use super::super::parser::ParseLayer;
    use super::super::types::Direction;

    #[test]
    fn p52_frontier_diagnostics_are_level_scoped_and_resettable() {
        cp_replay_diagnostics::enable();
        cp_replay_diagnostics::record_dirty_invalidation(2, 3, 5);
        cp_replay_diagnostics::record_tail_reinherit(2);
        cp_replay_diagnostics::record_tail_reinherit(1);

        let counters = cp_replay_diagnostics::snapshot();
        assert_eq!(counters[1].tail_reinherits, 1);
        assert_eq!(counters[2].pending_fallbacks, 3);
        assert_eq!(counters[2].tail_reinherits, 1);
        assert_eq!(counters[2].certificate_clear_recomputes, 5);

        cp_replay_diagnostics::disable();
        assert!(cp_replay_diagnostics::snapshot().is_empty());
    }

    fn seg(dir: Direction, si: usize, ei: usize, sp: i64, ep: i64) -> Segment {
        Segment { direction: dir, start_index: si, end_index: ei, start_price: sp, end_price: ep }
    }

    /// ★#613（收 #609 F2）：段账本回缩 bar 上 `l0_units()` 与 `tower[0]` 必须仍同长。
    ///
    /// 回归的是这条真 bug：回缩检测的 `cache.clear()` 曾位于 `l0_units_cache` 构建**之后**，
    /// 于是该 bar 上访问器返回空而 `tower[0]` 满载（BTC 100k @ as_of=71040：0 vs 547）。
    /// 消费方（`p123_fast_replay` 的 L2 活窗派生）拿空切片重扫，只在 `resume_from > 0` 时被
    /// 越界守卫恰好接住；`resume_from == 0` 时会静默落 `no_window_formed`。
    #[test]
    fn l0_units_stays_in_sync_with_tower0_on_segment_ledger_shrink() {
        let cfg = ThetaConfig::default();
        let mut cache = TowerCache::new();

        // 第一 bar：6 段账本（confirmed 前缀 5，末段未确认——古怪线段可重划）。
        let long_segments = vec![
            seg(Direction::Up, 0, 4, 100, 150),
            seg(Direction::Down, 4, 8, 150, 120),
            seg(Direction::Up, 8, 12, 120, 148),
            seg(Direction::Down, 12, 16, 148, 110),
            seg(Direction::Up, 16, 20, 110, 145),
            seg(Direction::Down, 20, 24, 145, 115),
        ];
        let closes: Vec<i64> = (0..28).map(|i| 100 + if i % 2 == 0 { 20 } else { -20 }).collect();
        let long_layer = ParseLayer {
            segments: Rc::new(long_segments.clone()),
            segments_confirmed_len: 5,
            merged_bars: Rc::new(bars_from_closes(&closes)),
            ..Default::default()
        };
        let (_, tower_long) = classify_with_tower_incremental(&long_layer, &cfg, &mut cache);
        assert_eq!(
            cache.l0_units().len(),
            tower_long[0].len(),
            "非回缩 bar 本就同长"
        );

        // 第二 bar：段账本**回缩**到 4 段（末两段被重划吞并）⟹ 走 `cache.clear()` 分支。
        let short_layer = ParseLayer {
            segments: Rc::new(long_segments[..4].to_vec()),
            segments_confirmed_len: 3,
            merged_bars: Rc::new(bars_from_closes(&closes)),
            ..Default::default()
        };
        let (_, tower_short) = classify_with_tower_incremental(&short_layer, &cfg, &mut cache);

        assert!(!tower_short.is_empty(), "4 段仍足以产出 L0 塔快照");
        assert_eq!(
            tower_short[0].len(),
            4,
            "回缩后 tower[0] = 新段账本全量重建"
        );
        assert_eq!(
            cache.l0_units().len(),
            tower_short[0].len(),
            "★F2 不变式：回缩 bar 上 l0_units() 不得为空/失步（#613 收 #609 F2）"
        );
        // 同长之外再钉同源：逐元素等于新段账本的 `segment_to_unit` 投影。
        let expected: Vec<UnitRange> = short_layer.segments.iter().map(segment_to_unit).collect();
        assert_eq!(cache.l0_units(), expected.as_slice(), "同序同源，非仅同长");
    }

    /// 构造 merged_bars：source_index 连续 0..n，close = vals（MACD 背驰真算用）。
    fn bars_from_closes(vals: &[i64]) -> Vec<super::super::types::Bar> {
        vals.iter()
            .enumerate()
            .map(|(i, &v)| super::super::types::Bar {
                source_index: i,
                timestamp: i as i64,
                open: v,
                high: v,
                low: v,
                close: v,
                volume: 1,
                untradable: false,
            })
            .collect()
    }

    /// ★批1（force_state 生产热路由 step4）：TowerCache 的 dif 增量通路 bit-exact 对拍全量。
    ///
    /// 严格路线（Lead 裁定，拒绝「增量恒 None」降级）：`compute_macd_hist_incremental` 逐 bar 产出的
    /// `macd_dif` 必与全量 `compute_macd(&closes[..k]).dif` **逐位相等**（非 tolerance——bit-exact 是
    /// 断言，不是近似；tolerance 会把非 bit-exact 藏进容差 = 声明膨胀）。同证 `macd_hist`（dif/hist
    /// 锁步的锚），并证 `closes_tick == merged_bars.close`（force 价格振幅 proxy 输入的整数往返）。
    ///
    /// 覆盖：resume 增量路径（append-only 前缀稳定，confirmed_len=k-1）逐 bar 生长——每步驱动
    /// truncate+append+tail 三段（含单 bar 分支 k=1）。dif 是 hist 子表达式（hist=dif-dea），hist
    /// 增量已证 bit-exact（tower 测试锁 BspPoint）⟹ dif 同证；本测试直接坐实 dif 数组本身。
    #[test]
    fn incremental_macd_dif_and_closes_tick_bit_exact() {
        let cfg = super::super::config::MacdConfig::default();
        // 合成 closes：上升 + 震荡 + 下降（EMA 充分递推，覆盖 dif 正负峰）。整值 ⟹ closes_tick 往返精确。
        let vals: Vec<i64> = (0..90)
            .map(|i| 1000 + (30.0 * ((i as f64) * 0.3).sin()) as i64 + i as i64)
            .collect();
        let closes: Vec<f64> = vals.iter().map(|&v| v as f64).collect();

        // ── dif/hist 增量 vs 全量（逐 bar 生长，resume 增量路径）──
        let mut cache = TowerCache::new();
        for k in 1..=closes.len() {
            let prefix = &closes[..k];
            let confirmed = k.saturating_sub(1); // append-only：前 k-1 稳定，尾 bar 不稳定。
            compute_macd_hist_incremental(prefix, confirmed, &cfg, &mut cache);
            let full = divergence::compute_macd(prefix, &cfg);
            assert_eq!(cache.macd_dif(), full.dif.as_slice(), "bar {k}: dif 增量 ≠ 全量（bit-exact 破）");
            assert_eq!(cache.macd_hist_for_test(), full.hist.as_slice(), "bar {k}: hist 增量 ≠ 全量");
            assert_eq!(cache.macd_dif().len(), cache.macd_hist_for_test().len(), "dif/hist 锁步等长");
        }

        // ── closes_tick 增量 == merged_bars.close（整数域，force 振幅/速度 proxy 输入）──
        let bars = bars_from_closes(&vals);
        let mut cache2 = TowerCache::new();
        for k in 1..=bars.len() {
            update_closes_cache(&bars[..k], k.saturating_sub(1), &mut cache2);
            let expect: Vec<Tick> = bars[..k].iter().map(|b| b.close).collect();
            assert_eq!(cache2.closes_tick(), expect.as_slice(), "bar {k}: closes_tick ≠ merged_bars.close");
        }
    }

    /// parser BUG-04 回归：缓存守卫按真实覆盖契约校验——`compute_macd_hist_incremental`
    /// 在 n≥2 时 `macd_state_len = n-1`（state 只覆盖稳定前缀，hist/dif 才含不稳定尾 bar），
    /// 旧守卫 `== n` 恒假 ⟹ 增量缓存死代码、每 bar 退化全量 O(n²)。修复后逐 bar 驱动
    /// 生产同源更新（update_closes_cache + compute_macd_hist_incremental），守卫必须命中；
    /// over-invalidate 方向保持（空/不齐 cache 必不命中）。
    #[test]
    fn cand_cache_guard_accepts_incremental_contract() {
        let cfg = super::super::config::MacdConfig::default();
        let vals: Vec<i64> = (0..40).map(|i| 1000 + (i as i64 * 3) % 17).collect();
        let closes: Vec<f64> = vals.iter().map(|&v| v as f64).collect();
        let bars = bars_from_closes(&vals);
        let mut cache = TowerCache::new();
        for k in 1..=bars.len() {
            update_closes_cache(&bars[..k], k.saturating_sub(1), &mut cache);
            compute_macd_hist_incremental(&closes[..k], k.saturating_sub(1), &cfg, &mut cache);
            assert!(
                cache_series_ok(&cache, k),
                "bar {k}: 生产同源增量更新后守卫必须命中（BUG-04：旧 ==n 守卫在 k≥2 恒假）"
            );
        }
        // over-invalidate 方向保持：空 cache 对非空序列必不命中（退化全量，bit-exact）。
        assert!(!cache_series_ok(&TowerCache::new(), 5), "空 cache 必不命中（守卫仍 over-invalidate）");
        assert!(cache_series_ok(&TowerCache::new(), 0), "n=0：空 cache 与空序列自洽（与旧守卫同界）");
    }

    /// ★端到端 B2 真产出（#53 验证门，L1 管线正确性）：升级后的递归塔（`RMove::Compose` 携 subs）
    /// 让 `extract_second_signals` **真接入生产路径**——classify 在真实结构输入上产出 B2 买点。
    ///
    /// 旧塔（`UnitRange` 无 subs）在**任何**输入上产 0 个 B2（descend 得空，结构上不可产）；新塔
    /// 在此输入上产 1 个 B2，坐实升级解除了 still-MISSING-塔。
    ///
    /// 路径（codex 异质裁决确认）：B2 在 **L1→L2 几何路径**产出——3 个同向 L1 走势经几何窗口
    /// （不强制方向交替）compose 成 L2 走势，其 descend 取回的 3 个 L1 走势内识别第二类结构：
    /// L1[1]（i1=1）破 L2 中枢 + MACD 背驰（相对前同向 L1[0]）= 第一类离开；L1[2]（i2=2）回拉不
    /// 创新低 = 第二类回拉走势。B2 端点 = L1[2] 的回拉结束点（坐标由 source_index 侧车真映射）。
    ///
    /// ★诚实边界（still-MISSING-窗口，codex 裁决坐实）：B2/S2 **不在 L0→L1 三段交替窗口产**——
    /// 三段方向交替窗口里可背驰的同向段只在位置 2（末段，无后继回拉），位置 0 无前同向对照，
    /// 位置 1 是唯一异向（无前同向）。这是固定三段封装的结构上界，非接入缺陷（接入逻辑双侧完整）。
    #[test]
    fn end_to_end_second_buy_via_l1_l2_geometric() {
        let cfg = ThetaConfig::default();
        // 9 段 L0：三组（每组 → 一个 L1 走势）。★中枢延伸语义下的诚实重算（PDF §5，task #142）：
        // 组间首段必须与前组**冻结核心 [ZD,ZG]** 不相交（Step3 non-extension），否则整串被 Step2
        // 吸收为 1 个延伸中枢 ⟹ 塔不生长（旧全触及 fixture 的坍缩后果）。推导：
        // - 组A up-down-up：核心 K_A=[max(110,120,120),min(150,150,148)]=[120,148]，外缘 O_A=[110,150]。
        // - 组B down-up-down：首段 [80,115] hi=115 < ZD_A=120 ⟹ non-extension（组间分离）；
        //   核心 K_B=[max(80,80,85),min(115,125,114)]=[85,114]，外缘 O_B=[80,125]。
        // - 组C up-down-up：首段 [115,148] lo=115 > ZG_B=114 ⟹ non-extension；
        //   核心 K_C=[max(115,112,112),min(148,148,147)]=[115,147]，外缘 O_C=[112,148]。
        // L2 核心（几何路径，三 L1 外缘交）= [max(110,80,112), min(150,125,148)] = [112,125] 非空。
        // B2 结构：L1[1].lo=80 < ZD2=112 深破 L2 核心下沿（第一类离开候选，Side::Long）；
        // L1[2] 回拉不创新低（lo=112 >= L1[1].lo=80）；L1[0]/L1[1] 外缘占位方向同 Down
        // （末子 hi < 首子 hi：148<150 / 114<115）⟹ 背驰可配对（closes 前大后小）。
        let segments = vec![
            // 组A（L1[0]）：up-down-up，核心 [120,148]，外缘 [110,150]
            seg(Direction::Up,   0,  4, 110, 150),
            seg(Direction::Down, 4,  8, 150, 120),
            seg(Direction::Up,   8, 12, 120, 148),
            // 组B（L1[1]）：down-up-down，首段 hi=115<ZD_A=120 non-ext，lo=80 深破 L2 核心下沿 112
            seg(Direction::Down,12, 16, 115,  80),
            seg(Direction::Up,  16, 20,  80, 125),
            seg(Direction::Down,20, 24, 114,  85),
            // 组C（L1[2]）：up-down-up，首段 lo=115>ZG_B=114 non-ext，回拉不创新低（lo=112 >= 80）
            seg(Direction::Up,  24, 28, 115, 148),
            seg(Direction::Down,28, 32, 148, 112),
            seg(Direction::Up,  32, 36, 112, 147),
        ];
        // closes 让 L1[1] 区间（source_index [12,24]）MACD 面积 < L1[0] 区间（[0,12]）= 背驰（真算）。
        // 前段大幅波动（面积大），后段小幅（面积小）。
        let mut closes: Vec<i64> = Vec::new();
        for i in 0..12 { closes.push(100 + if i % 2 == 0 { 40 } else { -40 }); } // L1[0] 大幅
        for i in 0..12 { closes.push(100 + if i % 2 == 0 { 5 } else { -5 }); }   // L1[1] 小幅（背驰）
        for i in 0..16 { closes.push(100 + if i % 2 == 0 { 3 } else { -3 }); }   // L1[2] 更小
        let layer = ParseLayer { segments: Rc::new(segments), merged_bars: Rc::new(bars_from_closes(&closes)), ..Default::default() };
        let out = classify(&layer, &cfg);

        // L1 级别（索引 1）含 L2 中枢 + B2（递归组装层产出）。
        assert!(out.levels.len() >= 2, "三组 L0 → L1 走势塔 → L2 中枢，至少 2 级");
        let l1 = &out.levels[1];
        assert_eq!(l1.centers.len(), 1, "3 个 L1 走势 → 1 个 L2 中枢（几何路径）");
        let second_buys: Vec<_> = l1.bsp.iter().filter(|p| p.bits.buy2).collect();
        assert_eq!(second_buys.len(), 1, "★升级后塔真产 B2（旧 UnitRange 塔产 0）");
        let b2 = second_buys[0];
        // B2 端点坐标由 source_index 侧车真映射（回拉走势 L1[2] 的 end_index=36）。
        assert_eq!(b2.source_index, 36, "B2 source_index = 回拉走势 L1[2] 的原始 K 序（坐标侧车真映射）");
        // 第二类止损 = 回拉低点（second_point = 回拉走势 m2.lo）——止损仍 pivot 非 center.zg/zd。
        assert!(b2.pivot_low != 0, "B2 携结构止损价 pivot_low（回拉低点 single source）");
        // ★#218 面 A（spec owner-attribution-fix-20260724 ID-1，机械改写归因：载体形态变化）：
        // 二类点归属载体从判定中枢 c1（次级别中枢，确认层对象）改载该走势一类点身份锚——
        // 第一类离开走势 m1（L1[1]，背驰次级别走势）的终点坐标（区间套：该走势终点极值点 =
        // 一类点）；止损仍 pivot（上条已锁，止损语义不变）。
        assert_eq!(b2.center, Some(signal::OwnerRef::Type1Anchor(24)), "二类点归属载体 = 该走势一类点锚（m1=L1[1] 终点坐标 24）");
        // 互斥语义：B2 端点不置 1/3 类 bit。
        assert!(!b2.bits.buy1 && !b2.bits.buy3, "第二类端点不置 1/3 类 bit");
    }

    /// ★still-MISSING-窗口边界（codex 异质裁决坐实，编码为可执行断言，formalization-validity-domain）：
    /// L0→L1 的三段方向交替窗口**结构上不产 B2/S2**——三段交替里可背驰的同向段只在位置 2（末段，
    /// 无后继回拉），位置 0 无前同向对照，位置 1 是唯一异向（无前同向）。故单个 L1 走势的 3 段 L0
    /// subs 内识别不出「第一类离开（破中枢∧背驰）+ 后继回拉」。
    ///
    /// 此断言锁定边界：L0 级别（索引 0）的 bsp **不含 B2/S2**（B2/S2 由 L1→L2 几何路径产，见
    /// `end_to_end_second_buy_via_l1_l2_geometric`）。这是固定三段封装的结构上界，非补丁——放松
    /// 背驰约束或窗口大小来强产 L0 层 B2 = 声明膨胀（no-patch 禁止）。
    #[test]
    fn l0_level_emits_no_second_class_window_bound() {
        let cfg = ThetaConfig::default();
        // 简单 up-down-up 三段（一个 L0 中枢，一个 L1 走势）——L1 走势 3 段 subs 内无法产 B2/S2。
        let segments = vec![
            seg(Direction::Up,   0,  4, 100, 200),
            seg(Direction::Down, 4,  8, 200, 100),
            seg(Direction::Up,   8, 12, 100, 200),
        ];
        let closes: Vec<i64> = (0..16).map(|i| 100 + (i % 4) * 10).collect();
        let layer = ParseLayer { segments: Rc::new(segments), merged_bars: Rc::new(bars_from_closes(&closes)), ..Default::default() };
        let out = classify(&layer, &cfg);
        // L0 级别 bsp 不含第二类（三段交替窗口结构上界）。
        for p in out.levels[0].bsp.iter() {
            assert!(!p.bits.buy2, "L0→L1 三段交替窗口不产 B2（still-MISSING-窗口，codex 裁决）");
            assert!(!p.bits.sell2, "L0→L1 三段交替窗口不产 S2（still-MISSING-窗口，codex 裁决）");
        }
    }

    /// ★codex-decide-20260703 裁定 A 最大实现风险点（end_price 忠实性，单测强制）：级别-N 输入单元
    /// → Segment 的端点价按 fold_direction 取 hi/lo，且与 `segment_to_unit` 互逆（L0 段 round-trip
    /// bit-exact）。此测试失败 ⟹ 级别-N「线段」端点价错位 ⟹ A/C 破中枢几何 + judge_third 判据全错。
    #[test]
    fn unit_to_segment_endpoint_faithful_and_roundtrips() {
        use super::center::UnitRange;
        // 向上单元：起点=lo、终点=hi（seg_end 取 end_price=hi=高点）。
        let up = UnitRange { start_index: 4, end_index: 8, direction: Direction::Up, lo: 90, hi: 150 };
        let s_up = unit_to_segment(&up);
        assert_eq!((s_up.start_price, s_up.end_price), (90, 150), "向上单元 end_price=hi（终点=高点）");
        assert_eq!(s_up.direction, Direction::Up);
        assert_eq!((s_up.start_index, s_up.end_index), (4, 8), "source_index 坐标保留（A/C 面积映射用）");
        // 向下单元：起点=hi、终点=lo（终点=低点）。
        let down = UnitRange { start_index: 8, end_index: 12, direction: Direction::Down, lo: 90, hi: 150 };
        let s_down = unit_to_segment(&down);
        assert_eq!((s_down.start_price, s_down.end_price), (150, 90), "向下单元 end_price=lo（终点=低点）");
        // round-trip：L0 段 → segment_to_unit → unit_to_segment == 原段（互逆 bit-exact）。
        for orig in [seg(Direction::Up, 0, 4, 100, 200), seg(Direction::Down, 4, 8, 200, 50)] {
            let back = unit_to_segment(&segment_to_unit(&orig));
            assert_eq!(
                (back.direction, back.start_index, back.end_index, back.start_price, back.end_price),
                (orig.direction, orig.start_index, orig.end_index, orig.start_price, orig.end_price),
                "segment_to_unit ∘ unit_to_segment = id（round-trip bit-exact）"
            );
        }
    }

    /// ★裁定 A gap-fill 非 no-op（结构合法性）：`extract_first_third_for_level` 在级别-N 下跌趋势
    /// units（承担线段角色）+ ≥2 依次向下中枢（Trend(Down)）+ C 段破最后中枢 + C<A 背驰上产 1 买。
    /// 复用 signal.rs `first_buy_extracted_with_trend_divergence` 的 A/B/C 几何，但以 UnitRange 表达
    /// ——证明级别-N 一/三类判定真接线（旧 `else { Vec::new() }` 恒产 0，此测试产 1 = 缺口已填）。
    #[test]
    fn level_ge1_extract_first_third_produces_type1_via_units() {
        use super::center::UnitRange;
        // 两依次向下中枢（c1.gg=210 < c0.dd=290 ⟹ DownContinuation ⟹ trend_class=Trend(Down)）。
        let c0 = Center { zd: 300, zg: 400, dd: 290, gg: 410, start_index: 0, end_index: 2 };
        let c1 = Center { zd: 100, zg: 200, dd: 90, gg: 210, start_index: 0, end_index: 8 };
        // Down 单元：lo=终点价、hi=起点价（unit_to_segment 还原 start=hi/end=lo）。
        let units = vec![
            UnitRange { start_index: 3, end_index: 5, direction: Direction::Down, lo: 250, hi: 350 }, // A 段（C0 离开）
            UnitRange { start_index: 5, end_index: 7, direction: Direction::Up, lo: 250, hi: 280 },   // B 段连接
            UnitRange { start_index: 9, end_index: 11, direction: Direction::Down, lo: 80, hi: 150 }, // C 段破 C1（<100）
        ];
        // A 段 bar[3,5] 急跌（hist 面积大）、C 段 bar[9,11] 缓动（面积小=背驰）——同 signal.rs fixture。
        let prices: Vec<i64> = vec![300, 300, 300, 300, 100, 250, 250, 250, 250, 248, 246, 244];
        let closes: Vec<f64> = prices.iter().map(|&v| v as f64).collect();
        let close_src: Vec<usize> = (0..prices.len()).collect();
        let hist = divergence::compute_macd(&closes, &ThetaConfig::default().macd).hist;
        // 本测试只验结构六 bit（force 旁挂不改），传空 dif/closes_tick ⟹ force=None（不影响 buy1 判据）。
        // Q7-#1 裁定C：显式全锚（本测试验证的是 Trend ownership 单元的 gap-fill 路径）。
        let anchors = [Some(Direction::Down), Some(Direction::Up), Some(Direction::Down)];
        let (bsp, _pan) = extract_first_third_for_level(&[c0, c1], &units, &anchors, &hist, &[], &[], &close_src, divergence::DivergenceGauge::default());
        let buy1: Vec<_> = bsp.iter().filter(|p| p.bits.buy1).collect();
        assert_eq!(buy1.len(), 1, "级别-N 下跌趋势 C 段破最后中枢 ∧ C<A 背驰 ⟹ 一个 1 买（缺口已填，非 no-op）");
        assert_eq!(buy1[0].source_index, 11, "1 买端点 = C 段（破最后中枢单元）终止 source_index");
        assert_eq!(buy1[0].pivot_low, 80, "1 买止损源 = pivot_low（C 段破中枢端点极值）");
        // ★owner 载体补齐（关③ 补记② 路径 (a)）：一类点构造时填入判定中枢 last_center=c1
        //（被破的最后中枢）——名实一致根据同 signal.rs `first_buy_extracted_with_trend_divergence`
        //（本测试复用其 A/B/C 几何的 UnitRange 表达）；center 是 owner 载体，止损仍 pivot
        //（pivot_low=80 上条已锁，center 不进 1/2 类止损判据）。
        // （#218 面 A 载体形态机械适配：一/三类载 OwnerRef::Center，语义不动。）
        assert_eq!(buy1[0].center, Some(signal::OwnerRef::Center(c1)), "一类点 center = 判定中枢（owner 载体）；止损仍 pivot 非 center");
    }

    /// ★裁定 A 三类（高级别「中枢外缘区间」边界语义，codex 风险点单独 snapshot）：级别-N 离开中枢
    /// + 回试不重入 ⟹ 3 买。`judge_third` 在级别-N units（外缘区间端点 hi/lo）vs 几何中枢 [zd,zg]
    /// 上判定——离开单元终点 > c.zg ∧ 回试单元终点 > c.zg（严格不触闭区间）。
    #[test]
    fn level_ge1_extract_first_third_produces_type3_via_units() {
        use super::center::UnitRange;
        // 单中枢 [100,200]（盘整 τ ⟹ 无一类）——三类是纯几何位置判据，不依赖趋势门控。
        let c = Center { zd: 100, zg: 200, dd: 90, gg: 210, start_index: 0, end_index: 12 };
        let units = vec![
            // 离开单元：向上，终点=hi=250 > zg=200（离开中枢上方）。
            UnitRange { start_index: 12, end_index: 16, direction: Direction::Up, lo: 150, hi: 250 },
            // 回试单元：向下，终点=lo=210 > zg=200（不重入闭区间中枢）⟹ 3 买。
            UnitRange { start_index: 16, end_index: 20, direction: Direction::Down, lo: 210, hi: 250 },
        ];
        // 三类无 MACD 依赖（纯整数几何），hist 空亦可——传空 hist/dif/closes_tick（第一类自然不产，force=None）。
        // Q7-#1 裁定C：显式全锚（leave 单元有 Trend ownership 资格的三类路径）。
        let anchors = [Some(Direction::Up), Some(Direction::Down)];
        let (bsp, _pan) = extract_first_third_for_level(&[c], &units, &anchors, &[], &[], &[], &(0..24).collect::<Vec<_>>(), divergence::DivergenceGauge::default());
        let buy3: Vec<_> = bsp.iter().filter(|p| p.bits.buy3).collect();
        assert_eq!(buy3.len(), 1, "级别-N 离开中枢 + 回试不重入 ⟹ 一个 3 买（外缘区间端点判据）");
        assert_eq!(buy3[0].source_index, 20, "3 买端点 = 回试单元终止 source_index");
        assert_eq!(buy3[0].pivot_low, 210, "3 买止损源 = pivot_low（回试低点）");
        assert_eq!(
            buy3[0].center.and_then(|o| match o { signal::OwnerRef::Center(c) => Some(c.zg), _ => None }),
            Some(200),
            "3 买 center=Some（止损=zg；#218 面 A 载体形态：Center 变体读出）"
        );
    }

    /// ★Q7-#1 裁定C + p117 窄域授权（686 翻转条款第一支，终端背书裁定 T2 核准）：Consolidation
    /// ownership 的 endpoint fallback 单元（anchor=None）在**第一类路径**经窄域授权降级——方向锚
    /// 取单元结构方向（`anchors_self`，行程方向=τ 等式 veto，「趋势中」定义域由 τ 门承担），
    /// fallback 不再是第一类击杀理由；**三类路径**裁定C 整体保留（leave 段 fallback 仍不得作
    /// 三类方向锚）。三向验证（同 fixture 对照）：全锚 ⟹ 1 买产（对照组，保留）；A 段 fallback
    /// ⟹ 结构同向筛选可配 ⟹ 产 1 买（0→1 授权翻转）；C 段 fallback ⟹ 结构方向 Down=τ ⟹ broke
    /// 触发 ⟹ 产 1 买（0→1 授权翻转）。三类：leave 段 fallback ⟹ 不产 3 买（保留）。成员身份
    /// 不变（centers/分解不受锚门影响）。
    #[test]
    fn q7_ruling_c_first_class_structural_direction_third_class_provenance_kept() {
        use super::center::UnitRange;
        // fixture 同 level_ge1_extract_first_third_fills_type1_gap（两下行中枢 + A/B/C 三单元）。
        let c0 = Center { zd: 300, zg: 400, dd: 290, gg: 410, start_index: 0, end_index: 2 };
        let c1 = Center { zd: 100, zg: 200, dd: 90, gg: 210, start_index: 0, end_index: 8 };
        let units = vec![
            UnitRange { start_index: 3, end_index: 5, direction: Direction::Down, lo: 250, hi: 350 },
            UnitRange { start_index: 5, end_index: 7, direction: Direction::Up, lo: 250, hi: 280 },
            UnitRange { start_index: 9, end_index: 11, direction: Direction::Down, lo: 80, hi: 150 },
        ];
        let prices: Vec<i64> = vec![300, 300, 300, 300, 100, 250, 250, 250, 250, 248, 246, 244];
        let closes: Vec<f64> = prices.iter().map(|&v| v as f64).collect();
        let close_src: Vec<usize> = (0..prices.len()).collect();
        let hist = divergence::compute_macd(&closes, &ThetaConfig::default().macd).hist;
        let n_buy1 = |anchors: &[Option<Direction>]| {
            let (bsp, _) = extract_first_third_for_level(&[c0, c1], &units, anchors, &hist, &[], &[], &close_src, divergence::DivergenceGauge::default());
            bsp.iter().filter(|p| p.bits.buy1).count()
        };
        // 对照组：全锚 ⟹ 1 买产（gap-fill 路径活；保留断言）。
        assert_eq!(n_buy1(&[Some(Direction::Down), Some(Direction::Up), Some(Direction::Down)]), 1);
        // A 段单元 fallback：p117 后第一类 A 窗筛选用单元结构方向（行程方向 Down=τ）⟹ A 候选
        // 可配 ⟹ 产 1 买（S4 救回型；授权翻转 0→1。provenance 锚消费方仅余三类/诊断仪器）。
        assert_eq!(n_buy1(&[None, Some(Direction::Up), Some(Direction::Down)]), 1,
            "p117 窄域授权：第一类 A 段方向锚 = 单元结构方向（fallback 单元行程方向=τ 时可配）");
        // C 段（破中枢段）单元 fallback：p117 后 broke 锚门用单元结构方向（Down=τ）⟹ 触发 ⟹
        // 产 1 买（S2 救回型；授权翻转 0→1。行程方向 ≠τ 的单元仍拒——signal.rs 反向 veto 锁）。
        assert_eq!(n_buy1(&[Some(Direction::Down), Some(Direction::Up), None]), 1,
            "p117 窄域授权：第一类破中枢段方向锚 = 单元结构方向（fallback 单元行程方向=τ 时触发）");
        // 三类：leave 段 fallback ⟹ 不产 3 买（686 对三类的保护整体保留；retest 是几何角色不设锚门）。
        let c = Center { zd: 100, zg: 200, dd: 90, gg: 210, start_index: 0, end_index: 12 };
        let u3 = vec![
            UnitRange { start_index: 12, end_index: 16, direction: Direction::Up, lo: 150, hi: 250 },
            UnitRange { start_index: 16, end_index: 20, direction: Direction::Down, lo: 210, hi: 250 },
        ];
        let src24: Vec<usize> = (0..24).collect();
        let (bsp, _) = extract_first_third_for_level(&[c], &u3, &[None, Some(Direction::Down)], &[], &[], &[], &src24, divergence::DivergenceGauge::default());
        assert_eq!(bsp.iter().filter(|p| p.bits.buy3).count(), 0, "fallback 单元不得作三类离开段方向锚（686 裁定C 三类保留）");
        // 成员身份不变：同 fixture 全锚下产出恢复（锚门不改变序列成员/中枢几何）。
        let (bsp2, _) = extract_first_third_for_level(&[c], &u3, &[Some(Direction::Up), Some(Direction::Down)], &[], &[], &[], &src24, divergence::DivergenceGauge::default());
        assert_eq!(bsp2.iter().filter(|p| p.bits.buy3).count(), 1);
    }

    /// ★L2 信号普查诊断（裁定 A gap-fill 真实数据核验，`--ignored` 手动跑，依赖 analysis/data_cache）：
    /// 逐级别统计 BTC 全量分类的 buy1/sell1/buy2/sell2/buy3/sell3 计数 + 抽样 level≥1 一类端点。
    /// 修前 level≥1 一/三类恒 0（audit #119 `else{Vec::new()}`），故 level≥1 的 type1/type3 计数 =
    /// 本次实装引入的净增信号。运行：`cargo test --lib -- --ignored --nocapture level_signal_census_btc`。
    #[test]
    #[ignore = "L2 真实数据普查：cargo test --lib -- --ignored --nocapture level_signal_census_btc"]
    fn level_signal_census_btc() {
        use super::super::backtest::data::load_by_symbol;
        use super::super::parser::parse_layer;
        let cfg = ThetaConfig::default();
        let full = load_by_symbol("BTC", &cfg).expect("BTC 数据加载（analysis/data_cache/btc_1m_full.json）");
        // 可选日期窗（CENSUS_WINDOW="2020-10-01,2021-04-01"）——检验 type1 的水平线依赖性：
        // 全历史中枢链全局非单调 ⟹ trend_class=Degenerate ⟹ type1=0；单向牛/熊窗内某级链可单调 ⟹ type1>0。
        let ds = match std::env::var("CENSUS_WINDOW") {
            Ok(w) => {
                let (s, e) = w.split_once(',').expect("CENSUS_WINDOW 格式 start,end");
                eprintln!("[census] window={s}..{e}");
                full.slice_date_window(s, e)
            }
            Err(_) => full,
        };
        eprintln!("[census] BTC bars={}", ds.bars.len());
        let layer = parse_layer(&ds.bars, &cfg);
        eprintln!("[census] L0 segments={} merged_bars={}", layer.segments.len(), layer.merged_bars.len());
        let out = classify(&layer, &cfg);
        eprintln!("[census] levels={}", out.levels.len());
        for (li, lv) in out.levels.iter().enumerate() {
            let trend = lv.moves.iter().filter(|m| m.kind == MoveKind::Trend).count();
            let (mut b1, mut s1, mut b2, mut s2, mut b3, mut s3) = (0, 0, 0, 0, 0, 0);
            for p in lv.bsp.iter() {
                b1 += p.bits.buy1 as usize; s1 += p.bits.sell1 as usize;
                b2 += p.bits.buy2 as usize; s2 += p.bits.sell2 as usize;
                b3 += p.bits.buy3 as usize; s3 += p.bits.sell3 as usize;
            }
            eprintln!(
                "[census] L{li}: centers={} moves={}(trend={}) bsp={} pan_div={} | buy1={b1} sell1={s1} buy2={b2} sell2={s2} buy3={b3} sell3={s3}",
                lv.centers.len(), lv.moves.len(), trend, lv.bsp.len(), lv.pan_div.len()
            );
            // 抽样：level≥1 的前 3 个一类端点（若有）+ 前 3 个三类端点（人工核结构合法性——
            // source_index + center[zd,zg] + pivot（回试端点极值）；三类不重入判据由 judge_third 保证）。
            if li >= 1 {
                let t1: Vec<_> = lv.bsp.iter().filter(|p| p.bits.buy1 || p.bits.sell1).take(3).collect();
                for (k, p) in t1.iter().enumerate() {
                    eprintln!(
                        "[census]   L{li} type1#{k}: src_idx={} buy1={} sell1={} break_dir={:?} pivot_low={} pivot_high={}",
                        p.source_index, p.bits.buy1, p.bits.sell1, p.struct_break_dir, p.pivot_low, p.pivot_high
                    );
                }
                let t3: Vec<_> = lv.bsp.iter().filter(|p| p.bits.buy3 || p.bits.sell3).take(3).collect();
                for (k, p) in t3.iter().enumerate() {
                    eprintln!(
                        "[census]   L{li} type3#{k}: src_idx={} buy3={} sell3={} center_zd={:?} center_zg={:?} pivot_low={} pivot_high={}",
                        p.source_index, p.bits.buy3, p.bits.sell3,
                        p.center.and_then(|o| match o { signal::OwnerRef::Center(c) => Some(c.zd), _ => None }),
                        p.center.and_then(|o| match o { signal::OwnerRef::Center(c) => Some(c.zg), _ => None }),
                        p.pivot_low, p.pivot_high
                    );
                }
            }
        }
    }

    /// ★#141 一类判据链漏斗普查（外审问题包证据，`--ignored` 手动跑，依赖 analysis/data_cache）：
    /// 逐级别逐环真实计数——候选评估→趋势门→≥2中枢→broke→A/C配对→背驰。级别循环与 `classify_impl`
    /// 同一批私有函数（classify_level/compose_level/project_to_units），每级 centers 与生产 `classify`
    /// 输出 assert 对拍（675号：探针走生产路径）。另产每级中枢链关系直方图 + 前缀 τ 时间线（因果
    /// 重放中趋势门何时永久锁死 Degenerate）+ 反事实局部同向 run 计数（若按走势分解的局部趋势数）。
    /// 运行：`cargo test --release --lib -- --ignored --nocapture type1_funnel_census_btc`
    /// 窗口对照：`CENSUS_WINDOW="2020-10-01,2021-04-01" cargo test --release --lib -- --ignored --nocapture type1_funnel_census_btc`
    #[test]
    #[ignore = "L2 真实数据漏斗普查：cargo test --release --lib -- --ignored --nocapture type1_funnel_census_btc"]
    fn type1_funnel_census_btc() {
        use super::super::backtest::data::load_by_symbol;
        use super::super::parser::parse_layer;
        use super::center::{classify_relation, CenterRelation};
        let cfg = ThetaConfig::default();
        let full = load_by_symbol("BTC", &cfg).expect("BTC 数据加载（analysis/data_cache/btc_1m_full.json）");
        let ds = match std::env::var("CENSUS_WINDOW") {
            Ok(w) => {
                let (s, e) = w.split_once(',').expect("CENSUS_WINDOW 格式 start,end");
                eprintln!("[funnel] window={s}..{e}");
                full.slice_date_window(s, e)
            }
            Err(_) => full,
        };
        eprintln!("[funnel] BTC bars={}", ds.bars.len());
        let layer = parse_layer(&ds.bars, &cfg);
        eprintln!("[funnel] L0 segments={} merged_bars={}", layer.segments.len(), layer.merged_bars.len());

        // 生产对拍源（675号守卫：级别循环不分叉）。
        let out = classify(&layer, &cfg);

        // classify_impl 同源输入（同一批私有函数，非重写）。
        let min_parts = cfg.level.min_parts_per_level as usize;
        let l_max = cfg.level.l_max as usize;
        let mut units: Vec<UnitRange> = layer.segments.iter().map(segment_to_unit).collect();
        assert!(!units.is_empty(), "空 L0 无漏斗对象");
        let closes: Vec<f64> = layer.merged_bars.iter().map(|b| b.close as f64).collect();
        let close_src: Vec<usize> = layer.merged_bars.iter().map(|b| b.source_index).collect();
        let series = divergence::compute_macd(&closes, &cfg.macd);
        let closes_tick: Vec<Tick> = layer.merged_bars.iter().map(|b| b.close).collect();
        let mut moves_tower: Rc<Vec<LeveledMove>> = Rc::new(
            units
                .iter()
                .enumerate()
                .map(|(i, u)| LeveledMove::from_unit(u, ElementId { level: 0, ordinal: i as u64 }))
                .collect(),
        );

        for level_idx in 0..=l_max {
            if units.len() < min_parts {
                break;
            }
            let is_l0 = level_idx == 0;
            let (centers, _outcome) = classify_level(&units, is_l0);
            assert_eq!(
                centers, *out.levels[level_idx].centers,
                "L{level_idx} 中枢对拍（探针级别循环须与生产 classify 逐字段一致）"
            );

            // ★task #142 量化验收：延伸段数分布（窗口段数 = seed 3 + 延伸段；同一 build 直调
            // detect_centers_windowed_resume 取窗口，中枢序列与生产 classify_level 对拍）。
            {
                let build: fn(&UnitRange, &UnitRange, &UnitRange) -> Option<Center> = if is_l0 {
                    center::center_from_segments
                } else {
                    center::center_from_window
                };
                let windowed = recursive_tower::detect_centers_windowed_resume(&units, build, 0).0;
                assert_eq!(
                    windowed.iter().map(|(c, _)| *c).collect::<Vec<_>>(),
                    centers,
                    "L{level_idx} 窗口探针中枢序列 == 生产中枢序列"
                );
                let mut hist = [0usize; 4]; // 桶：=3（无延伸）/4-5/6-9/≥10 段
                let mut max_w = 0usize;
                for (_, (s, e)) in &windowed {
                    let w = e - s + 1;
                    max_w = max_w.max(w);
                    hist[if w <= 3 { 0 } else if w <= 5 { 1 } else if w <= 9 { 2 } else { 3 }] += 1;
                }
                eprintln!(
                    "[funnel] L{level_idx}: 窗口段数分布 =3段:{} 4-5:{} 6-9:{} ≥10:{} max={}",
                    hist[0], hist[1], hist[2], hist[3], max_w
                );
            }

            // 中枢链相邻关系直方图 + 前缀 τ 时间线 + 反事实局部同向 run。
            let rels: Vec<CenterRelation> =
                centers.windows(2).map(|w| classify_relation(&w[0], &w[1])).collect();
            let n_up = rels.iter().filter(|r| **r == CenterRelation::UpContinuation).count();
            let n_down = rels.iter().filter(|r| **r == CenterRelation::DownContinuation).count();
            let n_exp = rels.iter().filter(|r| **r == CenterRelation::LevelExpansion).count();
            // 前缀 τ：τ(前k中枢)=Trend ⟺ k≥2 ∧ rels[0..k-1] 全等且非 Expansion。锁死点=首个异关系下标。
            let trend_open = !rels.is_empty() && rels[0] != CenterRelation::LevelExpansion;
            let lock_at = if rels.is_empty() {
                None
            } else if !trend_open {
                Some(0) // 首关系即 Expansion ⟹ 第3个中枢确认时 τ 已锁死 Degenerate
            } else {
                rels.iter().position(|r| *r != rels[0])
            };
            // 反事实（若走势分解为局部走势类型）：同向关系（Up/Down）的极大 run，每个 run 长 L = 局部
            // 趋势含 L+1 个中枢。计 run 数与最长 run。
            let (mut runs_ge1, mut longest_run, mut cur_run) = (0usize, 0usize, 0usize);
            for (k, r) in rels.iter().enumerate() {
                let same_dir = *r != CenterRelation::LevelExpansion;
                let cont = same_dir && (k == 0 || rels[k - 1] == *r);
                if same_dir {
                    cur_run = if cont { cur_run + 1 } else { 1 };
                    if cur_run == 1 {
                        runs_ge1 += 1;
                    }
                    longest_run = longest_run.max(cur_run);
                } else {
                    cur_run = 0;
                }
            }
            let lock_desc = match lock_at {
                None if trend_open => format!("全链同向（不锁死）"),
                None => format!("链长<2 无关系"),
                Some(i) => {
                    let c_end = centers[i + 1].end_index;
                    let date = ds.dates.get(c_end).map(|d| d.get(..10).unwrap_or("?")).unwrap_or("?");
                    format!("中枢#{}（end_src={} {date}）", i + 1, c_end)
                }
            };

            let (segs, funnel_anchors): (Vec<Segment>, Option<Vec<Option<Direction>>>) = if is_l0 {
                (layer.segments.to_vec(), None)
            } else {
                // Q7-#1 裁定C + 675号：漏斗探针锚与生产 units_anchors 同源（producer blocks 派生）。
                let pb = &out.levels[level_idx - 1].moves;
                (
                    units.iter().map(unit_to_segment).collect(),
                    Some((0..units.len()).map(|i| decompose::center_own_dir_at(pb, i)).collect()),
                )
            };
            let f = signal::type1_funnel_dx(&centers, &segs, funnel_anchors.as_deref(), &series.hist, &series.dif, &closes_tick, &close_src);
            eprintln!(
                "[funnel] L{level_idx}: centers={} segs={} rel(up/down/exp)={}/{}/{} blocks(trend/consol)={}/{} 最长趋势块={}中枢 | 旧AllTrend锁死点={} 局部同向run≥2中枢数={} 最长run={}(={}中枢)",
                f.n_centers, f.n_segments, n_up, n_down, n_exp, f.n_trend_blocks, f.n_consol_blocks,
                f.longest_trend_run, lock_desc, runs_ge1, longest_run, longest_run + 1
            );
            eprintln!(
                "[funnel] L{level_idx}: 环0候选(有最近中枢)={} → 环1有前驱中枢={} → 环2过局部趋势门={} → 环3破最后中枢={} → 环4 A/C配对={} → 环4b 037:20破b极值={} → 环5坐标映射={} → 环6背驰C<A={}",
                f.s_with_center, f.s_pos_ge1, f.s_gate_open, f.s_broke, f.s_a_paired, f.s_extreme, f.s_mapped, f.s_diverge
            );
            // ★task #144 验收证据：2021 顶区 sell1 在全历史因果重放中出现（先例窗反差闭合的正验证，
            // 生产 classify 输出直读——非探针另算）。窗口 = #141 外审切窗 2020-10-01..2021-04-15。
            {
                let top_sell1: Vec<&str> = out.levels[level_idx]
                    .bsp
                    .iter()
                    .filter(|p| p.bits.sell1)
                    .filter_map(|p| ds.dates.get(p.source_index).map(|d| d.get(..10).unwrap_or("?")))
                    .filter(|d| ("2020-10-01".."2021-04-15").contains(d))
                    .collect();
                let n_sell1 =
                    out.levels[level_idx].bsp.iter().filter(|p| p.bits.sell1).count();
                let n_buy1 = out.levels[level_idx].bsp.iter().filter(|p| p.bits.buy1).count();
                eprintln!(
                    "[funnel] L{level_idx}: 全历史 buy1={} sell1={} | 2021顶区(2020-10-01..2021-04-15) sell1×{}: {:?}",
                    n_buy1, n_sell1, top_sell1.len(), top_sell1
                );
            }

            let (_cw, upper_moves, _) = compose_level(&units, &moves_tower[..], is_l0, level_idx as u32 + 1);
            units = project_to_units(&upper_moves, &out.levels[level_idx].moves); // Q7：生产同源块
            moves_tower = Rc::new(upper_moves);
            if units.is_empty() {
                break;
            }
        }
    }

    #[test]
    fn classify_empty_layer_yields_empty() {
        let cfg = ThetaConfig::default();
        let out = classify(&ParseLayer::default(), &cfg);
        assert_eq!(out, Classification::default());
    }

    #[test]
    fn fewer_than_min_parts_natural_termination() {
        // L0 线段数 < min_parts_per_level(3) ⟹ 无 L0 走势，levels 空（自然终止）。
        let cfg = ThetaConfig::default();
        let layer = ParseLayer {
            segments: Rc::new(vec![seg(Direction::Up, 0, 4, 0, 10), seg(Direction::Down, 4, 8, 10, 5)]),
            ..Default::default()
        };
        let out = classify(&layer, &cfg);
        assert!(out.levels.is_empty(), "2 段 < min_parts 3 ⟹ 自然终止");
    }

    #[test]
    fn three_overlapping_segments_form_center() {
        // 完整判据（Origin.CenterComplete，口径 B 637号）：方向交替 上-下-上 + 全三段核心非空。
        // 段区间 [0,10]up,[3,12]down,[5,15]up：核心取**全三段** zd=max(0,3,5)=5, zg=min(10,12,15)=10。
        // 第三段 [5,15] 收窄核心下沿（A 口径 zd=3 → B zd=5）⟹ 真中枢成立。
        let cfg = ThetaConfig::default();
        let layer = ParseLayer {
            segments: Rc::new(vec![
                seg(Direction::Up, 0, 4, 0, 10),
                seg(Direction::Down, 4, 8, 12, 3),
                seg(Direction::Up, 8, 12, 5, 15),
            ]),
            ..Default::default()
        };
        let out = classify(&layer, &cfg);
        assert!(!out.levels.is_empty());
        let l0 = &out.levels[0];
        assert_eq!(l0.centers.len(), 1, "三段方向交替+贯穿 ⟹ 一个真中枢");
        // 核心取全三段（口径 B，637号 computeZD/computeZG s1 s2 s3）——第三段收窄核心下沿至 5。
        assert_eq!((l0.centers[0].zd, l0.centers[0].zg), (5, 10));
        // 一个中枢 ⟹ 分解 = 单盘整块（PDF §6 情形1）。
        assert_eq!(l0.moves.len(), 1);
        assert_eq!((l0.moves[0].kind, l0.moves[0].start_center, l0.moves[0].end_center),
                   (MoveKind::Consolidation, 0, 0));
    }

    #[test]
    fn same_direction_three_segments_rejected_by_complete() {
        // G4 完整判据反退化：三段同向（全 up）+ 前两段核心非空，但无方向交替 ⟹ L0 不识别中枢
        // （旧几何窗口会误判，完整判据正确拒绝，对齐 Origin.CenterComplete.sameDir_not_centerConfirmed）。
        let cfg = ThetaConfig::default();
        let layer = ParseLayer {
            segments: Rc::new(vec![
                seg(Direction::Up, 0, 4, 10, 20),
                seg(Direction::Up, 4, 8, 18, 25),
                seg(Direction::Up, 8, 12, 22, 30),
            ]),
            ..Default::default()
        };
        let out = classify(&layer, &cfg);
        // 无方向交替 ⟹ L0 无中枢（完整判据拒绝单边三段）。
        assert_eq!(out.levels.len(), 1);
        assert!(out.levels[0].centers.is_empty(), "同向三段无方向交替 ⟹ 完整判据拒绝（非中枢）");
    }

    #[test]
    fn lmax_bound_respected() {
        // 构造大量重叠段，验证递归不超过 l_max+1 级（每级至少消耗中枢，最终自然终止）。
        let cfg = ThetaConfig::default();
        let mut segments = Vec::new();
        // 27 段全重叠区间 [0,100]（每三段成一中枢，逐级递归）。
        for i in 0..27 {
            let dir = if i % 2 == 0 { Direction::Up } else { Direction::Down };
            segments.push(seg(dir, i * 4, i * 4 + 4, 0, 100));
        }
        let layer = ParseLayer { segments: Rc::new(segments), ..Default::default() };
        let out = classify(&layer, &cfg);
        // 级别数不超过 l_max+1（reference:30 上界，default l_max=6）。
        assert!(out.levels.len() <= cfg.level.l_max as usize + 1);
    }

    #[test]
    fn no_center_terminates_recursion() {
        // 三段无公共重叠 ⟹ L0 无中枢 ⟹ moves 为空（HigherCenterCandidate→None）+ 递归终止。
        let cfg = ThetaConfig::default();
        let layer = ParseLayer {
            segments: Rc::new(vec![
                seg(Direction::Up, 0, 4, 0, 4),
                seg(Direction::Down, 4, 8, 14, 10),
                seg(Direction::Up, 8, 12, 20, 24),
            ]),
            ..Default::default()
        };
        let out = classify(&layer, &cfg);
        // L0 无中枢 ⟹ 该级 moves 空（裁决退化），递归在该级后终止（无上级单元）。
        assert_eq!(out.levels.len(), 1);
        assert!(out.levels[0].centers.is_empty());
        assert!(out.levels[0].moves.is_empty());
    }

    /// ★端到端 fixture（Lead 验证门）：≥3 重叠线段 → 非空中枢 + 至少一个买卖点。
    /// 解阻塞点 A：classify 在真实结构输入上返回非空 Classification（n_orders>0 的前提）。
    #[test]
    fn end_to_end_third_buy_signal() {
        let cfg = ThetaConfig::default();
        // 段0-2：三段在 [100,200] 重叠 ⟹ seed 中枢，核心 [ZD,ZG]=[100,200] 冻结，end_index=12。
        // 段3：向上离开——延伸语义下（PDF §5 Step3，task #142）离开段必须与冻结核心不相交：
        //   lo=205 > ZG=200 ⟹ non-extension（旧 fixture lo=150 ≤ 200 会被 Step2 吸收进中枢 ⟹ 无离开段）。
        // 段4：向下回试低点 210 > zg=200（严格不触闭区间）⟹ 3 买 @ source_index=20。
        let layer = ParseLayer {
            segments: Rc::new(vec![
                seg(Direction::Up, 0, 4, 100, 200),
                seg(Direction::Down, 4, 8, 200, 100),
                seg(Direction::Up, 8, 12, 100, 200),
                seg(Direction::Up, 12, 16, 205, 250),   // 离开中枢上方（lo=205>ZG ⟹ non-extension）
                seg(Direction::Down, 16, 20, 250, 210), // 回试低点 > zg → 3 买
            ]),
            ..Default::default()
        };
        let out = classify(&layer, &cfg);

        // 1) 非空 Classification + L0 有中枢。
        assert!(!out.levels.is_empty(), "解阻塞 A：classify 返回非空");
        let l0 = &out.levels[0];
        assert_eq!(l0.centers.len(), 1, "三段重叠 ⟹ 一个中枢");
        assert_eq!((l0.centers[0].zd, l0.centers[0].zg), (100, 200));

        // 2) 至少一个买卖点（第三类买点），且携带结构止损价 single source。
        assert!(!l0.bsp.is_empty(), "解阻塞 A：L0 至少一个买卖点");
        let third_buys: Vec<_> = l0.bsp.iter().filter(|p| p.bits.buy3).collect();
        assert_eq!(third_buys.len(), 1, "一个第三类买点");
        let p = third_buys[0];
        assert_eq!(p.source_index, 20, "买卖点定位回试端点");
        assert_eq!(p.pivot_low, 210, "结构止损价 pivot_low single source");
        assert_eq!(
            p.center.and_then(|o| match o { signal::OwnerRef::Center(c) => Some(c.zg), _ => None }),
            Some(200),
            "3 买止损 = ZG single source（#218 面 A 载体形态：Center 变体读出）"
        );
    }

    /// 端到端边界：有中枢但无离开/回试 ⟹ 中枢非空、bsp 空（无买卖点是诚实产出，非错误）。
    #[test]
    fn end_to_end_center_without_signal() {
        let cfg = ThetaConfig::default();
        // 三段重叠成中枢，但无后续离开线段 ⟹ 无第三类买卖点。
        let layer = ParseLayer {
            segments: Rc::new(vec![
                seg(Direction::Up, 0, 4, 100, 200),
                seg(Direction::Down, 4, 8, 200, 100),
                seg(Direction::Up, 8, 12, 100, 200),
            ]),
            ..Default::default()
        };
        let out = classify(&layer, &cfg);
        assert_eq!(out.levels[0].centers.len(), 1);
        assert!(out.levels[0].bsp.is_empty(), "无离开/回试 ⟹ 无买卖点（诚实空）");
    }

    /// ★classify_with_tower (i) 段导出桥——tower 非空 + depth≥1 真嵌套存在。
    ///
    /// 9 段 L0 → 3 个 L1 走势 → L2 中枢（几何路径）：tower[1] 含 sub_moves 非空的
    /// LeveledMove（RMove::Compose，depth=1 真嵌套）。坐实：导出桥正确产出真嵌套塔。
    #[test]
    fn classify_with_tower_depth_ge1_true_nesting() {
        let cfg = ThetaConfig::default();
        // 9 段：三组 up-down-up（每组 → 一个 L1 走势），三个 L1 走势外缘重叠成 L2 中枢。
        // ★task #142 延伸语义诚实重算（同 end_to_end_second_buy_via_l1_l2_geometric 推导）：三组
        // 核心分离（组B 首段 hi=115<ZD_A=120、组C 首段 lo=115>ZG_B=114 ⟹ non-extension，PDF §5
        // Step3），外缘 O_A=[110,150]/O_B=[80,125]/O_C=[112,148] 共同相交 ⟹ L2 核心 [112,125] 非空。
        let segments = vec![
            seg(Direction::Up,   0,  4, 110, 150),
            seg(Direction::Down, 4,  8, 150, 120),
            seg(Direction::Up,   8, 12, 120, 148),
            seg(Direction::Down,12, 16, 115,  80),
            seg(Direction::Up,  16, 20,  80, 125),
            seg(Direction::Down,20, 24, 114,  85),
            seg(Direction::Up,  24, 28, 115, 148),
            seg(Direction::Down,28, 32, 148, 112),
            seg(Direction::Up,  32, 36, 112, 147),
        ];
        let mut closes: Vec<i64> = Vec::new();
        for i in 0..12 { closes.push(100 + if i % 2 == 0 { 40 } else { -40 }); }
        for i in 0..12 { closes.push(100 + if i % 2 == 0 {  5 } else {  -5 }); }
        for i in 0..16 { closes.push(100 + if i % 2 == 0 {  3 } else {  -3 }); }
        let layer = ParseLayer { segments: Rc::new(segments), merged_bars: Rc::new(bars_from_closes(&closes)), ..Default::default() };
        let (_, tower) = classify_with_tower(&layer, &cfg);
        assert!(!tower.is_empty(), "tower 非空（至少 L0 级被处理）");
        assert!(tower.len() >= 2, "9 段 L0 → 3 个 L1 走势 → L2 中枢 ⟹ tower 至少 2 层");
        // depth≥1 真嵌套：tower[1] 含 sub_moves 非空的 LeveledMove（RMove::Compose，L1 输入塔）。
        let has_true_nesting = tower[1].iter().any(|m| !m.sub_moves.is_empty());
        assert!(has_true_nesting, "tower[1] 含真嵌套 LeveledMove（sub_moves 非空，depth≥1）");
    }

    /// ★classify_with_tower Classification 与 classify 同输入 bit-identical（导出不改原分类）。
    #[test]
    fn classify_with_tower_classification_equals_classify() {
        let cfg = ThetaConfig::default();
        // ★task #142 延伸语义诚实重算（同 end_to_end_second_buy_via_l1_l2_geometric 推导）：三组
        // 核心分离（组B 首段 hi=115<ZD_A=120、组C 首段 lo=115>ZG_B=114 ⟹ non-extension，PDF §5
        // Step3），外缘 O_A=[110,150]/O_B=[80,125]/O_C=[112,148] 共同相交 ⟹ L2 核心 [112,125] 非空。
        let segments = vec![
            seg(Direction::Up,   0,  4, 110, 150),
            seg(Direction::Down, 4,  8, 150, 120),
            seg(Direction::Up,   8, 12, 120, 148),
            seg(Direction::Down,12, 16, 115,  80),
            seg(Direction::Up,  16, 20,  80, 125),
            seg(Direction::Down,20, 24, 114,  85),
            seg(Direction::Up,  24, 28, 115, 148),
            seg(Direction::Down,28, 32, 148, 112),
            seg(Direction::Up,  32, 36, 112, 147),
        ];
        let mut closes: Vec<i64> = Vec::new();
        for i in 0..12 { closes.push(100 + if i % 2 == 0 { 40 } else { -40 }); }
        for i in 0..12 { closes.push(100 + if i % 2 == 0 {  5 } else {  -5 }); }
        for i in 0..16 { closes.push(100 + if i % 2 == 0 {  3 } else {  -3 }); }
        let layer = ParseLayer { segments: Rc::new(segments), merged_bars: Rc::new(bars_from_closes(&closes)), ..Default::default() };
        let expected = classify(&layer, &cfg);
        let (actual, _) = classify_with_tower(&layer, &cfg);
        assert_eq!(actual, expected, "classify_with_tower Classification 与 classify bit-identical（原分类不变）");
    }

    // ──────────────────────────────────────────────────────────────────────
    //  增量塔 API bit-exact（task #93：incremental == 全量，逐 bar 断言）
    // ──────────────────────────────────────────────────────────────────────

    /// 构造逐段追加的合成段序列（方向交替 + 价格震荡，产足够中枢触发多级塔）。
    fn synthetic_segments(count: usize) -> Vec<Segment> {
        (0..count)
            .map(|i| {
                let dir = if i % 2 == 0 { Direction::Up } else { Direction::Down };
                let base = 100i64 + (i as i64) * 3;
                let swing = if i % 2 == 0 { 50 } else { -50 };
                let sp = base;
                let ep = base + swing;
                seg(dir, i * 4, i * 4 + 4, sp, ep)
            })
            .collect()
    }

    /// ★增量塔单次 bit-exact：对完整段序列，`classify_with_tower_incremental(.., fresh cache)`
    /// 输出 == `classify_with_tower`（Classification + tower 逐字段相等）。
    ///
    /// fresh cache（空）从 consumed=0 续扫 == 全量扫描。验证增量入口的基础正确性。
    #[test]
    fn incremental_tower_fresh_cache_equals_full() {
        let cfg = ThetaConfig::default();
        for n in [3usize, 6, 9, 12, 18] {
            let segments = synthetic_segments(n);
            let closes: Vec<i64> = (0..(n * 4 + 8) as i64).map(|i| 100 + (i % 5) * 5).collect();
            let layer =
                ParseLayer { segments: Rc::new(segments), merged_bars: Rc::new(bars_from_closes(&closes)), ..Default::default() };

            let (full_cls, full_tower) = classify_with_tower(&layer, &cfg);
            let mut cache = TowerCache::new();
            let (inc_cls, inc_tower) = classify_with_tower_incremental(&layer, &cfg, &mut cache);

            assert_eq!(inc_cls, full_cls, "n={n}: 增量 Classification == 全量");
            assert_eq!(inc_tower.len(), full_tower.len(), "n={n}: 增量 tower 层数 == 全量");
            for (lvl, (il, fl)) in inc_tower.iter().zip(full_tower.iter()).enumerate() {
                assert_eq!(il, fl, "n={n} level {lvl}: 增量 tower 级 LeveledMove 序列 == 全量");
            }
        }
    }

    /// ★增量塔逐段追加 bit-exact（#93 核心铁律）：模拟 per-bar substrate 逐段追加，
    /// 每步断言 `classify_with_tower_incremental(layer[..=i], cache)` ==
    /// `classify_with_tower(layer[..=i])`（Classification + tower 逐字段相等）。
    ///
    /// 这是增量塔的真实使用场景——段账本单调增长，cache 跨步复用前级 confirmed 前缀。
    /// 任何 resume bit-exact 破裂、跨级传播错误、裁决漂移都会在此捕获。
    #[test]
    fn incremental_tower_per_segment_append_matches_full() {
        let cfg = ThetaConfig::default();
        let all_segments = synthetic_segments(21);
        let closes: Vec<i64> = (0..100).map(|i| 100 + (i % 7) * 4).collect();

        let mut cache = TowerCache::new();
        for n in 1..=all_segments.len() {
            let segments = all_segments[..n].to_vec();
            let layer = ParseLayer {
                segments: Rc::new(segments),
                merged_bars: Rc::new(bars_from_closes(&closes)),
                ..Default::default()
            };

            // 全量基准。
            let (full_cls, full_tower) = classify_with_tower(&layer, &cfg);
            // 增量（cache 跨步复用）。
            let (inc_cls, inc_tower) = classify_with_tower_incremental(&layer, &cfg, &mut cache);

            assert_eq!(inc_cls, full_cls, "n={n}: 增量 Classification != 全量（bit-exact 破裂）");
            assert_eq!(
                inc_tower.len(),
                full_tower.len(),
                "n={n}: 增量 tower 层数 != 全量"
            );
            for (lvl, (il, fl)) in inc_tower.iter().zip(full_tower.iter()).enumerate() {
                assert_eq!(
                    il, fl,
                    "n={n} level {lvl}: 增量 tower 级 LeveledMove != 全量（真 subs 嵌套破裂）"
                );
            }
        }
    }



    /// ★段账本回缩退化 bit-exact：模拟 parser 回撤最后一段（非单调追加），
    /// `cache` 自动检测回缩 ⟹ 清空 + 全量重扫 ⟹ 仍 bit-exact（退化不破坏正确性）。
    #[test]
    fn incremental_tower_shrink_falls_back_to_full() {
        let cfg = ThetaConfig::default();
        let all_segments = synthetic_segments(12);
        let closes: Vec<i64> = (0..80).map(|i| 100 + (i % 6) * 4).collect();

        let mut cache = TowerCache::new();
        // 先追加到 12 段。
        let layer_full =
            ParseLayer { segments: Rc::new(all_segments.clone()), merged_bars: Rc::new(bars_from_closes(&closes)), ..Default::default() };
        let _ = classify_with_tower_incremental(&layer_full, &cfg, &mut cache);
        // 回缩到 8 段（parser 回撤）。
        let layer_shrink = ParseLayer {
            segments: Rc::new(all_segments[..8].to_vec()),
            merged_bars: Rc::new(bars_from_closes(&closes)),
            ..Default::default()
        };
        let (full_cls, full_tower) = classify_with_tower(&layer_shrink, &cfg);
        let (inc_cls, inc_tower) = classify_with_tower_incremental(&layer_shrink, &cfg, &mut cache);
        assert_eq!(inc_cls, full_cls, "回缩退化：增量 Classification == 全量");
        assert_eq!(inc_tower, full_tower, "回缩退化：增量 tower == 全量");
    }

    /// ★B2 真产出 bit-exact：增量塔在产 B2 的真实结构（9 段三组 up-down-up）下，
    /// `classify_with_tower_incremental` 产出的 B2 与全量 `classify_with_tower` bit-identical——
    /// 验证增量 compose 的真 Fugue 547（subs 真 Compose，B2 真可产，禁级别差伪造）。
    #[test]
    fn incremental_tower_preserves_b2_second_buy() {
        let cfg = ThetaConfig::default();
        // ★task #142 延伸语义诚实重算（同 end_to_end_second_buy_via_l1_l2_geometric 推导）：三组
        // 核心分离（组B 首段 hi=115<ZD_A=120、组C 首段 lo=115>ZG_B=114 ⟹ non-extension，PDF §5
        // Step3），外缘 O_A=[110,150]/O_B=[80,125]/O_C=[112,148] 共同相交 ⟹ L2 核心 [112,125] 非空。
        let segments = vec![
            seg(Direction::Up,   0,  4, 110, 150),
            seg(Direction::Down, 4,  8, 150, 120),
            seg(Direction::Up,   8, 12, 120, 148),
            seg(Direction::Down,12, 16, 115,  80),
            seg(Direction::Up,  16, 20,  80, 125),
            seg(Direction::Down,20, 24, 114,  85),
            seg(Direction::Up,  24, 28, 115, 148),
            seg(Direction::Down,28, 32, 148, 112),
            seg(Direction::Up,  32, 36, 112, 147),
        ];
        let mut closes: Vec<i64> = Vec::new();
        for i in 0..12 { closes.push(100 + if i % 2 == 0 { 40 } else { -40 }); }
        for i in 0..12 { closes.push(100 + if i % 2 == 0 {  5 } else {  -5 }); }
        for i in 0..16 { closes.push(100 + if i % 2 == 0 {  3 } else {  -3 }); }
        let layer = ParseLayer { segments: Rc::new(segments), merged_bars: Rc::new(bars_from_closes(&closes)), ..Default::default() };

        let (full_cls, _) = classify_with_tower(&layer, &cfg);
        let mut cache = TowerCache::new();
        let (inc_cls, _) = classify_with_tower_incremental(&layer, &cfg, &mut cache);

        // 全量产 1 个 B2（见 end_to_end_second_buy_via_l1_l2_geometric），增量须 bit-identical。
        let full_b2: Vec<_> = full_cls.levels[1].bsp.iter().filter(|p| p.bits.buy2).collect();
        let inc_b2: Vec<_> = inc_cls.levels[1].bsp.iter().filter(|p| p.bits.buy2).collect();
        assert_eq!(inc_b2.len(), full_b2.len(), "增量塔 B2 数量 == 全量（真 Fugue 547 保留）");
        assert_eq!(inc_b2.len(), 1, "增量塔仍真产 B2（subs 真 Compose，非级别差伪造）");
        assert_eq!(inc_b2[0].source_index, full_b2[0].source_index, "B2 source_index bit-exact");
        assert_eq!(inc_cls, full_cls, "增量塔完整 Classification == 全量（含 B2 BSP）");
    }

    /// ★codex 反例（cascade reset 完备性，L1 构造）：L0 frontier 末段**内点改写**——
    /// 上级投影 `UnitRange(lo,hi)` bit-identical 但底层 `sub_moves` 变。
    ///
    /// 场景：9 段三组（task #142 核心分离 fixture），末段 seg[8] `Up[32,36]` end_price 从 147 改写
    /// 为 140。140 是组 C 外缘内点（组 C max-hi=148(seg[6]/seg[7]) / min-lo=112(seg[7]/seg[8].sp)
    /// 不变）⟹ L0 该窗口中枢 gg/dd 不变 ⟹ L1 输入投影 `project_to_units` bit-identical。但 seg[8]
    /// 的 lo/hi 从 [112,147] 变 [112,140] ⟹ L0 upper_moves[2].sub_moves[2] 深嵌套坐标变。
    ///
    /// 旧守卫（仅比对本级 `project_to_units` 投影）：L0 reset 正确，但 L1 frontier_mutated=false
    /// 漏 reset ⟹ `cache.levels[1].upper_moves` 深嵌套 sub_moves 陈旧（仍 [112,147]）+ BSP memo
    /// （key 仅三长度）复用陈旧 BSP ⟹ 与全量发散。
    /// cascade reset 修复：L0 变异 → 强制 reset L1+（无条件跟随下级），深嵌套 sub_moves 重建为 [112,140]。
    ///
    /// **L1**（合成构造，验证管线完备性，非真实数据假设——formalization-validity-domain 231号）。
    #[test]
    fn cascade_reset_on_frontier_interior_rewrite() {
        let cfg = ThetaConfig::default();
        // ★task #142 延伸语义诚实重算：三组核心分离 fixture（同 end_to_end_second_buy_via_l1_l2_geometric
        // 推导——组B 首段 hi=115<ZD_A=120、组C 首段 lo=115>ZG_B=114 ⟹ non-extension，PDF §5 Step3）。
        let base = vec![
            seg(Direction::Up,   0,  4, 110, 150),
            seg(Direction::Down, 4,  8, 150, 120),
            seg(Direction::Up,   8, 12, 120, 148),
            seg(Direction::Down,12, 16, 115,  80),
            seg(Direction::Up,  16, 20,  80, 125),
            seg(Direction::Down,20, 24, 114,  85),
            seg(Direction::Up,  24, 28, 115, 148),
            seg(Direction::Down,28, 32, 148, 112),
            seg(Direction::Up,  32, 36, 112, 147), // v1 末段
        ];
        let mut closes: Vec<i64> = Vec::new();
        for i in 0..12 { closes.push(100 + if i % 2 == 0 { 40 } else { -40 }); }
        for i in 0..12 { closes.push(100 + if i % 2 == 0 {  5 } else {  -5 }); }
        for i in 0..16 { closes.push(100 + if i % 2 == 0 {  3 } else {  -3 }); }
        let merged = Rc::new(bars_from_closes(&closes));

        // v1: 末段 end_price=147。
        let layer_v1 = ParseLayer { segments: Rc::new(base.clone()), merged_bars: merged.clone(), ..Default::default() };
        // v2: 仅末段 end_price 改写 147→140（组 C 外缘内点，L1 投影不变，L0 sub_moves 变）。
        let mut v2_segs = base.clone();
        v2_segs[8].end_price = 140;
        let layer_v2 = ParseLayer { segments: Rc::new(v2_segs), merged_bars: merged.clone(), ..Default::default() };

        // 前提自检（codex 反例成立的必要条件）：L1 投影输入 v1==v2 bit-identical（守卫看不到变异），
        // 但 L0 末段 sub_moves 已变（147→140）。若此前提不成立，本测试不构成反例。
        let mk_l1_units = |segs: &Rc<Vec<Segment>>| {
            let l0_units: Vec<UnitRange> = segs.iter().map(segment_to_unit).collect();
            let moves_l0: Vec<LeveledMove> = l0_units.iter().enumerate()
                .map(|(i,u)| LeveledMove::from_unit(u, recursive_tower::ElementId{level:0,ordinal:i as u64})).collect();
            let (c, upper, _) = recursive_tower::compose_level(&l0_units, &moves_l0, true, 1);
            recursive_tower::project_to_units(&upper, &decompose::decompose(&c))
        };
        assert_eq!(mk_l1_units(&layer_v1.segments), mk_l1_units(&layer_v2.segments),
            "前提：L1 投影输入 v1==v2（守卫的本级投影比对看不到此变异）");

        // 共享 cache：先喂 v1（缓存 L0/L1），再喂 v2（frontier 内点改写）——模拟 per-bar 末段重划。
        let mut cache = TowerCache::new();
        let _ = classify_with_tower_incremental(&layer_v1, &cfg, &mut cache);
        let (inc_v2, inc_tower_v2) = classify_with_tower_incremental(&layer_v2, &cfg, &mut cache);
        let (full_v2, full_tower_v2) = classify_with_tower(&layer_v2, &cfg);

        // ★核心断言（latent 陈旧检测，非仅返回值）：cache 内 L1 深嵌套 sub_moves 末段坐标必须 == v2
        // 的 [112,140]。返回的 Classification/tower 不消费 cache 内 L1 upper_moves 的深 subs（tower 用
        // 新鲜 moves_tower 快照），故陈旧在返回值里 latent——但它喂 BSP（extract_second_for_level）+
        // 下一 bar 的 L2 投影。直接断言 cache 深 subs，捕获 latent 陈旧（640：不靠返回值碰巧相等）。
        // 推导（task #142 fixture）：v2 seg[8] = Up 112→140 ⟹ 区间 [112,140]（v1 为 [112,147]）。
        let l0_seg8_full = classify_with_tower(&layer_v2, &cfg).1[0].last().unwrap().rmove.clone();
        assert_eq!(l0_seg8_full, descend::RMove::Segment { direction: Direction::Up, lo: 112, hi: 140 },
            "前提：v2 全量 L0 末段 == [112,140]");
        // cache.L1.upper_moves[0].sub_moves[2](groupC).sub_moves[2](seg[8]) 应 == [112,140]。
        let l1_deep = &cache.levels[1].upper_moves[0].sub_moves[2].sub_moves[2].rmove;
        assert_eq!(*l1_deep, descend::RMove::Segment { direction: Direction::Up, lo: 112, hi: 140 },
            "cascade: cache L1 深嵌套 seg[8] == v2 [112,140]（陈旧则 [112,147]——L1 漏 cascade reset）");

        // 返回值也须 bit-exact（cascade 后 L1 重建，tower/Classification 全对齐）。
        assert_eq!(inc_v2, full_v2, "cascade: v2 增量 Classification == 全量");
        assert_eq!(inc_tower_v2.len(), full_tower_v2.len(), "cascade: tower 层数 == 全量");
        for (lvl, (il, fl)) in inc_tower_v2.iter().zip(full_tower_v2.iter()).enumerate() {
            assert_eq!(il, fl, "cascade: level {lvl} LeveledMove == 全量");
        }
    }

    /// ★on2w2 G7（epoch 递增覆盖性）：L0 尾段重划场景（= A12 bar3020 假命中场景）**必须** bump
    /// forest_epoch。复用 `cascade_reset_on_frontier_interior_rewrite` 的 v1→v2 fixture——v2 仅末段
    /// end_price 147→140（组 C 外缘内点，L1 投影 bit-identical，L0 sub_moves 变）。这是 gen 快路当年
    /// 假命中返陈旧森林的精确形态（TowerCache::generation 靠 l0_is_root blunt 兜底才没漏）。
    ///
    /// 断言：喂 v1 后喂 v2，forest_epoch **严格递增**（若 epoch 漏 bump ⟹ 下游 TreeCache 假命中返
    /// v1 陈旧森林 ⟹ bit-exact 破裂）。E1 逐值判据在此场景 [reuse..] 尾段值变（147→140）⟹ dirty。
    #[test]
    fn forest_epoch_bumps_on_l0_tail_redivision() {
        let cfg = ThetaConfig::default();
        let base = vec![
            seg(Direction::Up,   0,  4, 110, 150),
            seg(Direction::Down, 4,  8, 150, 120),
            seg(Direction::Up,   8, 12, 120, 148),
            seg(Direction::Down,12, 16, 115,  80),
            seg(Direction::Up,  16, 20,  80, 125),
            seg(Direction::Down,20, 24, 114,  85),
            seg(Direction::Up,  24, 28, 115, 148),
            seg(Direction::Down,28, 32, 148, 112),
            seg(Direction::Up,  32, 36, 112, 147), // v1 末段
        ];
        let mut closes: Vec<i64> = Vec::new();
        for i in 0..12 { closes.push(100 + if i % 2 == 0 { 40 } else { -40 }); }
        for i in 0..12 { closes.push(100 + if i % 2 == 0 {  5 } else {  -5 }); }
        for i in 0..16 { closes.push(100 + if i % 2 == 0 {  3 } else {  -3 }); }
        let merged = Rc::new(bars_from_closes(&closes));
        let layer_v1 = ParseLayer { segments: Rc::new(base.clone()), merged_bars: merged.clone(), ..Default::default() };
        let mut v2_segs = base.clone();
        v2_segs[8].end_price = 140; // L0 尾段重划（内点改写）——A12 bar3020 假命中场景。
        let layer_v2 = ParseLayer { segments: Rc::new(v2_segs), merged_bars: merged.clone(), ..Default::default() };

        let mut cache = TowerCache::new();
        let _ = classify_with_tower_incremental(&layer_v1, &cfg, &mut cache);
        let epoch_after_v1 = cache.forest_epoch();
        let _ = classify_with_tower_incremental(&layer_v2, &cfg, &mut cache);
        let epoch_after_v2 = cache.forest_epoch();
        assert!(
            epoch_after_v2 > epoch_after_v1,
            "L0 尾段重划（147→140）必须 bump forest_epoch（漏 bump ⟹ TreeCache 假命中返陈旧森林）：\
             v1_epoch={epoch_after_v1} v2_epoch={epoch_after_v2}"
        );
    }

    /// ★on2w2：无字节变更 bar（inclusion-only，L0 尾段值不变）**不** bump forest_epoch——O(n²) 消除
    /// 的机制（bump 率贴近 forest 真变率而非每 bar）。喂完全相同的 layer 两次，第二次 epoch 不应变。
    #[test]
    fn forest_epoch_stable_on_no_change() {
        let cfg = ThetaConfig::default();
        let base = vec![
            seg(Direction::Up,   0,  4, 110, 150),
            seg(Direction::Down, 4,  8, 150, 120),
            seg(Direction::Up,   8, 12, 120, 148),
            seg(Direction::Down,12, 16, 115,  80),
            seg(Direction::Up,  16, 20,  80, 125),
            seg(Direction::Down,20, 24, 114,  85),
            seg(Direction::Up,  24, 28, 115, 148),
        ];
        let mut closes: Vec<i64> = Vec::new();
        for i in 0..28 { closes.push(100 + if i % 2 == 0 { 30 } else { -30 }); }
        let merged = Rc::new(bars_from_closes(&closes));
        let layer = ParseLayer { segments: Rc::new(base.clone()), merged_bars: merged.clone(), ..Default::default() };

        let mut cache = TowerCache::new();
        let _ = classify_with_tower_incremental(&layer, &cfg, &mut cache);
        let e1 = cache.forest_epoch();
        // 完全相同输入再喂一次——无任何塔字节变更 ⟹ epoch 不应 bump。
        let _ = classify_with_tower_incremental(&layer, &cfg, &mut cache);
        let e2 = cache.forest_epoch();
        assert_eq!(e1, e2, "无变更 bar 不应 bump forest_epoch（否则退化每 bar bump = O(n²) 未消除）：e1={e1} e2={e2}");
    }

    // ──────────────────────────────────────────────────────────────────────
    //  标度验证（task #93：per-bar 累积成本，增量 vs 全量）
    // ──────────────────────────────────────────────────────────────────────

    /// ★合成标度：per-bar 段追加累积成本，增量 exp 显著 < 全量 exp。
    ///
    /// 全量 `classify_with_tower` 每步从 0 重扫塔 ⟹ 累积 O(Σ i) ≈ O(N²)，exp≈2。
    /// 增量 `classify_with_tower_incremental` 每步续扫 tail ⟹ 累积 O(Σ tail) ≈ O(N)，exp≈1。
    /// 合成段序列单调追加（增量有效域）；此测试 always-run（无需真实数据）。
    ///
    /// ## ⚠ 时间敏感（票 #619 L9 登记）
    ///
    /// 本测试的判据是**墙钟时间比**（`ratio_at_max < 0.7`），因而对机器负载敏感：并行跑
    /// 整个 `--lib` 套件、或机器同时在跑别的重活时，会偶发红（观测集群 ~0.55，余量 ~0.14）。
    /// **隔离单跑恒绿**——复现红时的正确处置是
    /// `cargo test --release --lib incremental_tower_scaling_dominates_full_synthetic`
    /// 单独重跑确认，而不是改阈值（阈值的因果标定见下方长注释，`no-patch-mentality` 合规）。
    ///
    /// 这是**该测试的固有属性**，不是回归信号，**与 #491**（`extract_signals_bit_exact_digest_guard`
    /// 的确定性红）**无关**：#491 是字节摘要不符、恒红且与负载无关；本测试是负载相关的偶发红。
    /// 此前该性质只散落在 4 份评审报告的自然语言里（`frontier-had-emitted-window-20260702.md:52`
    /// 首次定性、`shadow-review-389-20260727.md:111`、`treasury-reverify-20260727.md:504`、
    /// `shadow-603-review-20260728.md` §6），测试本体无注记 ⟹ 登记口径与 #491 不对称。本注记
    /// 补齐该不对称的测试本体侧。
    #[test]
    fn incremental_tower_scaling_dominates_full_synthetic() {
        let cfg = ThetaConfig::default();
        let sizes = [100usize, 200, 400];
        let mut full_times = Vec::new();
        let mut inc_times = Vec::new();

        for &n in &sizes {
            let all_segments = synthetic_segments(n);
            let closes: Vec<i64> = (0..(n * 4 + 8) as i64).map(|i| 100 + (i % 7) * 4).collect();

            // 全量 per-bar 累积。
            let t0 = std::time::Instant::now();
            for k in 1..=n {
                let layer = ParseLayer {
                    segments: Rc::new(all_segments[..k].to_vec()),
                    merged_bars: Rc::new(bars_from_closes(&closes)),
                    ..Default::default()
                };
                let _ = classify_with_tower(&layer, &cfg);
            }
            full_times.push(t0.elapsed().as_secs_f64());

            // 增量 per-bar 累积（cache 跨步复用）。
            let t0 = std::time::Instant::now();
            let mut cache = TowerCache::new();
            for k in 1..=n {
                let layer = ParseLayer {
                    segments: Rc::new(all_segments[..k].to_vec()),
                    merged_bars: Rc::new(bars_from_closes(&closes)),
                    ..Default::default()
                };
                let _ = classify_with_tower_incremental(&layer, &cfg, &mut cache);
            }
            inc_times.push(t0.elapsed().as_secs_f64());
        }

        // exp 估计（log-log 斜率，sizes 翻倍）。
        let full_exp = (full_times[2] / full_times[0]).ln() / (sizes[2] as f64 / sizes[0] as f64).ln();
        let inc_exp = (inc_times[2] / inc_times[0]).ln() / (sizes[2] as f64 / sizes[0] as f64).ln();

        eprintln!(
            "\n===== 增量塔标度（合成 per-bar 累积）=====\n  \
             sizes={sizes:?}\n  full_times={full_times:?} (exp≈{full_exp:.2})\n  \
             inc_times={inc_times:?} (exp≈{inc_exp:.2})\n  \
             增量/全量比 @n={}: {:.2}x（越小增量越优）",
            sizes[2],
            inc_times[2] / full_times[2].max(1e-12)
        );

        // 增量须显著快于全量（MACD 增量 + 塔构造增量 + 走势分解增量 综合加速）。
        // ★判据：最大规模下增量/全量时间比 < 0.7（即增量至少 ~1.43x 加速）为稳健下界。
        // exp 差距在小规模 debug 噪声大（两者均 O(n²) 受限于 LevelState/tower_snapshots clone
        // 的 API 所需 O(k)/iter，故此合成尺度只能验证常数因子优势，asymptotic 分离须看
        // profile_incremental_tower_real_scaling 的真实大规模 #[ignore]）。此处验证常数因子：
        // 增量消除 MACD 全量重算 + 塔构造全量扫描。
        // 标度重标定（B4 / task#2，commit 254 改调 extract_signals_with_hist）：MACD 消重后
        // 全量只做 1×MACD（原 2×），增量相对优势从 >2x 收窄到 ~1.8x（ratio 实测集群
        // 0.543/0.548/0.559/0.55 across runs）。原阈值 0.5 按 full=2×MACD 标定，1×MACD 后需
        // 重标；取 0.7 为稳健下界（观测集群 ~0.55，留 ~0.14 机器噪声余量，仍断言真常数因子优势——
        // 若增量退化到无优势 ratio→1.0 则捕获）。这是因果重标定非「为绿改阈值」（no-patch 合规）。
        let ratio_at_max = inc_times[2] / full_times[2].max(1e-12);
        assert!(
            ratio_at_max < 0.7,
            "增量/全量比 @n={} = {ratio_at_max:.3} 须 < 0.7（增量至少 ~1.43x 加速；MACD+塔+分解增量）\n\
             full_exp≈{full_exp:.2}, inc_exp≈{inc_exp:.2}",
            sizes[2]
        );
    }
}

#[cfg(test)]
mod incremental_profile {
    //! 增量塔真实数据标度 profile（#[ignore]，需真实数据 + release）。
    use super::*;
    use super::super::backtest::data;
    use super::super::parser;

    /// ★真实数据 per-bar 标度：CL 真实段账本逐段追加，增量 vs 全量累积成本 + exp。
    ///
    /// 真实段账本单调追加（parser 前缀稳定语义）⟹ 增量有效域命中。验证真实数据下增量 exp≈1
    /// 而全量 exp≈2（塔构造超线性消除）。L2 经验标度（formalization-validity-domain 231号）。
    #[test]
    #[ignore = "真实数据标度 profile：需 CL 数据；--release（per-bar 双跑对照）"]
    fn profile_incremental_tower_real_scaling() {
        let cfg = ThetaConfig::default();
        let ds = match data::load_by_symbol("CL", &cfg) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("DATA BLOCKER: {e}");
                panic!("需真实数据");
            }
        };
        let oos = ds.slice_date_window("2023-01-01", "2025-06-30");
        // ponytail: 大规模标度验证 acceptance[4]——全引擎 per-bar exp≈1.0 @16K。
        // a7dfec46: n<5000 不可靠；此处用 [5K, 10K, 16K] 真实大规模。
        let sizes = [5_000usize, 10_000, 16_000];
        let mut full_times = Vec::new();
        let mut inc_times = Vec::new();
        let mut used_sizes = Vec::new();

        for &n in &sizes {
            if n > oos.bars.len() {
                eprintln!("DATA LIMIT: n={n} > oos.bars.len()={}，跳过", oos.bars.len());
                break;
            }
            let bars = &oos.bars[..n];

            // 全量 per-bar 累积。
            let t0 = std::time::Instant::now();
            for i in 50..n {
                let l0 = parser::parse_layer(&bars[..i], &cfg);
                let _ = classify_with_tower(&l0, &cfg);
            }
            full_times.push(t0.elapsed().as_secs_f64());

            // 增量 per-bar 累积。
            let t0 = std::time::Instant::now();
            let mut cache = TowerCache::new();
            for i in 50..n {
                let l0 = parser::parse_layer(&bars[..i], &cfg);
                let _ = classify_with_tower_incremental(&l0, &cfg, &mut cache);
            }
            inc_times.push(t0.elapsed().as_secs_f64());
            used_sizes.push(n);
            eprintln!("n={n} done: full={:.2}s inc={:.2}s", *full_times.last().unwrap(), *inc_times.last().unwrap());
        }

        // 逐相邻对算 exp（log-log 斜率），大规模验证 acceptance[4]。
        eprintln!("\n===== 增量塔真实标度（CL per-bar 累积，大规模）=====");
        for w in used_sizes.windows(2) {
            let (n0, n1) = (w[0], w[1]);
            let i0 = used_sizes.iter().position(|&s| s == n0).unwrap();
            let i1 = i0 + 1;
            let full_exp = (full_times[i1] / full_times[i0].max(1e-12)).ln()
                / (n1 as f64 / n0 as f64).ln();
            let inc_exp = (inc_times[i1] / inc_times[i0].max(1e-12)).ln()
                / (n1 as f64 / n0 as f64).ln();
            eprintln!(
                "  [{n0}→{n1}] full exp≈{full_exp:.2}  inc exp≈{inc_exp:.2}  \
                 增量/全量比 @{n1}: {:.2}x",
                inc_times[i1] / full_times[i1].max(1e-12)
            );
        }
    }
}
