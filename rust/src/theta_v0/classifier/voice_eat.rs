//! C06 声部吃到判定 `Eat(v,b)`（完全分类推导 §3, 页3；spec
//! `.chanlun/specs/2026-06-28-complete-classification-pdf-extract.md` C06）。
//!
//! ## 定义（C06，逐字）
//!
//! `Eat(v,b)` ⟺ `∀t∈I_b, a_{v,t}=1 ∧ σ_v=ε_b ∧ q_{v,t}>0`
//!
//! 声部 v 吃到笔 b ⟺ 在整笔时间区间 `I_b=[λ_b,ρ_b)` 内，声部 v 始终满足三个条件：
//! 1. **活动** `a_{v,t}=1`：声部在 t 处于激活态（开仓/持仓，非空仓观望非平仓）。
//! 2. **方向一致** `σ_v=ε_b`：声部操作极性 = 笔的几何方向（向上笔 → 多头声部吃，
//!    向下笔 → 空头声部吃）。σ_v 是**整笔不变量**（声部方向在一笔内不翻转）。
//! 3. **单位为正** `q_{v,t}>0`：声部在 t 持有正手数（与 a_{v,t}=1 蕴含一致——激活
//!    且非零仓位）。
//!
//! 三条件须在**整个**区间 `[λ_b,ρ_b)` 内**逐时**成立（全称量词 ∀t），任一时刻破裂
//! ⟹ `¬Eat(v,b)`。
//!
//! ## 对接现有结构
//!
//! - 笔 `b` = [`Stroke`](super::super::types::Stroke)：方向 `ε_b`=`direction`
//!   （`Direction::Up`/`Down`），区间 `I_b=[start_index, end_index)`（整数 K 序号，
//!   左闭右开，bit-exact 对齐 spec C01 `I_i=[λ_i,ρ_i)`）。
//! - 声部方向 `σ_v` = [`VoiceSide`](super::super::strategy::voice::VoiceSide)
//!   （`Long`/`Short`/`Flat`）。
//! - 几何方向 → 声部极性桥接 `dir_to_side`：`Up→Long`、`Down→Short`——与现有
//!   `strategy::mod` 的 `Long→Buy`/`Short→Sell`（strategy/mod.rs:159-160）+ `exec` 持仓
//!   方向语义（向上笔做多吃、向下笔做空吃）一致。
//! - 逐时活动 `(a_{v,t}, q_{v,t})` = [`VoiceActivitySample`]：现有
//!   [`VoiceState`](super::super::strategy::voice::VoiceState) 是**单时刻 snapshot**
//!   不带时间轴；C06 的全称量词 `∀t∈I_b` 需要**区间内逐时序列**，故本模块新建轻量
//!   样本类型承载 `(t, a_t, q_t)`，不改 `VoiceState`（C06 是组装层语义，逐时序列由
//!   运行时活动流提供）。
//!
//! ## 认识论等级（231号 formalization-validity-domain）
//!
//! **L0（定义内蕴）**：`Eat(v,b)` 的真假是「区间逐时三条件全称合取」的逻辑必然，
//! 不依赖经验数据——给定笔 b 与声部活动序列，`Eat` 是纯布尔判定。Rust 实装对齐
//! C06 的 L0 结构定义 = L1 一致性（判定逻辑 bit-exact），**不冒充 L2**（不声明
//! 「声部吃到 ⟹ 实盘盈利」——那是 C16/C21 毛收益 + L2/L3 验证范畴，本模块不涉及）。

use super::super::strategy::voice::VoiceSide;
use super::super::types::{Direction, Stroke};
use super::recursive_tower::LeveledMove;

/// ∀e∈E 的 WF-Contain 违反计数（gap-B 数据层验证结果）。
///
/// `uncovered_elements` = 塔中 par(e) 区间**不包含** e 区间的元素数（违反闭端点包含
/// λ_a≤λ_e ∧ ρ_e≤ρ_a）。`total_elements` = 检查的 (parent,sub) 对总数。
///
/// 认识论 **L2**（formalization-validity-domain 231号）：这是在**真实数据解析出的塔**上
/// 验证 WF-Contain 不变量，可否证——`uncovered>0` 是否定性结果（暴露 par 实为 host^op
/// 依附 = gap-B 反例真实存在），`uncovered=0` 是确认性结果（情况 A：par=host^struct 构成）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ContainmentReport {
    /// 检查的 (parent, sub) 对总数（= 全塔所有上级走势的 sub_moves 之和）。
    pub total_elements: usize,
    /// WF-Contain 违反数（par 区间不包含 sub 区间）。情况 A 下应 = 0。
    pub uncovered_elements: usize,
}

/// **∀e∈E WF-Contain 验证器**（gap-B 数据层确证，spec MR2/§D + on2.pdf 情况 A 判定）。
///
/// 遍历递归走势塔 `tower`（每级 = `Vec<LeveledMove>`，元素集 E = 全级别全走势），对每个
/// 上级走势 `parent` 的每个构成子 `sub ∈ parent.sub_moves`，检查 WF-Contain：
///
/// ```text
/// λ_a ≤ λ_e ∧ ρ_e ≤ ρ_a   即   parent.start_index ≤ sub.start_index ∧ sub.end_index ≤ parent.end_index
/// ```
///
/// ★为何 `parent.sub_moves` 是 **host^struct 构成父**而非 **host^op 依附**（gap-B 成立关键，
/// #24 + codex 异质审 2026-06-30）：`LeveledMove::compose`（recursive_tower.rs:132）令
/// `parent.start = subs.first().start`、`parent.end = subs.last().end`——父区间 = 子元素并集
/// `[λ_{e1},ρ_{em})`（WF-Par 构成关系），且 `descend(parent)==subs` 不变量保证 `sub_moves`
/// 是真正构成 parent 的次级别走势，**不是**最近容器/右端点命中容器。故 codex 反例
/// `a=[0,5),e=[2,8)`（依附 host）在此 carrier 下不可达。
///
/// ★L2 否定性：若真实数据跑出 `uncovered>0`，说明某处 par 实为依附（gap-B 反例真实存在），
/// 须 escalate（定义冲突，no-workaround——不伪造覆盖）。`uncovered=0` 确认情况 A。
///
/// 递归性：`sub.sub_moves` 由本函数对 `tower` 各级遍历自然覆盖（塔已展开每级，无需在此再下钻）。
pub fn verify_containment(tower: &[&[LeveledMove]]) -> ContainmentReport {
    let mut report = ContainmentReport::default();
    for level in tower {
        for parent in *level {
            for sub in &parent.sub_moves {
                report.total_elements += 1;
                // WF-Contain：闭端点包含。任一端点越界 ⟹ par 非构成父（依附 host）⟹ 违反。
                if parent.start_index > sub.start_index || sub.end_index > parent.end_index {
                    report.uncovered_elements += 1;
                }
            }
        }
    }
    report
}

/// 几何方向 `ε_b` → 声部操作极性 `σ` 桥接（Up→Long，Down→Short）。
///
/// C06 的方向一致条件 `σ_v=ε_b` 比较的是声部极性与笔方向。笔方向是几何的
/// （[`Direction`]，向上/向下），声部方向是操作的（[`VoiceSide`]，多/空）；二者
/// 通过「向上笔由多头声部捕获、向下笔由空头声部捕获」对齐——与现有
/// `strategy::mod` `side_to_fill`（`Long→Buy`/`Short→Sell`，strategy/mod.rs:159-160）
/// 同一极性律：做多吃涨、做空吃跌。
///
/// 边界条件：[`Direction`] 只有 Up/Down 两态（无 Flat），故本映射只返回 Long/Short，
/// 永不返回 `VoiceSide::Flat`——笔恒有非零几何方向（spec C01 `ε_i(P_ρ-P_λ)>0` 即
/// 笔方向严格非零）。若 Θ 引入中性笔（ε=0）则须 change request（不在 Θ v0 内）。
pub fn dir_to_side(dir: Direction) -> VoiceSide {
    match dir {
        Direction::Up => VoiceSide::Long,
        Direction::Down => VoiceSide::Short,
    }
}

/// 声部在某时刻 `t` 的活动样本 `(t, a_{v,t}, q_{v,t})`（C06 逐时判定的单点）。
///
/// 字段语义（C06 §3）：
/// - `index`：时刻 t 的 K 序号（与 [`Stroke`] 的 `start_index`/`end_index` 同坐标系，
///   整数 K 序号）。
/// - `active`：`a_{v,t}∈{0,1}` 的 bool 镜像——`true`=激活（a=1），`false`=非激活（a=0）。
/// - `units`：`q_{v,t}≥0` 持仓手数。
///
/// 一笔区间 `I_b=[λ_b,ρ_b)` 内声部的活动 = 一组 `VoiceActivitySample`（每个 t∈I_b
/// 一个样本）。本类型不携带方向 σ_v——σ_v 是**整笔不变量**（声部方向一笔内不翻转），
/// 在 [`eat`] 中作为单独参数传入，避免逐样本冗余方向字段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoiceActivitySample {
    /// 时刻 t 的 K 序号（与 [`Stroke`] 区间端点同坐标系）。
    pub index: usize,
    /// `a_{v,t}`：声部在 t 是否激活（true=a=1 激活，false=a=0 非激活）。
    pub active: bool,
    /// `q_{v,t}`：声部在 t 的持仓手数（≥0）。
    pub units: u32,
}

/// **C06 `Eat(v,b)`：声部 v 是否吃到笔 b**（spec §3, 页3，bit-exact）。
///
/// 判定 `∀t∈I_b, a_{v,t}=1 ∧ σ_v=ε_b ∧ q_{v,t}>0`：
///
/// 1. **方向一致**（整笔不变量，先判）：`voice_side == dir_to_side(stroke.direction)`。
///    σ_v 是整笔常量，方向不一致 ⟹ 立即 `false`（无需逐时遍历）。
/// 2. **区间覆盖完整**：`samples` 必须**精确覆盖** `I_b=[start_index, end_index)` 的
///    每个 K 序号 t——缺任一 t 的样本 ⟹ 无法确证该 t 的 `a/q`，按全称量词保守判
///    `false`（C06 要求 ∀t 成立，缺样本 = 该 t 条件未证立 = 不满足 ∀）。
/// 3. **逐时三合取**：对每个 t∈I_b，须 `active=true`（a=1）∧ `units>0`（q>0）。任一
///    时刻破裂 ⟹ `false`。
///
/// 实现以 `samples` 提供的逐时活动序列为 `(a_{v,t}, q_{v,t})` 的真相源；σ_v 由
/// `voice_side` 参数给定（整笔不变量）。`samples` 顺序无关——按 `index` 集合是否
/// 等于 `I_b` 的全部整点判断覆盖完整性。
///
/// ★边界条件（结论翻转点）：
/// - `voice_side = Flat` ⟹ 永远 `false`（空仓声部无方向，不可能 σ_v=ε_b；空仓
///   亦无激活持仓）。
/// - 空笔区间 `start_index == end_index`（`I_b=∅`）⟹ ∀t∈∅ 真空真，但缠论笔恒非空
///   （spec C01 笔间隔 ≥config 根数 > 0），故此分支不应在合法笔上触发；为严格对齐
///   全称量词的真空真语义，空区间返回 `true`（∀t∈∅ 恒真），由上游保证不喂空笔。
/// - `start_index > end_index`（区间非法，倒序）⟹ `false`（非法区间不构成可吃笔）。
/// - 任一 t 缺样本 / `active=false` / `units=0` ⟹ `false`。
///
/// ★认识论 L0：返回值是上述布尔条件的逻辑必然，不依赖经验数据。
pub fn eat(voice_side: VoiceSide, stroke: &Stroke, samples: &[VoiceActivitySample]) -> bool {
    // 条件 1：方向一致（σ_v=ε_b），整笔不变量，先判可短路。
    // Flat 经 dir_to_side 永不返回（Direction 无 Flat），故 Flat 声部必不一致 ⟹ false。
    if voice_side != dir_to_side(stroke.direction) {
        return false;
    }

    let start = stroke.start_index;
    let end = stroke.end_index;

    // 非法区间（倒序）：不构成可吃笔。
    if start > end {
        return false;
    }

    // 空区间 [s,s)：∀t∈∅ 真空真（合法笔恒非空，由上游保证）。
    if start == end {
        return true;
    }

    // 条件 2+3：对 I_b=[start,end) 的每个整点 t，须存在样本且 active ∧ units>0。
    // 逐 t 查找对应样本（全称量词：缺任一 t 或任一 t 破裂 ⟹ false）。
    for t in start..end {
        match samples.iter().find(|s| s.index == t) {
            // 缺该 t 的样本：条件未证立，按 ∀ 保守判 false。
            None => return false,
            // a_{v,t}=1（active）∧ q_{v,t}>0（units>0）须同时成立。
            Some(sample) => {
                if !sample.active || sample.units == 0 {
                    return false;
                }
            }
        }
    }

    // 整笔区间逐时三条件全称合取成立 ⟹ Eat(v,b) 为真。
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 几何方向 → 声部极性桥接：Up→Long，Down→Short（与 strategy::mod side_to_fill 同极性律）。
    #[test]
    fn dir_to_side_maps_geometric_to_operational() {
        assert_eq!(dir_to_side(Direction::Up), VoiceSide::Long);
        assert_eq!(dir_to_side(Direction::Down), VoiceSide::Short);
    }

    /// 构造覆盖区间 [start,end) 全整点、全激活正仓的样本序列（Eat 真例的活动流）。
    fn full_active_samples(start: usize, end: usize, units: u32) -> Vec<VoiceActivitySample> {
        (start..end)
            .map(|t| VoiceActivitySample { index: t, active: true, units })
            .collect()
    }

    fn up_stroke(start: usize, end: usize) -> Stroke {
        Stroke { direction: Direction::Up, start_index: start, end_index: end, start_price: 100, end_price: 200 }
    }

    fn down_stroke(start: usize, end: usize) -> Stroke {
        Stroke { direction: Direction::Down, start_index: start, end_index: end, start_price: 200, end_price: 100 }
    }

    /// ★Eat 真例：向上笔 [3,6) + 多头声部 + 整区间逐时激活正仓 ⟹ Eat=true。
    #[test]
    fn eat_true_when_long_voice_covers_up_stroke_fully() {
        let stroke = up_stroke(3, 6);
        let samples = full_active_samples(3, 6, 5); // t=3,4,5 全 active+units=5
        assert!(eat(VoiceSide::Long, &stroke, &samples));
    }

    /// ★Eat 真例（空头侧）：向下笔 + 空头声部 + 整区间覆盖 ⟹ Eat=true（镜像对称）。
    #[test]
    fn eat_true_when_short_voice_covers_down_stroke_fully() {
        let stroke = down_stroke(0, 3);
        let samples = full_active_samples(0, 3, 2);
        assert!(eat(VoiceSide::Short, &stroke, &samples));
    }

    /// ★Eat 假例（方向不一致）：向上笔 + 空头声部 ⟹ σ_v≠ε_b ⟹ Eat=false。
    #[test]
    fn eat_false_when_direction_mismatch() {
        let stroke = up_stroke(3, 6);
        let samples = full_active_samples(3, 6, 5); // 活动流完整，但方向不一致
        assert!(!eat(VoiceSide::Short, &stroke, &samples));
    }

    /// Eat 假例（中途非激活）：某 t 处 a_{v,t}=0 ⟹ ∀t 破裂 ⟹ Eat=false。
    #[test]
    fn eat_false_when_inactive_at_some_t() {
        let stroke = up_stroke(3, 6);
        let mut samples = full_active_samples(3, 6, 5);
        samples[1].active = false; // t=4 非激活
        assert!(!eat(VoiceSide::Long, &stroke, &samples));
    }

    /// Eat 假例（中途零仓）：某 t 处 q_{v,t}=0 ⟹ ∀t 破裂 ⟹ Eat=false。
    #[test]
    fn eat_false_when_zero_units_at_some_t() {
        let stroke = up_stroke(3, 6);
        let mut samples = full_active_samples(3, 6, 5);
        samples[2].units = 0; // t=5 零仓
        assert!(!eat(VoiceSide::Long, &stroke, &samples));
    }

    /// Eat 假例（区间未完整覆盖）：缺 t=5 的样本 ⟹ ∀t 不可证 ⟹ Eat=false。
    #[test]
    fn eat_false_when_interval_not_fully_covered() {
        let stroke = up_stroke(3, 6);
        let samples = full_active_samples(3, 5, 5); // 只覆盖 t=3,4，缺 t=5
        assert!(!eat(VoiceSide::Long, &stroke, &samples));
    }

    /// Eat 假例（Flat 声部）：空仓声部无方向 ⟹ 永 false（边界条件）。
    #[test]
    fn eat_false_for_flat_voice() {
        let stroke = up_stroke(3, 6);
        let samples = full_active_samples(3, 6, 5);
        assert!(!eat(VoiceSide::Flat, &stroke, &samples));
    }

    /// Eat 边界（非法倒序区间）：start>end ⟹ false。
    #[test]
    fn eat_false_for_reversed_interval() {
        let stroke = up_stroke(6, 3); // start=6 > end=3
        let samples: Vec<VoiceActivitySample> = Vec::new();
        assert!(!eat(VoiceSide::Long, &stroke, &samples));
    }

    /// Eat 边界（空区间 [s,s)）：∀t∈∅ 真空真 ⟹ true（合法笔恒非空，由上游保证）。
    #[test]
    fn eat_true_for_empty_interval_vacuously() {
        let stroke = up_stroke(3, 3); // I_b=∅
        let samples: Vec<VoiceActivitySample> = Vec::new();
        assert!(eat(VoiceSide::Long, &stroke, &samples));
    }

    /// 样本顺序无关：打乱 samples 顺序不改判定（按 index 集合判覆盖）。
    #[test]
    fn eat_independent_of_sample_order() {
        let stroke = up_stroke(0, 3);
        let mut samples = full_active_samples(0, 3, 1);
        samples.reverse(); // 顺序打乱
        assert!(eat(VoiceSide::Long, &stroke, &samples));
    }

    // ── ∀e∈E WF-Contain 验证器（gap-B 数据层确证，MR2/§D）──────────────────

    use super::super::recursive_tower::{compose_level, ElementId};
    use super::super::center::UnitRange;
    use super::super::descend::RMove;

    fn unit(start: usize, end: usize, dir: Direction, lo: i64, hi: i64) -> UnitRange {
        UnitRange { start_index: start, end_index: end, direction: dir, lo, hi }
    }

    /// 构造真实多级塔（compose_level 滑窗，生产路径 `n`）：≥3 段 L0 线段 → 上级走势塔。
    /// 返回每级 LeveledMove（含 sub_moves 携坐标）。
    fn build_real_tower() -> Vec<Vec<LeveledMove>> {
        // 9 段 L0 线段，区间连续递增（真实塔的 K 序单调），价格上下交替成中枢。
        let units: Vec<UnitRange> = (0..9)
            .map(|i| {
                let s = i * 4;
                let (dir, lo, hi) = if i % 2 == 0 {
                    (Direction::Up, 100 + i as i64 * 2, 120 + i as i64 * 2)
                } else {
                    (Direction::Down, 90 + i as i64 * 2, 110 + i as i64 * 2)
                };
                unit(s, s + 4, dir, lo, hi)
            })
            .collect();
        let l0: Vec<LeveledMove> = units
            .iter()
            .enumerate()
            .map(|(i, u)| LeveledMove::from_unit(u, ElementId { level: 0, ordinal: i as u64 }))
            .collect();
        let (_c, l1) = compose_level(&units, &l0, true, 1);
        vec![l0, l1]
    }

    /// ★L2 确认（情况 A）：真实数据塔上 `verify_containment` 的 uncovered_elements = 0。
    /// par=host^struct 构成父（compose 令父区间=子并集），故 WF-Contain 恒成立 ⟹ gap-B 成立。
    #[test]
    fn verify_containment_zero_uncovered_on_real_tower() {
        let tower = build_real_tower();
        let refs: Vec<&[LeveledMove]> = tower.iter().map(|v| v.as_slice()).collect();
        let report = verify_containment(&refs);
        assert!(report.total_elements > 0, "塔须含上级走势（sub_moves 非空）");
        assert_eq!(
            report.uncovered_elements, 0,
            "情况 A：par=host^struct 构成父 ⟹ WF-Contain 恒成立 ⟹ ∀e Eat(e) 数据层必要条件满足"
        );
    }

    /// ★验证器非死代码（抓 host^op 依附反例）：手造 codex 反例 a=[0,5),e=[2,8)——
    /// par 区间不包含 e 区间（依附 host）⟹ uncovered_elements=1。证明验证器能否证 gap-B。
    #[test]
    fn verify_containment_catches_dependency_host_counterexample() {
        // 子 e=[2,8)，父 a=[0,5)：ρ_e=8 > ρ_a=5 违反 WF-Contain（依附 host，非构成父）。
        let child = LeveledMove {
            rmove: RMove::Segment { direction: Direction::Up, lo: 0, hi: 1 },
            start_index: 2,
            end_index: 8,
            sub_moves: Vec::new(),
            id: ElementId { level: 0, ordinal: 0 },
        };
        let parent = LeveledMove {
            rmove: RMove::Segment { direction: Direction::Up, lo: 0, hi: 1 },
            start_index: 0,
            end_index: 5,
            sub_moves: vec![child],
            id: ElementId { level: 1, ordinal: 0 },
        };
        let level = vec![parent];
        let refs: Vec<&[LeveledMove]> = vec![level.as_slice()];
        let report = verify_containment(&refs);
        assert_eq!(report.uncovered_elements, 1, "依附 host 反例须被验证器抓到（否则 verifier 是死代码）");
    }
}
