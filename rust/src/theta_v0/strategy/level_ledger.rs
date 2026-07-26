//! ★LEE M1 级别账本只读旁路 `LevelLedgerMirror`（multi-level-native-execution-design-20260719
//! §C.1 支柱1 / §C.2 不变量 / §D 迁移表 M1 行；lee-integration-points-20260720 (c) 最小实装面①）。
//!
//! ## 本文件做什么：OverlayState 的同一份 `SepLeg` 暴露按级别分桶的只读镜像账本 Ledger_ℓ
//!
//! M1 定义（设计文档 §D 迁移表逐字）：「OverlayState 的 SepLeg 按 formation_level 分组，建只读
//! Ledger_ℓ 镜像；不改净额主路径，bit-exact」。级别口径：`formation_level ≡ SepLeg.id.level`
//! （塔级别 = BSP 出现级别——集成点文档 (b) 判定：沿用既有 `level` 字段语义，**不引入
//! `formation_level` 新字段名**，roadmap:60「不得重命名后偷换口径」纪律）。账本行复用
//! [`overlay_state::VoiceBook`]（hedge-mode 逐声部簿行，设计文档 §C.1 支柱1 点名「账本行可复用」）；
//! 分桶键 = `id.level`（[`ElementId`] 已含级别，**零 schema 改动**，集成点文档 (b)「按级别分桶
//! 不需要 source_index/point_class」）。`BTreeMap` 保确定序（bit-exact 可复现，同
//! `settle_forced_virtual` 的确定序纪律）。
//!
//! ## 不变量（设计文档 §C.2，M1 验收锚）
//!
//! - **LEE-Net 恒等**：`Σ_ℓ net_ℓ ≡ N`，其中 `net_ℓ = Σ_{v∈ℓ} σ_v q_v`、N = overlay 净敞口——
//!   逐级分解是 Net 的**加性细化**（线性代数，认识论 L1，同 `overlay_state.rs` §11 Fubini 对账
//!   一类）。q_v 是整数手数，整数求和无浮点结合律问题 ⟹ 逐 bar **精确**成立（非 eps 容差）。
//! - **级别封闭**：`Ledger_ℓ` 只含 `id.level=ℓ` 的行；跨级影响（如父级失效要求子级强平）必须以
//!   显式跨级消息落账（M3+ 的事）——本镜像无跨级传导，`parent_id` 仅作元数据保留不落账。
//! - **只读旁路 bit-exact**：本镜像与 [`OverlayState`] 并列消费同一 `StepTrace.sep_legs`，不改
//!   净额主路径（cash/units/trade_pnls/equity）；镜像行与 overlay 行**逐字段 bit-equal**
//!   （同公式、同 ΔP 序、同取整规则逐声部重放——D8 逐字节对拍协议，dual_ledger.rs:440-525 模板）。
//!
//! ## M1 明确不做（设计文档 §D + 集成点文档 (c)「明确排除」逐字）
//!
//! 不动 `pi_theta_step`/净额主路径；不动 `newly_confirmed_step` 的全级别混合 diff（clock_ℓ 拆分
//! 是 M3 事件门控的事，M1 仍每决策点重估）；不动 `leg_target` 权重公式（级别 sizing 是 M4）；
//! 不引入 `formation_level` 新字段名；不碰 nautilus 生产路径。本镜像**不产订单**（Consume_ℓ
//! 独立目标计算是 M2 以后的事）——只证加性细化恒等。
//!
//! ## 风险点（集成点文档 (c) 照实登记，M1 口径声明）
//!
//! ① registry restore 注入的祖先腿按**自身** `id.level` 落桶（其 id 入场时定型，`level` 语义
//! 不变）；② 幽灵腿（gross_zeroed）`q_units` 已零化 ⟹ 取整为 0 ⟹ 两账一致剔除，不产生分桶偏差。
//!
//! ## 认识论等级（formalization-validity-domain / 231号）
//!
//! **L1**（管线正确性）：LEE-Net 恒等是构造性结构恒等（整数加性分解），零信息增量，
//! **不声明 alpha**（同 overlay_state.rs 的认识论声明）。

use std::collections::BTreeMap;

use super::coverage::{SepLeg, Vertical};
use super::overlay_state::{side_sign, ClosedVoice, VoiceBook};
use super::voice::VoiceSide;
use crate::theta_v0::classifier::recursive_tower::ElementId;

/// P^sep 目标的按「级别 → 声部」归并表（`level → ordinal → (side, q_lots, role_v, parent_id)`）。
pub(crate) type LevelTargets =
    BTreeMap<u32, BTreeMap<u64, (VoiceSide, i64, Vertical, Option<ElementId>)>>;

/// ★P^sep 目标构造**单源**（[`LevelLedgerMirror::step`] ② 与 [`level_nets`] 共用）：
/// `q_v` 取整为手数（`<lot/2` 归 0 ⟹ 剔除，诚实退化，与 [`OverlayState`] 同口径）；同 carrier
/// 多条 `SepLeg`（`next_active` 的 `ElementId` 唯一 ⟹ 不应发生）取**首条方向**、q 累加
/// （防御性，同 overlay）。分桶键 = `id.level` ≡ formation_level（M1 口径，零 schema 改动）。
pub(crate) fn build_level_targets(sep_legs: &[SepLeg], lot: i64) -> LevelTargets {
    let lot = lot.max(1);
    let mut target: LevelTargets = BTreeMap::new();
    for leg in sep_legs {
        let q = (leg.q_units / lot as f64).round() as i64 * lot;
        if q <= 0 {
            continue; // 未达最小手数 ⟹ 不进 P^sep（诚实退化，与 overlay 同口径）
        }
        let entry = target
            .entry(leg.id.level)
            .or_default()
            .entry(leg.id.ordinal)
            .or_insert((leg.side, 0, leg.role_v, leg.parent_id));
        entry.1 += q;
    }
    target
}

/// ★各级结构净额 `net_ℓ = Σ_{v∈ℓ} σ_v q_v`（level 升序表，`Vec` 保确定序）——LEE M2 归因基准。
///
/// 与 [`LevelLedgerMirror::step`] 的 `nets` **构造性同源**：两者都是同一 [`build_level_targets`]
/// 输出的按级折叠（镜像 rebalance 后各级簿的键与 `(side,q)` 恒等于目标表 ⟹ 折叠结果逐项相等）。
/// 单源保证 M2 的归因基准与 M1 的镜像账本不会各自漂移（无第二裁决源）。
pub fn level_nets(sep_legs: &[SepLeg], lot: i64) -> Vec<(u32, i64)> {
    build_level_targets(sep_legs, lot)
        .into_iter()
        .map(|(lvl, book)| {
            (lvl, book.values().map(|(side, q, _, _)| side_sign(*side) * q).sum::<i64>())
        })
        .collect()
}

/// ★LEE-Net 恒等的 **release 可见**见证读数（#289 影子评审 MED ①）。
///
/// M1 的逐 bar 恒等只落 `debug_assert` ⟹ release 跑批零执行；integration 断言取 `force_flat`
/// 之后（两账均已归零）⟹ 平凡通过。本读数在 fill loop 内**每决策点**累计，release 下同样执行：
/// 「`max_abs_residual == 0` **且** `max_abs_net > 0`」才构成非平凡证据（前者恒等、后者非空转）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LeeNetWitness {
    /// 观测到的决策点数（分母）。
    pub n_observations: u64,
    /// `max |Σ_ℓ net_ℓ − N|`（LEE-Net 恒等残差，应**恒 0**——整数手数求和，非 eps 容差）。
    pub max_abs_residual: i64,
    /// `max |N|`（**非平凡性证据**：>0 才说明恒等不是在空账上平凡成立）。
    pub max_abs_net: i64,
}

impl LeeNetWitness {
    /// 恒等见证成立：残差恒 0 **且** 曾出现非零净敞口（`max|N| > 0`）。
    pub fn identity_witnessed(&self) -> bool {
        self.max_abs_residual == 0 && self.max_abs_net > 0
    }
}

/// [`LevelLedgerMirror::step`] 单步产出（LEE-Net 恒等见证）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelLedgerStep {
    /// `Σ_ℓ net_ℓ`（rebalance 后各级净敞口之和；与 overlay `N_t` 逐决策点对拍的恒等左端）。
    pub total_net: i64,
    /// 本步末活动级别桶数（账非空的级别数；级别稀疏性诊断读数——M3 稀疏性的 M1 形态前导）。
    pub n_active_levels: usize,
}

/// ★LEE M1 级别账本镜像 `LevelLedgerMirror`（只读旁路 Ledger_ℓ）。
///
/// 结构：`books[ℓ][ordinal] = VoiceBook`（级别 → 该级活动声部簿）；`closed[ℓ]` = 该级已离场
/// 声部归因表；`nets[ℓ] = Σ_{v∈ℓ} σ_v q_v`（冗余缓存 = books 派生，守恒断言锚，同
/// [`OverlayState`] 的 `net` 缓存纪律）。
///
/// 逐决策点 [`LevelLedgerMirror::step`]：消费与 [`OverlayState::step`] **同一份** `sep_legs` +
/// px + bar + lot，按 `id.level` 分桶重放同一 rebalance 语义（① 价格 PnL 累计 → ② rebalance
/// → ③ net_ℓ 派生），产 LEE-Net 恒等左端 `Σ_ℓ net_ℓ`。
#[derive(Debug, Clone, Default)]
pub struct LevelLedgerMirror {
    /// 各级活动声部簿（Ledger_ℓ 的活动腿；外键=级别，内键=`id.ordinal`——`ElementId` 无 `Ord`，
    /// 级别已由外键承载，内键 ordinal 即唯一确定声部）。
    books: BTreeMap<u32, BTreeMap<u64, VoiceBook>>,
    /// `net_ℓ = Σ_{v∈ℓ} σ_v q_v`（各级净敞口，冗余缓存 = books 派生）。
    nets: BTreeMap<u32, i64>,
    /// 各级已离场声部归因表（exit_v/pnl_v 落盘，按级分桶）。
    closed: BTreeMap<u32, Vec<ClosedVoice>>,
    /// 上一决策点收盘价（价格 PnL 累计的 P_{t−1}）；None=首步（无 ΔP 可累计）。
    last_px: Option<f64>,
    /// LEE-Net 恒等的 release 可见见证（#289 MED ①；由 [`observe_lee_net`](Self::observe_lee_net) 累计）。
    witness: LeeNetWitness,
}

impl LevelLedgerMirror {
    pub fn new() -> Self {
        LevelLedgerMirror::default()
    }

    /// `Σ_ℓ net_ℓ`（LEE-Net 恒等左端；与 overlay `net()` 对拍）。
    pub fn total_net(&self) -> i64 {
        self.nets.values().sum()
    }

    /// 级别 ℓ 的净敞口 `net_ℓ`（无活动簿的级别 = 0）。
    pub fn net_at(&self, level: u32) -> i64 {
        self.nets.get(&level).copied().unwrap_or(0)
    }

    /// 各级净敞口表（只读；`net_ℓ` 派生缓存）。
    pub fn nets(&self) -> &BTreeMap<u32, i64> {
        &self.nets
    }

    /// ★LEE-Net 恒等的逐决策点见证登记（#289 MED ①，**release 下同样执行**）：
    /// 累计 `max |Σ_ℓ net_ℓ − N|` 与 `max |N|`。调用点 = fill loop 内镜像 step 紧后
    /// （与 M1 的 `debug_assert` 同锚同时点；断言只在 debug 跑，本读数在 release 也跑）。
    pub fn observe_lee_net(&mut self, overlay_net: i64) {
        let total = self.total_net();
        let w = &mut self.witness;
        w.n_observations += 1;
        w.max_abs_residual = w.max_abs_residual.max((total - overlay_net).abs());
        w.max_abs_net = w.max_abs_net.max(overlay_net.abs());
    }

    /// LEE-Net 恒等见证读数（release 可见；见 [`LeeNetWitness`]）。
    pub fn lee_net_witness(&self) -> LeeNetWitness {
        self.witness
    }

    /// 有活动簿的级别（升序，确定序）。
    pub fn levels(&self) -> impl Iterator<Item = u32> + '_ {
        self.books.keys().copied()
    }

    /// 级别 ℓ 的活动声部簿（只读；逐声部持仓归因）。
    pub fn active_voices(&self, level: u32) -> impl Iterator<Item = &VoiceBook> {
        self.books.get(&level).into_iter().flat_map(|b| b.values())
    }

    /// 级别 ℓ 的已离场声部归因表（exit_v/pnl_v）。
    pub fn closed_voices(&self, level: u32) -> &[ClosedVoice] {
        self.closed.get(&level).map_or(&[], Vec::as_slice)
    }

    /// 活动声部总数（跨级求和；与 overlay `active_voices().count()` 对拍的分区完备锚）。
    pub fn n_active(&self) -> usize {
        self.books.values().map(BTreeMap::len).sum()
    }

    /// 已离场声部总数（跨级求和；与 overlay `closed_voices().len()` 对拍的分区完备锚）。
    pub fn n_closed(&self) -> usize {
        self.closed.values().map(Vec::len).sum()
    }

    /// ★逐决策点步进（与 [`OverlayState::step`] 同序同式，按 `id.level` 分桶重放）：
    /// 目标 P^sep（`sep_legs`）+ 当前价 px + bar 序号 + lot → ① 价格 PnL 累计（rebalance 前
    /// 持有的 q_v + ΔP）→ ② rebalance 各级簿到目标（新开/离场/resize，同取整同防御口径）→
    /// ③ `net_ℓ = Σ_{v∈ℓ} σ_v q_v` 派生，产 `Σ_ℓ net_ℓ`（LEE-Net 恒等左端）。
    ///
    /// **本镜像不产订单**（M1 只读：Consume_ℓ 独立目标计算是 M2 以后的事）。
    pub fn step(&mut self, sep_legs: &[SepLeg], px: f64, bar: usize, lot: i64) -> LevelLedgerStep {
        let lot = lot.max(1);
        // ── ① 价格 PnL 累计（rebalance 前持有的 q_v；ΔP=px−last_px；与 OverlayState::step ①
        //    逐声部同式同序——per-voice 累计互不交互 ⟹ 迭代序无关，逐行 bit-equal 成立）。 ──
        if let Some(lp) = self.last_px {
            let dp = px - lp;
            if dp != 0.0 {
                for book in self.books.values_mut() {
                    for b in book.values_mut() {
                        b.pnl_v += side_sign(b.side) as f64 * b.q as f64 * dp;
                    }
                }
            }
        }
        self.last_px = Some(px);

        // ── ② rebalance 各级簿 → 目标 P^sep（按 id.level 分桶）。 ──
        // 目标构造走 [`build_level_targets`] 单源（与 M2 归因基准 [`level_nets`] 同一函数 ⟹
        // 两者不会各自漂移；口径细则见该函数注释）。
        let target = build_level_targets(sep_legs, lot);

        // 离场：books 中不在 target 的声部 → 记 exit_v/冻结 pnl_v，移出该级簿。
        for (lvl, book) in self.books.iter_mut() {
            let gone: Vec<u64> = match target.get(lvl) {
                Some(t) => book.keys().filter(|ord| !t.contains_key(ord)).copied().collect(),
                None => book.keys().copied().collect(),
            };
            for ord in gone {
                let b = book.remove(&ord).expect("gone 来自 book.keys");
                self.closed.entry(*lvl).or_default().push(ClosedVoice {
                    id: b.id,
                    side: b.side,
                    role_v: b.role_v,
                    parent_id: b.parent_id,
                    entry_bar: b.entry_bar,
                    exit_bar: bar,
                    entry_px: b.entry_px,
                    exit_px: px,
                    pnl_v: b.pnl_v,
                });
            }
        }
        // 账空的级别桶剔除（级别不活跃 ⟹ 无桶；net_ℓ 由下方派生段重建，缺席=0）。
        self.books.retain(|_, book| !book.is_empty());

        // 开仓 / resize：target 声部映射进各级簿（entry_px 手数加权同 overlay；减仓保 entry_px）。
        for (lvl, tbook) in &target {
            let book = self.books.entry(*lvl).or_default();
            for (ord, (side, q, role_v, parent_id)) in tbook {
                match book.get_mut(ord) {
                    Some(b) => {
                        if *q > b.q {
                            let added = (*q - b.q) as f64;
                            b.entry_px = (b.entry_px * b.q as f64 + px * added) / *q as f64;
                        }
                        b.q = *q;
                        b.side = *side; // 恒等（防御性覆盖，同 overlay）
                    }
                    None => {
                        book.insert(*ord, VoiceBook {
                            id: ElementId { level: *lvl, ordinal: *ord },
                            side: *side,
                            q: *q,
                            role_v: *role_v,
                            parent_id: *parent_id,
                            entry_bar: bar,
                            entry_px: px,
                            pnl_v: 0.0,
                        });
                    }
                }
            }
        }

        // ── ③ net_ℓ = Σ_{v∈ℓ} σ_v q_v（冗余缓存 = books 派生）；total = Σ_ℓ net_ℓ（LEE-Net 左端）。 ──
        self.nets = self
            .books
            .iter()
            .map(|(lvl, book)| (*lvl, book.values().map(|b| side_sign(b.side) * b.q).sum()))
            .collect();
        let total_net = self.nets.values().sum();
        LevelLedgerStep { total_net, n_active_levels: self.books.len() }
    }

    /// 窗口终点强平（含浮盈口径）：全部级别全部活动声部按末价 px 离场（记 exit_v/冻结 pnl_v）。
    /// 价格 PnL 已在最后一步 [`step`](Self::step) 累计到 px ⟹ 此处只搬账本行（不再累计 ΔP）。
    /// 各级 net_ℓ 归零（与 [`OverlayState::force_flat`] 同语义，按级重排）。
    pub fn force_flat(&mut self, px: f64, bar: usize) {
        for (lvl, book) in self.books.iter_mut() {
            let ords: Vec<u64> = book.keys().copied().collect();
            for ord in ords {
                let b = book.remove(&ord).expect("ords 来自 book.keys");
                self.closed.entry(*lvl).or_default().push(ClosedVoice {
                    id: b.id,
                    side: b.side,
                    role_v: b.role_v,
                    parent_id: b.parent_id,
                    entry_bar: b.entry_bar,
                    exit_bar: bar,
                    entry_px: b.entry_px,
                    exit_px: px,
                    pnl_v: b.pnl_v,
                });
            }
        }
        self.books.clear();
        self.nets.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::coverage::{SepLeg, Vertical};
    use super::super::overlay_state::OverlayState;
    use std::collections::HashMap;

    fn eid(level: u32, ordinal: u64) -> ElementId {
        ElementId { level, ordinal }
    }

    fn leg(id: ElementId, side: VoiceSide, q_units: f64, role_v: Vertical) -> SepLeg {
        SepLeg { id, side, q_units, role_v, parent_id: None }
    }

    /// ★LEE-Net 恒等（M1 验收锚，设计文档 §C.2）：多级别持仓下逐 step `Σ_ℓ net_ℓ == overlay N`——
    /// 开（L1 多10 + L2 多6 + L3 空4）→ resize/平 L3 → 全平，每步恒等精确成立（整数手数求和）。
    #[test]
    fn lee_net_identity_holds_per_step_multi_level() {
        let mut ov = OverlayState::new();
        let mut ll = LevelLedgerMirror::new();
        // t0：三级同开（L1 多 10 + L2 多 6 + L3 空 4）⟹ N = 10+6−4 = 12。
        let t0 = [
            leg(eid(1, 0), VoiceSide::Long, 10.0, Vertical::Ambient),
            leg(eid(2, 0), VoiceSide::Long, 6.0, Vertical::FollowParent),
            leg(eid(3, 0), VoiceSide::Short, 4.0, Vertical::FollowParent),
        ];
        let so = ov.step(&t0, 100.0, 0, 1);
        let sl = ll.step(&t0, 100.0, 0, 1);
        assert_eq!(sl.total_net, so.net_after, "LEE-Net 恒等 @t0：Σ_ℓ net_ℓ == N_t");
        assert_eq!(sl.total_net, ov.net());
        assert_eq!(ll.net_at(1), 10);
        assert_eq!(ll.net_at(2), 6);
        assert_eq!(ll.net_at(3), -4);
        assert_eq!(ll.net_at(4), 0, "无持仓级别 net_ℓ=0");
        assert_eq!(sl.n_active_levels, 3, "3 个级别桶活跃");
        // t1：L1 加仓到 15、L3 全平（目标缺席）、L2 不动；px 101（ΔP=+1 先累计再 rebalance）。
        let t1 = [
            leg(eid(1, 0), VoiceSide::Long, 15.0, Vertical::Ambient),
            leg(eid(2, 0), VoiceSide::Long, 6.0, Vertical::FollowParent),
        ];
        let so = ov.step(&t1, 101.0, 1, 1);
        let sl = ll.step(&t1, 101.0, 1, 1);
        assert_eq!(sl.total_net, so.net_after, "LEE-Net 恒等 @t1");
        assert_eq!(sl.total_net, ov.net());
        assert_eq!(ll.net_at(1), 15);
        assert_eq!(ll.net_at(2), 6);
        assert_eq!(ll.net_at(3), 0, "L3 已平 ⟹ 桶清除");
        assert_eq!(sl.n_active_levels, 2);
        // 离场分区：L3 声部进 closed[3]，与 overlay.closed 分区完备。
        assert_eq!(ll.closed_voices(3).len(), 1, "L3 离场行进 closed[3] 桶");
        assert_eq!(ll.closed_voices(3)[0].id.level, 3, "级别封闭：closed[3] 只含 id.level=3");
        assert_eq!(ll.n_closed(), ov.closed_voices().len(), "closed 按级分区完备");
        // t2：全平（空目标）⟹ 双账归零。
        let so = ov.step(&[], 102.0, 2, 1);
        let sl = ll.step(&[], 102.0, 2, 1);
        assert_eq!(so.net_after, 0);
        assert_eq!(sl.total_net, 0);
        assert_eq!(sl.total_net, ov.net(), "LEE-Net 恒等 @t2（归零）");
        assert_eq!(ll.n_active(), 0);
        assert_eq!(sl.n_active_levels, 0);
        assert_eq!(ll.n_closed(), ov.closed_voices().len(), "全平后 closed 分区完备");
        assert_eq!(ll.n_closed(), 3, "3 声部全部离场");
    }

    /// ★级别封闭 + 跨级父子（风险点①：restore 注入祖先腿按**自身** id.level 落桶）：
    /// 子腿 id.level=3、parent_id.level=2 ⟹ 子落 L3 桶；parent(v) 仅元数据保留，不落账。
    #[test]
    fn level_closure_cross_level_parent_buckets_by_own_level() {
        let mut ll = LevelLedgerMirror::new();
        let parent = eid(2, 0);
        let mut child = leg(eid(3, 0), VoiceSide::Long, 5.0, Vertical::FollowParent);
        child.parent_id = Some(parent);
        ll.step(&[child], 100.0, 0, 1);
        assert_eq!(ll.net_at(3), 5, "子腿按自身 id.level=3 落桶");
        assert_eq!(ll.net_at(2), 0, "父缺席不落桶（镜像是 sep_legs 暴露的按级重排，不补父）");
        let rows: Vec<&VoiceBook> = ll.active_voices(3).collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id.level, 3, "级别封闭：L3 桶只含 id.level=3 的行");
        assert_eq!(rows[0].parent_id, Some(parent), "parent(v) 跨级引用作元数据保留（不落账）");
        assert!(ll.active_voices(2).next().is_none());
    }

    /// ★幽灵腿/不足手数口径一致（风险点②）：q_units=0.4（gross_zeroed 同形）round→0 ⟹
    /// 两账一致剔除，不产生分桶偏差。
    #[test]
    fn sub_lot_leg_excluded_identically_to_overlay() {
        let mut ov = OverlayState::new();
        let mut ll = LevelLedgerMirror::new();
        let g = [leg(eid(1, 0), VoiceSide::Long, 0.4, Vertical::Ambient)];
        let so = ov.step(&g, 100.0, 0, 1);
        let sl = ll.step(&g, 100.0, 0, 1);
        assert_eq!(so.net_after, 0, "0.4 手 round→0 ⟹ overlay 不进 P^sep");
        assert_eq!(sl.total_net, 0, "镜像同口径剔除");
        assert_eq!(ll.n_active(), 0);
        assert_eq!(sl.n_active_levels, 0);
        assert_eq!(ll.n_closed(), 0);
    }

    /// ★镜像行与 overlay 行逐字段 bit-equal（D8 逐字节对拍协议模板，dual_ledger.rs:440-525）：
    /// 多空对冲两级持仓 + 加仓 + 平子 + 窗口终点强平后，closed 行按级分区且与 overlay 行
    /// entry_px/exit_px/pnl_v **逐位相等**（同公式、同 ΔP 序、同取整逐声部重放 ⟹ 构造性 bit-equal）。
    #[test]
    fn mirror_rows_bit_equal_overlay_rows() {
        let mut ov = OverlayState::new();
        let mut ll = LevelLedgerMirror::new();
        let p = eid(2, 0); // 父：L2 多头
        let c = eid(1, 0); // 子：L1 ShortDiff 空头
        let t0 = [
            leg(p, VoiceSide::Long, 10.0, Vertical::FollowParent),
            leg(c, VoiceSide::Short, 6.0, Vertical::ShortDiff),
        ];
        ov.step(&t0, 100.0, 0, 1);
        ll.step(&t0, 100.0, 0, 1);
        ov.step(&t0, 110.0, 1, 1); // ΔP=+10
        ll.step(&t0, 110.0, 1, 1);
        // 父加仓到 12（entry_px 手数加权），子不动；ΔP=−5。
        let t2 = [
            leg(p, VoiceSide::Long, 12.0, Vertical::FollowParent),
            leg(c, VoiceSide::Short, 6.0, Vertical::ShortDiff),
        ];
        ov.step(&t2, 105.0, 2, 1);
        ll.step(&t2, 105.0, 2, 1);
        // 平子（目标缺席）；ΔP=+3。
        let t3 = [leg(p, VoiceSide::Long, 12.0, Vertical::FollowParent)];
        ov.step(&t3, 108.0, 3, 1);
        ll.step(&t3, 108.0, 3, 1);
        // 窗口终点强平（父离场）。
        ov.force_flat(109.0, 4);
        ll.force_flat(109.0, 4);
        // 分区完备 + 行级逐位对拍。
        assert_eq!(ll.n_closed(), ov.closed_voices().len(), "closed 按级分区完备");
        assert_eq!(ll.n_closed(), 2);
        let ov_rows: HashMap<ElementId, &ClosedVoice> =
            ov.closed_voices().iter().map(|r| (r.id, r)).collect();
        for lvl in [1u32, 2] {
            for m in ll.closed_voices(lvl) {
                assert_eq!(m.id.level, lvl, "级别封闭：closed[{lvl}] 只含 id.level={lvl}");
                let o = ov_rows[&m.id];
                assert_eq!(m.entry_px.to_bits(), o.entry_px.to_bits(), "entry_px bit-equal");
                assert_eq!(m.exit_px.to_bits(), o.exit_px.to_bits(), "exit_px bit-equal");
                assert_eq!(m.pnl_v.to_bits(), o.pnl_v.to_bits(), "pnl_v bit-equal（同式同序重放）");
                assert_eq!(m.entry_bar, o.entry_bar);
                assert_eq!(m.exit_bar, o.exit_bar);
                assert_eq!(m.side, o.side);
            }
        }
        // Σ_ℓ pnl 对账（PDF §11 Fubini 重排；此处逐行 bit-equal ⟹ 精确为 0 残差）。
        let sum_ll: f64 = [1u32, 2]
            .iter()
            .map(|&l| ll.closed_voices(l).iter().map(|r| r.pnl_v).sum::<f64>())
            .sum();
        let sum_ov: f64 = ov.closed_voices().iter().map(|r| r.pnl_v).sum();
        assert!((sum_ll - sum_ov).abs() < 1e-9, "Σ_ℓ closed pnl ≈ overlay Σ closed pnl");
        assert_eq!(ll.total_net(), ov.net(), "LEE-Net 恒等（强平后归零）");
        assert_eq!(ll.n_active(), 0);
    }

    /// ★M2 归因基准单源（[`level_nets`] ≡ [`LevelLedgerMirror::nets`]）：同一批 `sep_legs` 下
    /// 两者逐项相等——含多级、含同 carrier 重复腿（防御合并）、含 sub-lot 剔除三种形态。
    /// 单源保证 M2 的归因基准不会与 M1 的镜像账本各自漂移（无第二裁决源）。
    #[test]
    fn level_nets_matches_mirror_nets_single_source() {
        let batches: Vec<Vec<SepLeg>> = vec![
            vec![],
            vec![leg(eid(1, 0), VoiceSide::Long, 10.0, Vertical::Ambient)],
            vec![
                leg(eid(1, 0), VoiceSide::Long, 10.0, Vertical::Ambient),
                leg(eid(2, 0), VoiceSide::Long, 6.0, Vertical::FollowParent),
                leg(eid(3, 0), VoiceSide::Short, 4.0, Vertical::FollowParent),
            ],
            vec![
                // 同 carrier 重复腿（防御合并：取首条方向、q 累加）。
                leg(eid(1, 0), VoiceSide::Long, 4.0, Vertical::Ambient),
                leg(eid(1, 0), VoiceSide::Short, 6.0, Vertical::ShortDiff),
                // sub-lot 剔除（0.4 手 round→0）。
                leg(eid(2, 1), VoiceSide::Long, 0.4, Vertical::Ambient),
            ],
            vec![
                // 同级别多空对冲 ⟹ net_ℓ = 0（该级仍在表内，与镜像同）。
                leg(eid(2, 0), VoiceSide::Long, 5.0, Vertical::Ambient),
                leg(eid(2, 1), VoiceSide::Short, 5.0, Vertical::ShortDiff),
            ],
        ];
        for legs in &batches {
            let mut ll = LevelLedgerMirror::new();
            ll.step(legs, 100.0, 0, 1);
            let via_fn = level_nets(legs, 1);
            let via_mirror: Vec<(u32, i64)> = ll.nets().iter().map(|(&l, &q)| (l, q)).collect();
            assert_eq!(via_fn, via_mirror, "level_nets ≡ mirror.nets()（legs={legs:?}）");
        }
    }

    /// ★LEE-Net 见证非平凡（#289 MED ①）：空账下 `identity_witnessed` 必须 false（平凡通过
    /// 不算证据）；有非零净敞口且残差恒 0 时才成立。
    #[test]
    fn lee_net_witness_requires_nonzero_net() {
        let mut ll = LevelLedgerMirror::new();
        assert!(!ll.lee_net_witness().identity_witnessed(), "零观测 ⟹ 见证不成立");
        // 空账观测（N=0）：残差 0 但量级 0 ⟹ 仍不成立（正是 #289 指出的平凡通过）。
        ll.observe_lee_net(0);
        assert!(!ll.lee_net_witness().identity_witnessed(), "N≡0 的空账观测 ⟹ 见证仍不成立");
        // 真持仓观测：Σ_ℓ net_ℓ = 10 = N ⟹ 残差 0 且量级 10。
        ll.step(&[leg(eid(1, 0), VoiceSide::Long, 10.0, Vertical::Ambient)], 100.0, 1, 1);
        ll.observe_lee_net(10);
        let w = ll.lee_net_witness();
        assert_eq!(w.max_abs_residual, 0);
        assert_eq!(w.max_abs_net, 10);
        assert_eq!(w.n_observations, 2);
        assert!(w.identity_witnessed(), "残差 0 + max|N|>0 ⟹ 见证成立");
        // 违例可见：喂错 N ⟹ 残差非零被记录（见证转 false）。
        ll.observe_lee_net(7);
        assert_eq!(ll.lee_net_witness().max_abs_residual, 3, "残差如实记录，不吞");
        assert!(!ll.lee_net_witness().identity_witnessed());
    }

    /// ★防御口径一致（同 carrier 多条 SepLeg，不应发生）：取首条方向、q 累加——与
    /// [`OverlayState::step`] 的 target 合并语义逐字同构。
    #[test]
    fn same_carrier_duplicate_legs_accumulate_like_overlay() {
        let mut ov = OverlayState::new();
        let mut ll = LevelLedgerMirror::new();
        let dup = [
            leg(eid(1, 0), VoiceSide::Long, 4.0, Vertical::Ambient),
            leg(eid(1, 0), VoiceSide::Short, 6.0, Vertical::ShortDiff), // 同 carrier 第二条
        ];
        let so = ov.step(&dup, 100.0, 0, 1);
        let sl = ll.step(&dup, 100.0, 0, 1);
        assert_eq!(sl.total_net, so.net_after, "防御合并后仍 LEE-Net 恒等");
        assert_eq!(ll.net_at(1), 10, "q 累加 4+6=10，方向取首条 Long");
        assert_eq!(ll.n_active(), 1, "同 carrier 合并为 1 行");
    }

    /// ★目标不变 = 纯 hold（M1 形态：无事件 bar 的镜像读数——目标序列不变 ⟹ 账不动）：
    /// 同一目标重喂 ⟹ 零离场、零 resize、net 不变（M3 事件门控稀疏性的 M1 前导读数）。
    #[test]
    fn persistent_target_is_pure_hold() {
        let mut ll = LevelLedgerMirror::new();
        let t = [leg(eid(2, 0), VoiceSide::Long, 10.0, Vertical::FollowParent)];
        ll.step(&t, 100.0, 0, 1);
        let s = ll.step(&t, 100.0, 1, 1); // 同目标同价重喂
        assert_eq!(s.total_net, 10);
        assert_eq!(ll.n_closed(), 0, "目标不变 ⟹ 零离场");
        assert_eq!(ll.active_voices(2).next().expect("L2 在飞").q, 10, "q 不变");
        assert_eq!(ll.active_voices(2).next().expect("L2 在飞").entry_bar, 0, "entry_v 不重写");
    }

    /// ★窗口终点强平按级分区（与 [`OverlayState::force_flat`] 同语义按级重排）：
    /// 两级在飞 ⟹ force_flat 后活动全空、closed 分区完备、net 归零。
    #[test]
    fn force_flat_empties_all_levels_partition_complete() {
        let mut ov = OverlayState::new();
        let mut ll = LevelLedgerMirror::new();
        let t = [
            leg(eid(1, 0), VoiceSide::Long, 10.0, Vertical::Ambient),
            leg(eid(3, 0), VoiceSide::Short, 5.0, Vertical::FollowParent),
        ];
        ov.step(&t, 100.0, 0, 1);
        ll.step(&t, 100.0, 0, 1);
        ov.force_flat(101.0, 1);
        ll.force_flat(101.0, 1);
        assert_eq!(ll.total_net(), 0);
        assert_eq!(ll.total_net(), ov.net(), "LEE-Net 恒等（强平后）");
        assert_eq!(ll.n_active(), 0);
        assert_eq!(ll.closed_voices(1).len(), 1, "L1 强平行落 closed[1]");
        assert_eq!(ll.closed_voices(3).len(), 1, "L3 强平行落 closed[3]");
        assert_eq!(ll.n_closed(), 2);
        assert_eq!(ll.n_closed(), ov.closed_voices().len(), "closed 分区完备");
    }
}
