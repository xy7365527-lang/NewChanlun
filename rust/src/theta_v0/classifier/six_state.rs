//! 走势六态 r + 信号位向量 b 的 canonical 对照层（L2-A；契约锚 `Origin.TrendSixState`）。
//!
//! ## 工位定位（L2-A 补全，FULL_USER_FORMULA_SOURCE.md §5 对照）
//!
//! FULL §5（line 277-308）的 canonical 表示有两个**不同构**的对象：
//!   (1) 走势六态 `r ∈ {⊥,I,U⁰,U¹,D⁰,D¹}`（位置三态 × 第三类标记位的真扩展为六态，partition）；
//!   (2) 信号位向量 `b = (B₁,B₂,B₃,S₁,S₂,S₃) ∈ {0,1}⁶`（六独立 bool，三类买卖点**不强制互斥**）。
//!
//! 六态 r 已在 [`super::level_state::RLevel`] 实装（Bot/Inside/AboveNo3B/AboveB3/BelowNo3S/BelowS3）；
//! 信号位 b 已在 [`super::super::types::BspBits`] 实装（buy1/buy2/buy3/sell1/sell2/sell3）。本模块
//! **不重复实装**——它建立 rust 实装 ↔ Lean `Origin.TrendSixState` 的 **canonical 命名对照 + parity
//! 断言**（六态构造子 bit-exact 对应 Lean `TrendSixState`；信号位 2B/3B 可重合 + 1B/2B 互斥与 Lean
//! `signalBits_2b3b_coexist`/`signalBits_1b2b_exclusive` 一致）。
//!
//! ## 认识论（formalization-validity-domain 231号）
//!
//! 全部 **L1**（bit-exact 一致性：rust 六态/信号位输出与 Lean `TrendSixState` 形式化对齐 = 验证管线
//! 正确，不验证 r/b 在市场上有效）。Lean 侧六态 partition ∃! + 信号位真 6 维非塌缩是 **L0**
//! （machine-checked）；本模块测试是 L1（管线正确性镜像，零信息增量）。L2/L3 不声称。
//!
//! GUARD-ROLE: lean-mirror-reference
//!
//! ## 名分（ADR-0004 C6 名分程序，#745）
//!
//! - **名分**：对照件——五态判据下属「现役」（`pub mod six_state;` 挂在 main 线，无
//!   deprecated 标记），角色是 rust 实装 ↔ Lean `Origin.TrendSixState` 的 canonical 命名
//!   对照 + parity 断言层，非主判据决策路径。
//! - **对照什么**：`Origin.TrendSixState`（走势六态 r 的构造子顺序 + 信号位 2B/3B 可重合、
//!   1B/2B 互斥性质）。
//! - **与现役差在哪**：本模块**不重复实装**判据——六态 r 的真实产出在
//!   [`super::level_state::RLevel`]，信号位 b 的真实置位在 [`super::bsp::endpoint_to_bsp`]；
//!   本文件只建立命名对照表 + parity 断言，全仓零生产调用者（仅自身 `#[cfg(test)]`）。
//! - **禁回灌**：不得反过来让 `level_state`/`bsp` 依赖本模块的命名表做判定——依赖方向单向
//!   （本模块读它们，不可逆），本文件也不得新增判据逻辑（那会制造第二份实现分叉）。

use super::super::types::{BspBits, Center, Tick};
use super::bsp::{endpoint_to_bsp, EndpointSituation};
use super::level_state::{rlevel_of, RContext, RLevel};

/// 走势六态 r 的 canonical 名称表（FULL §5 符号 ↔ Lean `TrendSixState` 构造子 ↔ rust `RLevel`）。
///
/// 顺序逐项 bit-exact 对应 Lean `Origin.TrendSixState`（bot/insideZ/aboveNo3B/aboveB3/belowNo3S/belowS3）
/// 与 §5 符号（⊥/I/U⁰/U¹/D⁰/D¹）。这是 parity 的命名锚——Lean 改构造子顺序 ⟹ 本表须随之改。
pub const SIX_STATE_CANONICAL: [(&str, &str); 6] = [
    ("⊥", "bot"),        // 无确认中枢
    ("I", "insideZ"),    // 中枢内
    ("U⁰", "aboveNo3B"), // 中枢上方·无三买
    ("U¹", "aboveB3"),   // 中枢上方·已有三买
    ("D⁰", "belowNo3S"), // 中枢下方·无三卖
    ("D¹", "belowS3"),   // 中枢下方·已有三卖
];

/// `RLevel` → canonical §5 符号（FULL §5；对齐 Lean `TrendSixState` 构造子语义）。
///
/// 六态 r 的 §5 符号投影。逐分支 bit-exact 对应 Lean `TrendSixState`：
/// Bot↔⊥, Inside↔I, AboveNo3B↔U⁰, AboveB3↔U¹, BelowNo3S↔D⁰, BelowS3↔D¹。
pub fn rlevel_to_symbol(r: RLevel) -> &'static str {
    match r {
        RLevel::Bot => "⊥",
        RLevel::Inside => "I",
        RLevel::AboveNo3B => "U⁰",
        RLevel::AboveB3 => "U¹",
        RLevel::BelowNo3S => "D⁰",
        RLevel::BelowS3 => "D¹",
    }
}

/// `RLevel` → Lean `TrendSixState` 构造子名（parity 锚：rust 态 ↔ Lean 态名）。
pub fn rlevel_to_lean_name(r: RLevel) -> &'static str {
    match r {
        RLevel::Bot => "bot",
        RLevel::Inside => "insideZ",
        RLevel::AboveNo3B => "aboveNo3B",
        RLevel::AboveB3 => "aboveB3",
        RLevel::BelowNo3S => "belowNo3S",
        RLevel::BelowS3 => "belowS3",
    }
}

/// 六态判定（薄包装 `rlevel_of`，契约锚 Lean `classifySixState`）。
///
/// 输入对齐 Lean `SixStateContext`（lastCenter/price/b3/s3）。返回值 = `rlevel_of` 的 `RLevel`
/// （bit-exact 镜像 Lean `classifySixState` 的 `TrendSixState`）。**不**新增逻辑——只是给 L2-A
/// 对照层一个与 Lean `classifySixState` 同名同签名的入口（消除「六态判定散落」的可读性缺口）。
pub fn classify_six_state(last_center: Option<Center>, price: Tick, b3: bool, s3: bool) -> RLevel {
    rlevel_of(&RContext {
        last_center,
        price,
        b3,
        s3,
    })
}

/// 信号位投影（薄包装 `endpoint_to_bsp`，契约锚 Lean `signalBitsOf`）。
///
/// 把端点情形投影为 {0,1}⁶ 信号位（`BspBits`）。**不**新增逻辑——对齐 Lean `signalBitsOf`
/// 从判据投影的语义（rust 侧判据已在 `EndpointSituation::is_first/is_second/is_third` 内）。
/// ★非互斥保留：若 `is_second() && is_third()`（V型反转）⟹ buy2=buy3=true 同时置位。
pub fn signal_bits_of(situ: &EndpointSituation) -> BspBits {
    endpoint_to_bsp(situ)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn center(zd: Tick, zg: Tick) -> Center {
        Center {
            zd,
            zg,
            dd: zd - 50,
            gg: zg + 50,
            start_index: 0,
            end_index: 12,
        }
    }

    // ── 六态 r parity（对齐 Lean TrendSixState witness_six_*，同口径中枢 [100,200]） ──

    /// parity：无中枢 ⟹ ⊥/bot（Lean `witness_six_bot`）。
    #[test]
    fn six_state_bot_parity() {
        let r = classify_six_state(None, 150, true, true);
        assert_eq!(r, RLevel::Bot);
        assert_eq!(rlevel_to_symbol(r), "⊥");
        assert_eq!(rlevel_to_lean_name(r), "bot");
    }

    /// parity：p=150 中枢内 ⟹ I/insideZ，不受 b3/s3 影响（Lean `witness_six_inside`）。
    #[test]
    fn six_state_inside_parity() {
        let c = center(100, 200);
        let r = classify_six_state(Some(c), 150, true, true);
        assert_eq!(r, RLevel::Inside);
        assert_eq!(rlevel_to_symbol(r), "I");
    }

    /// parity：p=250 上方·无三买 ⟹ U⁰/aboveNo3B（Lean `witness_six_aboveNo3B`）。
    #[test]
    fn six_state_above_no3b_parity() {
        let c = center(100, 200);
        let r = classify_six_state(Some(c), 250, false, false);
        assert_eq!(r, RLevel::AboveNo3B);
        assert_eq!(rlevel_to_symbol(r), "U⁰");
    }

    /// parity：p=250 上方·有三买 ⟹ U¹/aboveB3（Lean `witness_six_aboveB3`，above 按 b3 真裂两态）。
    #[test]
    fn six_state_above_b3_parity() {
        let c = center(100, 200);
        let r = classify_six_state(Some(c), 250, true, false);
        assert_eq!(r, RLevel::AboveB3);
        assert_eq!(rlevel_to_symbol(r), "U¹");
    }

    /// parity：p=50 下方·无三卖 ⟹ D⁰/belowNo3S（Lean `witness_six_belowNo3S`）。
    #[test]
    fn six_state_below_no3s_parity() {
        let c = center(100, 200);
        let r = classify_six_state(Some(c), 50, false, false);
        assert_eq!(r, RLevel::BelowNo3S);
        assert_eq!(rlevel_to_symbol(r), "D⁰");
    }

    /// parity：p=50 下方·有三卖 ⟹ D¹/belowS3（Lean `witness_six_belowS3`，below 按 s3 真裂两态）。
    #[test]
    fn six_state_below_s3_parity() {
        let c = center(100, 200);
        let r = classify_six_state(Some(c), 50, false, true);
        assert_eq!(r, RLevel::BelowS3);
        assert_eq!(rlevel_to_symbol(r), "D¹");
    }

    /// ★六态穷尽 parity（Lean `sixState_exhaustive`）：任意上下文落六态之一，且符号唯一。
    /// 同时验证 ⊥ 是真新增维度（无中枢恒 bot，对齐 Lean `no_center_maps_bot`）。
    #[test]
    fn six_state_exhaustive_and_bot_dimension() {
        let c = center(100, 200);
        let mut seen = std::collections::HashSet::new();
        for lc in [None, Some(c)] {
            for p in [-100, 50, 100, 150, 200, 250, 400] {
                for b3 in [true, false] {
                    for s3 in [true, false] {
                        let r = classify_six_state(lc, p, b3, s3);
                        // 落六态之一（穷尽）。
                        assert!(matches!(
                            r,
                            RLevel::Bot
                                | RLevel::Inside
                                | RLevel::AboveNo3B
                                | RLevel::AboveB3
                                | RLevel::BelowNo3S
                                | RLevel::BelowS3
                        ));
                        // ⊥ 真新增维度：无中枢 ⟹ 恒 Bot（不论 price/b3/s3）。
                        if lc.is_none() {
                            assert_eq!(r, RLevel::Bot, "无中枢恒 ⊥（Lean no_center_maps_bot）");
                        }
                        seen.insert(rlevel_to_symbol(r));
                    }
                }
            }
        }
        // 六态全部可达（partition 非退化，对照 Lean 六构造子）。
        assert_eq!(seen.len(), 6, "六态全部可达（partition 非退化）");
    }

    /// canonical 名称表与 RLevel 投影一致（Lean `TrendSixState` 构造子顺序锚）。
    #[test]
    fn canonical_name_table_consistent() {
        let order = [
            RLevel::Bot,
            RLevel::Inside,
            RLevel::AboveNo3B,
            RLevel::AboveB3,
            RLevel::BelowNo3S,
            RLevel::BelowS3,
        ];
        for (i, r) in order.iter().enumerate() {
            assert_eq!(rlevel_to_symbol(*r), SIX_STATE_CANONICAL[i].0);
            assert_eq!(rlevel_to_lean_name(*r), SIX_STATE_CANONICAL[i].1);
        }
    }

    // ── 信号位 b ∈ {0,1}⁶ parity（对齐 Lean signalBits_*） ──

    fn situ(
        after_first: bool,
        pullback: bool,
        left: bool,
        retrace: bool,
        below: bool,
        sell: bool,
    ) -> EndpointSituation {
        EndpointSituation {
            after_first_buy: after_first,
            is_pullback_end: pullback,
            left_center: left,
            retrace_not_reenter: retrace,
            below_last_center: below,
            is_sell_side: sell,
        }
    }

    /// ★信号位真 6 维·2B/3B 可同时为 1（Lean `signalBits_2b3b_coexist`，maimai:170 V型反转）。
    /// V型反转端点：1买后(after_first) 回调结束(pullback)=2买 ∧ 离开中枢(left) 回试不入(retrace)=3买。
    #[test]
    fn signal_2b3b_coexist_parity() {
        let v_reversal = situ(true, true, true, true, false, false);
        let bits = signal_bits_of(&v_reversal);
        assert!(bits.buy2, "2B 置位（V型反转：1买后回调结束）");
        assert!(bits.buy3, "3B 置位（V型反转：离开中枢回试不入）");
        // ★非互斥见证：buy2 ∧ buy3 同时 true（互斥 sum 无法表达此态，Lean signalBits_not_collapsible_to_sum）。
        assert!(
            bits.buy2 && bits.buy3,
            "2B/3B 共存 ⟹ 信号位真 6 维（非塌缩为互斥 sum）"
        );
    }

    /// ★信号位 1B/2B 不可重合（Lean `signalBits_1b2b_exclusive`，maimai:172 时间互斥）。
    /// 1买判据要求未离开中枢(below_last_center ∧ !left_center ∧ !after_first_buy)；
    /// 2买判据要求 after_first_buy=true ⟹ 两者 after_first_buy 取值矛盾 ⟹ 不可同时置位。
    #[test]
    fn signal_1b2b_exclusive_parity() {
        // 穷举所有端点情形（买侧），断言 buy1 ∧ buy2 永不同时为 1。
        for af in [true, false] {
            for pb in [true, false] {
                for lf in [true, false] {
                    for rt in [true, false] {
                        for bl in [true, false] {
                            let bits = signal_bits_of(&situ(af, pb, lf, rt, bl, false));
                            assert!(
                                !(bits.buy1 && bits.buy2),
                                "1B/2B 不可重合（after_first_buy 互斥，Lean signalBits_1b2b_exclusive）"
                            );
                        }
                    }
                }
            }
        }
    }

    /// 信号位状态空间 = subset（{0,1}⁶=64 态），非互斥 sum（6 态）的镜像确认。
    /// 至少存在一个两位同置的合法向量（2B/3B），证明状态空间 > 6（Lean signalBits_not_collapsible_to_sum）。
    #[test]
    fn signal_bits_is_subset_not_sum() {
        let bits = signal_bits_of(&situ(true, true, true, true, false, false));
        let set_count = [
            bits.buy1, bits.buy2, bits.buy3, bits.sell1, bits.sell2, bits.sell3,
        ]
        .iter()
        .filter(|x| **x)
        .count();
        assert!(
            set_count >= 2,
            "存在 ≥2 位同置的合法信号位 ⟹ subset（非互斥 sum 恰一格）"
        );
    }

    /// 卖侧镜像：2S/3S 可重合（信号位非互斥对买卖对称，maimai:176）。
    #[test]
    fn signal_2s3s_coexist_mirror() {
        let v_reversal_sell = situ(true, true, true, true, false, true);
        let bits = signal_bits_of(&v_reversal_sell);
        assert!(
            bits.sell2 && bits.sell3,
            "2S/3S 共存（卖侧镜像，maimai:176）"
        );
    }
}
