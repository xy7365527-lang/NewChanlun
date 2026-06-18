//! T 步骤 c)：走势完美判定（纯结构性背驰，选项C，不用 MACD）。
//!
//! 设计文档 §6：背驰是 T 中唯一的度量剩余。本模块只实装**结构性**部分
//! （第37课趋势背驰5条件的结构判据 + 嵌套深度力度比较），不调用 MACD。
//!
//! 第24课：「用均线或MACD看背驰都是辅助性的……配合上中枢，那是 100% 绝对的，
//! 因为这可以用纯数学的推理逻辑地证明。」
//!
//! ## 第37课趋势背驰5条件（`judge_trend_divergence` 全实装）
//!
//! 趋势 a+A+b+B+c 形式，A、B 锚定**最后两个同级别中枢**（A=倒数第二中枢=背驰背景，
//! B=末中枢；第24课「A 之前已有一个中枢，B 是另一个中枢」，≥3 中枢趋势同此）：
//! 1. A、B 是同级别中枢 —— `zhongshus.len() ≥ 2`；
//! 2. c 含对 B 的第三类买卖点 —— c 段回抽不破 B 的 ZG(上)/ZD(下)（守沿分量；「先离开」
//!    由 4+5 回补，「第一次回抽」序数未单独编码——有效域边界）；
//! 3. b 在级别上不大于 c —— b 段（A↔B 连接段）嵌套深度 ≤ c 段；
//! 4. 上涨 c 创新高 / 下跌 c 创新低 —— c 段极值超越此前全部极值；
//! 5. c 段含 ≥2 个次级别中枢 —— c 段嵌套深度和 ≥2（只验中枢计数，未验依次同向构成趋势）。
//! + 第24课力度：c 段力度 < a 段（嵌套深度 / 类背驰振幅）。
//!
//! 条件 2/3/5 是 79% 伪背驰（buysellpoint.rs:460-465 v3 raw type1「下跌途中连续刷新低
//! 的伪底背驰」）的结构滤网。基底层（a₀，无中枢嵌套）按第64课退化为类背驰（仅 1+4+力度）。
//! 中枢锚定/守沿/中枢计数的有效域边界由对抗核验（6 验证者）确立，见 §6.7 决策记录。
//!
//! ## 力度的结构化度量（设计文档 §6.4 候选「中枢嵌套深度比较」）
//!
//! 背驰 = 走势末段相对前一同向段「力度衰减」。本模块的力度度量分两档：
//! - **嵌套深度档**（level≥1，真背驰）：一段的力度 = 该段单元的 `inner_zhongshu_count`
//!   之和（下级走势的结构复杂度）。第37课「c 内部套用 a+A+b+B+c」= 区间套力度比较。
//! - **类背驰档**（level=0，a₀ 基底，第64课「线段以下是类中枢」）：单元无内部中枢
//!   （和为 0），退化为几何振幅 `max(high) - min(low)`。第65课「区间套用的就是类背
//!   驰的力度比较」。
//!
//! 这条「level=0 用振幅、level≥1 用嵌套深度」的划分不是补丁，是第64/65课原文对
//! 基底层与递归层的区分（设计文档 §2.5 + §6.5）。

use super::types::{BSPKind, Direction, TrendKind, TrendType, Unit, BSP};

/// 一段单元的结构化力度。
///
/// 优先用嵌套深度（下级中枢数之和）；若全段无内部中枢（a₀ 基底层）退化为几何振幅。
fn leg_strength(units: &[Unit]) -> f64 {
    let nest: usize = units.iter().map(|u| u.inner_zhongshu_count).sum();
    if nest > 0 {
        nest as f64
    } else {
        let hi = units.iter().map(|u| u.high).fold(f64::MIN, f64::max);
        let lo = units.iter().map(|u| u.low).fold(f64::MAX, f64::min);
        hi - lo
    }
}

/// 一段单元的极值（最高 high / 最低 low）。
fn leg_extreme(units: &[Unit], dir: Direction) -> (f64, i64) {
    match dir {
        Direction::Up => {
            // 取最高 high；该极值 bar：上行单元高点在 end_bar，下行单元高点在 start_bar。
            let mut best = f64::MIN;
            let mut bar = 0i64;
            for u in units {
                if u.high > best {
                    best = u.high;
                    bar = if u.direction == Direction::Up {
                        u.end_bar
                    } else {
                        u.start_bar
                    };
                }
            }
            (best, bar)
        }
        Direction::Down => {
            let mut best = f64::MAX;
            let mut bar = 0i64;
            for u in units {
                if u.low < best {
                    best = u.low;
                    bar = if u.direction == Direction::Down {
                        u.end_bar
                    } else {
                        u.start_bar
                    };
                }
            }
            (best, bar)
        }
    }
}

/// 步骤 c)：判定走势是否终完美（背驰），若是则产生 type1 买卖点。
///
/// 趋势背驰（第37课，需 ≥2 同级别中枢 A、B）：
/// 1. `a` 段 = 进入段（首中枢之前 + 首中枢，作为力度基准）；
/// 2. `c` 段 = 末中枢之后的单元（离开段）；
/// 3. 条件4：`c` 创新高（上涨）/ 新低（下跌）——超越此前全部极值；
/// 4. 力度衰减：`strength(c) < strength(a)`；
/// 5. 同时满足 → 背驰：上涨产生 Type1Sell，下跌产生 Type1Buy。
///
/// 盘整背驰（单中枢）：进入段 vs 离开段同样比较，离开创新极值且力度衰减 → 盘背 type1。
pub fn judge_divergence(t: &TrendType) -> Option<BSP> {
    match t.kind {
        TrendKind::UpTrend | TrendKind::DownTrend => judge_trend_divergence(t),
        TrendKind::Consolidation => judge_consolidation_divergence(t),
    }
}

fn judge_trend_divergence(t: &TrendType) -> Option<BSP> {
    let n_centers = t.zhongshus.len();
    if n_centers < 2 {
        return None;
    }
    // A = 倒数第二中枢（背驰背景，第24课「A 之前已有一个中枢，B 是这个大趋势的另一个
    // 中枢」），B = 末中枢。对 ≥3 中枢趋势，背驰段必须锚定**最后两个中枢**（与已验证
    // bit-exact 的 crate::divergence 的 indices[-2]/[-1] 一致）；旧首/末读法会把中间中枢
    // 折进 b 段使其膨胀、系统性过严错杀（对抗核验 major）。2 中枢时 n-2=0，与旧行为一致。
    let prev_center = &t.zhongshus[n_centers - 2];
    let last_center = &t.zhongshus[n_centers - 1];

    // pub fn 边界防御：中枢按第17课须 ≥3 单元（find_centers 保证），但 pub API 不能因
    // 外部构造空 units 的 Zhongshu 而 panic——返回 None 使「Option 永不 panic」契约诚实。
    if prev_center.units.is_empty() || last_center.units.is_empty() {
        return None;
    }

    // a 段：切片起点到倒数第二中枢末单元（进入段 + 累积至 A 的背驰背景，力度基准）。
    let a_end = *prev_center.units.last().unwrap();
    let a_leg = &t.units[0..=a_end];

    // c 段：末中枢末单元之后。
    let c_start = *last_center.units.last().unwrap() + 1;
    if c_start >= t.units.len() {
        return None; // 末中枢之后无离开段，走势尚未离开 → 未完成。
    }
    let c_leg = &t.units[c_start..];

    let (c_ext, c_bar) = leg_extreme(c_leg, t.direction);

    // 条件4：c 创新高/新低（超越 c 段之前的全部极值）。
    let prior = &t.units[0..c_start];
    let (prior_ext, _) = leg_extreme(prior, t.direction);
    let new_extreme = match t.direction {
        Direction::Up => c_ext > prior_ext,
        Direction::Down => c_ext < prior_ext,
    };
    if !new_extreme {
        return None;
    }

    // ── 第64课：有中枢嵌套信息 → 真背驰（第37课5条件全检）；无 → 类背驰 ──
    // 「线段以下是没有中枢的……是类中枢/类背驰」（第64课）。a₀ 基底层单元
    // `inner_zhongshu_count=0`（无真次级别中枢），条件 2/3/5 不适用，退化为条件 1+4+力度
    // （类背驰）。level≥1 封装单元携带真中枢数 → 条件 2/3/5 激活（真背驰）。判据与
    // `leg_strength` 的「nest>0 用嵌套深度、否则用振幅」同源（设计文档 §2.5/§6.4）。
    //
    // 真背驰是 79% 伪背驰（buysellpoint.rs:460-465「下跌途中连续刷新低的伪底背驰」）的
    // 结构滤网：单凭力度衰减（条件4+力度）= 背驰候选；加 2/3/5 = 走势完美。
    let has_nest = t.units.iter().any(|u| u.inner_zhongshu_count > 0);
    if has_nest {
        // 条件2：c 含对 B（末中枢）的第三类买卖点。完整 type3 = 先离开中枢、再回抽不破
        // ZG/ZD。本条只校验「回抽守住中枢边沿」这一**守沿分量**（∃ 单元不破 ZG/ZD）；
        // 「先离开」由条件4（c 创新高/新低）+ 条件5（c 含≥2 次级别中枢的离开走势）结构性
        // 回补，「第一次回抽」严格序数（第38课）未单独编码——对抗核验 minor，记为有效域边界。
        let c_holds_center_edge = c_leg.iter().any(|u| match t.direction {
            Direction::Up => u.low > last_center.high, // 回抽不破中枢上沿 ZG
            Direction::Down => u.high < last_center.low, // 回抽不破中枢下沿 ZD
        });
        if !c_holds_center_edge {
            return None;
        }
        // 条件5：c 段含 ≥2 个次级别(k-1)中枢（嵌套深度和 ≥2）。只校验中枢总数，不校验这
        // 2 个中枢「依次同向构成单一次级别趋势」（需对 c 段重跑 find_centers，当前未实装）
        // ——对抗核验 minor，避免声明膨胀（不声称「c 是次级别趋势」）。
        let c_nest: usize = c_leg.iter().map(|u| u.inner_zhongshu_count).sum();
        if c_nest < 2 {
            return None;
        }
        // 条件3：b 在级别上不大于 c——b 段（倒数第二中枢 A ↔ 末中枢 B 之间的连接段）嵌套
        // 深度 ≤ c 段。a_end 已锚定倒数第二中枢，故 ≥3 中枢时 b 段不再吞中间中枢。
        let b_start = a_end + 1;
        let b_endx = *last_center.units.first().unwrap(); // 末中枢首单元（不含）
        let b_nest: usize = if b_start < b_endx {
            t.units[b_start..b_endx]
                .iter()
                .map(|u| u.inner_zhongshu_count)
                .sum()
        } else {
            0 // A、B 紧邻，b 段为空（级别 0 ≤ c，自然满足）
        };
        if b_nest > c_nest {
            return None;
        }
    }

    // 第24课力度衰减：c 段结构力度 < a 段（嵌套深度 / 类背驰振幅，同 leg_strength）。
    if leg_strength(c_leg) >= leg_strength(a_leg) {
        return None;
    }

    let kind = match t.direction {
        Direction::Up => BSPKind::Type1Sell,
        Direction::Down => BSPKind::Type1Buy,
    };
    Some(BSP {
        kind,
        bar: c_bar,
        price: c_ext,
        level: t.level,
    })
}

fn judge_consolidation_divergence(t: &TrendType) -> Option<BSP> {
    if t.zhongshus.len() != 1 {
        return None;
    }
    let center = &t.zhongshus[0];
    if center.units.is_empty() {
        return None; // pub fn 边界防御（同 judge_trend_divergence）。
    }
    let enter_end = *center.units.last().unwrap();
    // 进入段 = 切片起点到中枢末单元；离开段 = 其后。
    if enter_end + 1 >= t.units.len() {
        return None;
    }
    let enter_leg = &t.units[0..=enter_end];
    let leave_leg = &t.units[enter_end + 1..];

    let (leave_ext, leave_bar) = leg_extreme(leave_leg, t.direction);
    let (enter_ext, _) = leg_extreme(enter_leg, t.direction);
    let new_extreme = match t.direction {
        Direction::Up => leave_ext > enter_ext,
        Direction::Down => leave_ext < enter_ext,
    };
    if !new_extreme {
        return None;
    }
    if leg_strength(leave_leg) >= leg_strength(enter_leg) {
        return None;
    }

    let kind = match t.direction {
        Direction::Up => BSPKind::Type1Sell,
        Direction::Down => BSPKind::Type1Buy,
    };
    Some(BSP {
        kind,
        bar: leave_bar,
        price: leave_ext,
        level: t.level,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recursive_t::types::Zhongshu;

    fn bi(low: f64, high: f64, s: i64, e: i64, d: Direction) -> Unit {
        Unit::stroke(low, high, s, e, d)
    }

    /// 构造一个上涨趋势（2 中枢），末段创新高但力度（振幅）衰减 → 应背驰。
    fn 背驰上涨趋势() -> TrendType {
        // 中枢1 [12,20]（段0-2），强进入段振幅大；离开段创新高但振幅小。
        let units = vec![
            bi(10.0, 30.0, 0, 1, Direction::Up),  // a 段进入：振幅 20（强）
            bi(12.0, 22.0, 1, 2, Direction::Down),
            bi(11.0, 21.0, 2, 3, Direction::Up),
            bi(21.0, 41.0, 3, 4, Direction::Up),  // 离开向上到中枢2
            bi(38.0, 48.0, 4, 5, Direction::Down),
            bi(40.0, 50.0, 5, 6, Direction::Up),
            bi(39.0, 49.0, 6, 7, Direction::Down),
            bi(49.0, 52.0, 7, 8, Direction::Up),  // c 段创新高 52 但振幅仅 3（弱）
        ];
        let z1 = Zhongshu { high: 20.0, low: 12.0, gg: 30.0, dd: 10.0, units: vec![0, 1, 2], level: 1 };
        let z2 = Zhongshu { high: 48.0, low: 40.0, gg: 50.0, dd: 38.0, units: vec![4, 5, 6], level: 1 };
        TrendType {
            kind: TrendKind::UpTrend,
            zhongshus: vec![z1, z2],
            units,
            level: 1,
            direction: Direction::Up,
            completed: false,
            bsp: None,
        }
    }

    #[test]
    fn 上涨趋势末段力度衰减创新高_产生一卖() {
        let t = 背驰上涨趋势();
        let bsp = judge_divergence(&t).expect("应产生背驰买卖点");
        assert_eq!(bsp.kind, BSPKind::Type1Sell);
        assert_eq!(bsp.price, 52.0); // c 段新高
        assert_eq!(bsp.level, 1);
    }

    #[test]
    fn 末段力度不衰减则无背驰() {
        let mut t = 背驰上涨趋势();
        // 把 c 段换成强力度（大振幅）：不应背驰。
        let last = t.units.len() - 1;
        t.units[last] = bi(49.0, 90.0, 7, 8, Direction::Up); // 振幅 41 > a 段 20
        assert!(judge_divergence(&t).is_none());
    }

    #[test]
    fn 末段不创新高则无背驰() {
        let mut t = 背驰上涨趋势();
        let last = t.units.len() - 1;
        t.units[last] = bi(45.0, 47.0, 7, 8, Direction::Up); // 高点 47 < 此前 50
        assert!(judge_divergence(&t).is_none());
    }

    #[test]
    fn 嵌套深度档_高级别背驰() {
        // level≥1：用 inner_zhongshu_count 而非振幅。a 段嵌套深，c 段嵌套浅。
        let mk = |low, high, s, e, d, nest| Unit {
            high,
            low,
            start_bar: s,
            end_bar: e,
            direction: d,
            level: 1,
            inner_zhongshu_count: nest,
        };
        let units = vec![
            mk(10.0, 30.0, 0, 1, Direction::Up, 3), // a 进入：嵌套深 3
            mk(12.0, 22.0, 1, 2, Direction::Down, 2),
            mk(11.0, 21.0, 2, 3, Direction::Up, 2),
            mk(21.0, 41.0, 3, 4, Direction::Up, 2),
            mk(38.0, 48.0, 4, 5, Direction::Down, 2),
            mk(40.0, 50.0, 5, 6, Direction::Up, 2),
            mk(39.0, 49.0, 6, 7, Direction::Down, 2),
            mk(49.0, 60.0, 7, 8, Direction::Up, 2), // c 创新高 60，含 2 次级别中枢(nest=2)，弱于 a
        ];
        let z1 = Zhongshu { high: 20.0, low: 12.0, gg: 30.0, dd: 10.0, units: vec![0, 1, 2], level: 2 };
        let z2 = Zhongshu { high: 48.0, low: 40.0, gg: 50.0, dd: 38.0, units: vec![4, 5, 6], level: 2 };
        let t = TrendType {
            kind: TrendKind::UpTrend,
            zhongshus: vec![z1, z2],
            units,
            level: 2,
            direction: Direction::Up,
            completed: false,
            bsp: None,
        };
        // a 段嵌套 = 3+2+2 = 7（进入段+首中枢），c 段嵌套 = 2（含2次级别中枢）< 7 → 背驰。
        let bsp = judge_divergence(&t).expect("高级别嵌套深度背驰");
        assert_eq!(bsp.kind, BSPKind::Type1Sell);
        assert_eq!(bsp.price, 60.0);
    }

    /// 真背驰条件5不足：c 段次级别中枢数 <2（c 非次级别趋势）→ 拒绝（滤掉 79% 伪背驰）。
    #[test]
    fn 真背驰_条件5_c段中枢不足2_拒绝() {
        let mk = |low, high, s, e, d, nest| Unit {
            high, low, start_bar: s, end_bar: e, direction: d, level: 1, inner_zhongshu_count: nest,
        };
        let units = vec![
            mk(10.0, 30.0, 0, 1, Direction::Up, 3),
            mk(12.0, 22.0, 1, 2, Direction::Down, 2),
            mk(11.0, 21.0, 2, 3, Direction::Up, 2),
            mk(21.0, 41.0, 3, 4, Direction::Up, 2),
            mk(38.0, 48.0, 4, 5, Direction::Down, 2),
            mk(40.0, 50.0, 5, 6, Direction::Up, 2),
            mk(39.0, 49.0, 6, 7, Direction::Down, 2),
            mk(49.0, 60.0, 7, 8, Direction::Up, 1), // c 段 nest=1 <2 → 条件5 不满足
        ];
        let z1 = Zhongshu { high: 20.0, low: 12.0, gg: 30.0, dd: 10.0, units: vec![0, 1, 2], level: 2 };
        let z2 = Zhongshu { high: 48.0, low: 40.0, gg: 50.0, dd: 38.0, units: vec![4, 5, 6], level: 2 };
        let t = TrendType {
            kind: TrendKind::UpTrend, zhongshus: vec![z1, z2], units, level: 2,
            direction: Direction::Up, completed: false, bsp: None,
        };
        assert!(judge_divergence(&t).is_none(), "c 段非次级别趋势（中枢<2）应拒绝");
    }

    /// 真背驰条件2不满足：c 段回抽跌破 B 中枢上沿 ZG（无类三买结构）→ 拒绝。
    #[test]
    fn 真背驰_条件2_回抽破中枢上沿_拒绝() {
        let mk = |low, high, s, e, d, nest| Unit {
            high, low, start_bar: s, end_bar: e, direction: d, level: 1, inner_zhongshu_count: nest,
        };
        let units = vec![
            mk(10.0, 30.0, 0, 1, Direction::Up, 3),
            mk(12.0, 22.0, 1, 2, Direction::Down, 2),
            mk(11.0, 21.0, 2, 3, Direction::Up, 2),
            mk(21.0, 41.0, 3, 4, Direction::Up, 2),
            mk(38.0, 48.0, 4, 5, Direction::Down, 2),
            mk(40.0, 50.0, 5, 6, Direction::Up, 2),
            mk(39.0, 49.0, 6, 7, Direction::Down, 2),
            mk(47.0, 60.0, 7, 8, Direction::Up, 2), // c low=47 < B.ZG=48 → 回抽破中枢上沿
        ];
        let z1 = Zhongshu { high: 20.0, low: 12.0, gg: 30.0, dd: 10.0, units: vec![0, 1, 2], level: 2 };
        let z2 = Zhongshu { high: 48.0, low: 40.0, gg: 50.0, dd: 38.0, units: vec![4, 5, 6], level: 2 };
        let t = TrendType {
            kind: TrendKind::UpTrend, zhongshus: vec![z1, z2], units, level: 2,
            direction: Direction::Up, completed: false, bsp: None,
        };
        assert!(judge_divergence(&t).is_none(), "c 段回抽破 ZG（无类三买）应拒绝");
    }

    /// 类背驰（a₀ 基底，无嵌套）：第64课「线段以下用类背驰」——仅条件1+4+力度，
    /// 不要求条件2/3/5。bi() 的 inner_zhongshu_count=0 → has_nest=false → 类路径。
    #[test]
    fn 类背驰_基底层无嵌套_仅力度创新高产一卖() {
        let units = vec![
            bi(10.0, 30.0, 0, 1, Direction::Up), // a 进入振幅 20（强）
            bi(12.0, 22.0, 1, 2, Direction::Down),
            bi(11.0, 21.0, 2, 3, Direction::Up),
            bi(21.0, 41.0, 3, 4, Direction::Up),
            bi(38.0, 48.0, 4, 5, Direction::Down),
            bi(40.0, 50.0, 5, 6, Direction::Up),
            bi(39.0, 49.0, 6, 7, Direction::Down),
            bi(49.0, 52.0, 7, 8, Direction::Up), // c 创新高 52，振幅 3（弱）
        ];
        let z1 = Zhongshu { high: 20.0, low: 12.0, gg: 30.0, dd: 10.0, units: vec![0, 1, 2], level: 1 };
        let z2 = Zhongshu { high: 48.0, low: 40.0, gg: 50.0, dd: 38.0, units: vec![4, 5, 6], level: 1 };
        let t = TrendType {
            kind: TrendKind::UpTrend, zhongshus: vec![z1, z2], units, level: 0,
            direction: Direction::Up, completed: false, bsp: None,
        };
        let bsp = judge_divergence(&t).expect("类背驰应产一卖");
        assert_eq!(bsp.kind, BSPKind::Type1Sell);
        assert_eq!(bsp.price, 52.0);
    }

    /// ≥3 中枢趋势：A/B 与 b 段必须锚定**最后两个中枢**（A=倒数第二、B=末），不得吞中间
    /// 中枢（对抗核验 major 回归）。中间中枢 z2 嵌套深（3×3=9）：旧首/末读法 b 段=z2 →
    /// b_nest=9 > c_nest=4 → 系统性过严错杀；修复后 b 段=z2↔z3 连接段（空）→ b_nest=0 ≤ 4，
    /// 5 条件全过正确产一卖。该路径在所有 2 中枢测试中零覆盖，是此前未暴露的有效域边界。
    #[test]
    fn 三中枢趋势_b段锚最后两中枢_不吞中间中枢() {
        let mk = |low, high, s, e, d, nest| Unit {
            high, low, start_bar: s, end_bar: e, direction: d, level: 2, inner_zhongshu_count: nest,
        };
        let units = vec![
            mk(8.0, 22.0, 0, 1, Direction::Up, 2), // z1
            mk(12.0, 18.0, 1, 2, Direction::Down, 2),
            mk(10.0, 20.0, 2, 3, Direction::Up, 2),
            mk(28.0, 42.0, 3, 4, Direction::Up, 3), // z2（中间中枢，嵌套深 3×3=9）
            mk(32.0, 40.0, 4, 5, Direction::Down, 3),
            mk(30.0, 41.0, 5, 6, Direction::Up, 3),
            mk(48.0, 62.0, 6, 7, Direction::Up, 2), // z3（末中枢 B）
            mk(52.0, 60.0, 7, 8, Direction::Down, 2),
            mk(50.0, 61.0, 8, 9, Direction::Up, 2),
            mk(63.0, 75.0, 9, 10, Direction::Up, 2), // c 段：离开 z3 向上，low=63 > ZG=60（守沿）
            mk(70.0, 80.0, 10, 11, Direction::Up, 2), // c 创新高 80，c_nest=2+2=4
        ];
        let z1 = Zhongshu { high: 20.0, low: 10.0, gg: 22.0, dd: 8.0, units: vec![0, 1, 2], level: 2 };
        let z2 = Zhongshu { high: 40.0, low: 30.0, gg: 42.0, dd: 28.0, units: vec![3, 4, 5], level: 2 };
        let z3 = Zhongshu { high: 60.0, low: 50.0, gg: 62.0, dd: 48.0, units: vec![6, 7, 8], level: 2 };
        let t = TrendType {
            kind: TrendKind::UpTrend, zhongshus: vec![z1, z2, z3], units, level: 2,
            direction: Direction::Up, completed: false, bsp: None,
        };
        // a 段=units[0..=5](entry+z1+z2) nest=15；b 段=z2↔z3 间(空) b_nest=0≤c_nest=4；
        // c 创新高 80、守 ZG=60、力度 4<15 → 5 条件全过。
        let bsp = judge_divergence(&t).expect("3 中枢趋势 b 段锚最后两中枢应产一卖");
        assert_eq!(bsp.kind, BSPKind::Type1Sell);
        assert_eq!(bsp.price, 80.0);
    }
}
