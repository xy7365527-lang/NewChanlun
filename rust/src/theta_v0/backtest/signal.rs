//! ★信号层 seam（B-M1，#88；#121 浅模块清理）：确认-bar 部署 + 入场结构止损，
//! 自 runner.rs 纯移动（设计 chanlun/plans/runner-rs-seam-designs-20260721.md §M1）。
//!
//! 对内经 `use super::signal::…` 供 runner/fill 消费；原函数在 runner 内全私有、
//! 无外部路径 ⟹ 无 `pub use` 门面。

use std::rc::Rc;

use super::super::classifier;

/// 买卖点身份判别 u8（seen-set append-only diff 键；6 类 bit 打包）。
///
/// 同一 `(level, source_index)` 上不同类买卖点（如 2买/3买 V 型可共存）是不同身份 ⟹ 入 bits 判别。
pub(crate) fn bsp_bits_disc(b: &super::super::types::BspBits) -> u8 {
    (b.buy1 as u8)
        | (b.buy2 as u8) << 1
        | (b.buy3 as u8) << 2
        | (b.sell1 as u8) << 3
        | (b.sell2 as u8) << 4
        | (b.sell3 as u8) << 5
}

/// ★[A] per-bar **确认-bar 部署**（修 bsp→订单转化；`recursive_t/stream.rs:273` 同构）。
///
/// 输入是**前缀因果分类** `classify_with_tower(l0[0..=i])`（639；非全窗）。返回本 bar **新确认**的
/// 买卖点（append-only diff vs `seen`，stream.rs 的 `seen_bsps`/"只增不改"语义）：`seen.insert(key)`
/// 为真（首次出现于前缀塔）⟹ 本 bar i 确认 ⟹ 保留 + 部署；已 seen ⟹ 跳过。
///
/// ## 为什么不是 `source_index==i`（旧错口径，零订单根因）
/// 买卖点**回溯确认**——其 `source_index`（触发 K 序）的端点常在**比 source_index 晚的 bar** 才被
/// 结构（中枢突破/线段确认/背驰）确认入前缀塔。按 `source_index==i` 切，当前 bar i 的前缀塔里
/// `source_index==i` 位置的买卖点**往往尚未确认**（host 仅 L0 / 候选父=∂ ⟹ Ambient 空切）⟹ 候选恒空
/// ⟹ **零订单**。改为"本 bar 新确认买卖点 diff"：买卖点在其被确认的那根 bar（其 source_index≤i）部署
/// （确认时点 = 因果，无 look-ahead；与 stream.rs「本 bar 新增 BSP 在当前 bar 投放」同构）。
///
/// 保留 `levels` 级别结构（与因果塔 `tower_i` 级别对齐）；`moves`/`centers` 空（σ_p 父容器方向由
/// `tower_i` 经 `assemble_gamma_with_tower` 的 `attach_bsp_to_tree` 查得，不读 classification moves/centers）。
///
/// ## 「至多响一次」的有效域 = 类型化身份，不是锚点（#361 核实，090）
///
/// 去重键是**三元组** `(level, source_index, bits_disc)`。同一锚点 `(level, source_index)` 因此
/// 可在**同一 bar** 发出多个点——[`bsp_bits_disc`] 文档所述「不同类买卖点可共存」的直接后果：
/// `classifier/signal.rs` 的 `make_first_point`/`make_second_point`/`make_third_point` 对同一锚点
/// 各产一个**单类** `BspPoint`，同 bar 一并入前缀塔。#361 在 **3500-bar / vol=4e7 一组** fixture 上
/// 实测（代码内可复现，见下方测试；其余 (n, vol) 的推广依据是上述构造子机制，非代码内实测）：
/// 锚点多响**全部同 bar**、**零**跨 bar 重分类样本（即不存在「弱分类先响、强分类后补响」）
/// ⟹ 判定为**合法语义**，不是同证据重复触发。任何在锚点粒度上声明
/// 「同身份至多响一次」的表述都过窄（历史表述见 `runner.rs::lee_m3_clock_obeys_first_observation_discipline`
/// 的 #361 订正段）。
///
/// ## 与 LEE `clock_ℓ` 的相交性（M3）
///
/// 本函数的输出**是** `clock_ℓ` 的 [`BspConfirmed`](super::super::strategy::level_clock::LevelEventKind::BspConfirmed)
/// 事件的唯一直接源（`strategy/level_clock.rs` 模块头对齐表第 1 行）。但事件门控**不**逐点触发：
/// `fill.rs` 只取「`bsp` 非空的级别下标」，再经 `LevelClockTicks` 的 `(level, kind)` BTreeSet 去重
/// ⟹ 同级同 bar 的 n 个点恒塌缩为**一个** tick。故锚点级多响既不会重复触发事件门控，也不会撑大
/// 稀疏性分母。
///
/// 该性质的回归守护 = `runner.rs::lee_m3_clock_obeys_first_observation_discipline` 的
/// `tick_inflation==0` 断言——该测试按**逐点**级别序列喂 `collect_ticks`（非本函数在 `fill.rs`
/// 侧已 distinct 的级别集），故 BTreeSet 去重路径真被走到、去重失效即转红（#361 影子评审
/// HIGH-1；同测的 `collapse_witness>0` 保证 fixture 里真出现过同 bar 同级多点）。
pub(crate) fn newly_confirmed_step(
    classification: &classifier::Classification,
    seen: &mut std::collections::HashSet<(usize, usize, u8)>,
) -> classifier::Classification {
    use super::super::classifier::LevelState;
    classifier::Classification {
        levels: classification
            .levels
            .iter()
            .enumerate()
            .map(|(lvl, ls)| LevelState {
                moves: Vec::new(),
                centers: Rc::new(Vec::new()),
                cp_ownership: Rc::new(Vec::new()),
                pan_div: Rc::new(Vec::new()), // Q4：新确认投影只携 bsp（盘整背驰承接在 econ 层，此处无消费者）
                // append-only：seen.insert 为真=本 bar 首次确认 ⟹ 保留；副作用把所有 bsp 标记 seen。
                bsp: ls
                    .bsp
                    .iter()
                    .filter(|p| seen.insert((lvl, p.source_index, bsp_bits_disc(&p.bits))))
                    .cloned()
                    .collect::<Vec<_>>()
                    .into(),
                level_projection: None, // #110 门关口径（默认零开销，bit-exact 不变）
            })
            .collect(),
    }
}

/// ★族A 修复：入场结构止损值（[`super::super::types::Tick`]，开仓 bar 一次性算 + 冻结到
/// [`LedgerOpen::entry_stop`]）。
///
/// 在决策 bar 因果分类 `classification` 上按候选 `(c.level, c.source_index)` 查 [`BspPoint`]——这是
/// **bsp 确认点坐标**（稳定，不漂移），方向由候选 `dir` 定（Long→pivot_low / Short→pivot_high，
/// 3 类→center）。`None` = structural_stop 返 None（非该方向交易点）/ BspPoint 缺失。
///
/// 与逐 bar 止损门 [`k_theta_risk_gate`] 共享同一 `structural_stop` 真值——本函数在**入场时**用候选
/// 坐标查得，冻结后供逐 bar 门读出（消除旧路径用 drifted `leg.source_index` 回查的覆盖度缺陷）。
pub(crate) fn entry_structural_stop(
    c: &super::super::strategy::interp::Candidate,
    classification: &super::super::classifier::Classification,
) -> Option<super::super::types::Tick> {
    use super::super::strategy::risk::{structural_stop, StopInput, StopSide};
    use super::super::strategy::voice::VoiceSide;
    use super::super::types::Center;
    let stop_side = match c.dir {
        VoiceSide::Long => StopSide::Long,
        VoiceSide::Short => StopSide::Short,
        VoiceSide::Flat => return None, // Flat 候选不开仓，无止损可言
    };
    let lvl = classification.levels.get(c.level as usize)?;
    let bsp = lvl.bsp.iter().find(|p| p.source_index == c.source_index)?;
    let stop_in = StopInput {
        pivot_low: bsp.pivot_low,
        pivot_high: bsp.pivot_high,
        // #218 面 A 载体形态消费面复核：Center 变体读出；二类点 Type1Anchor 载体无中枢——
        // 1/2 类止损只读 pivot（structural_stop 仅 3 类 bit 读 center），零占位无害（与
        // strategy/mod.rs build_decision 同一口径）。
        center: match bsp.center {
            Some(super::super::classifier::bsp::OwnerRef::Center(c)) => c,
            _ => Center { zd: 0, zg: 0, dd: 0, gg: 0, start_index: 0, end_index: 0 },
        },
    };
    structural_stop(stop_side, &bsp.bits, &stop_in)
}
