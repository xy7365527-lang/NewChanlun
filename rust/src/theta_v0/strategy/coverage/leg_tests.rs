use super::super::super::interp::Buckets;
use super::super::super::risk::{
    gross_notional, leverage_metrics, leverage_ok, net_notional, LeverageCaps, VoiceNotional,
};
use super::super::test_support::*;
use super::*;
use crate::theta_v0::types::Direction;

/// 验收点 2：ReverseOpen 合法性断言——`w_dir(role.v==ReverseOpen) ≡ 1.0`（§7.5 s_g=s_α 定理 1）。
/// 须在**所有三套预注册**下成立（CASE 1 优先于 CASE 3 查表，不被任何套缩放/归零）。
#[test]
fn w_dir_reverse_open_exempt_across_all_presets() {
    let presets = vec![
        ThetaDirPreset::Neutral,
        ThetaDirPreset::Follow {
            eta_adv: vec![0.5, 0.5, 0.5],
        },
        ThetaDirPreset::Adversary {
            eta_same: vec![0.5, 0.5, 0.5],
        },
    ];
    let mut cfg = VoiceConfig::default();
    for depth in 0..3u32 {
        for delta in [Dir::Plus, Dir::Minus] {
            // ReverseOpen 腿：18 类中所有 h（First/SameFollow/SameReverse）× ReverseOpen × ±δ。
            for h in [
                Horizontal::First,
                Horizontal::SameFollow,
                Horizontal::SameReverse,
            ] {
                let sd = role(h, Vertical::ReverseOpen, delta);
                for preset in &presets {
                    cfg.theta_dir = preset.clone();
                    let w = dir_weight(&sd, depth, &cfg);
                    assert_eq!(
                            w, 1.0,
                            "ReverseOpen w_dir≠1.0 破坏 §7.5 s_g=s_α (h={:?} δ={:?} depth={} preset={:?})",
                            h, delta, depth, preset
                        );
                }
            }
        }
    }
}

/// 验收点 3：neutral 套 bit-exact 自检——Θ_dir_neutral 下 w_dir≡1.0 对**所有 18 类角色** ×
/// 所有 depth 成立 ⟹ `leg_target` 输出 == v0 ⟹ p̃/p\*/订单/ledger 全不变（prereg §6.2）。
/// 这是「neutral 套 OOS == v0 基线 af8910d062」的单元级数学根因。
#[test]
fn w_dir_neutral_is_identity_for_all_18_roles() {
    let cfg = VoiceConfig::default(); // theta_dir = Neutral
    assert!(matches!(cfg.theta_dir, ThetaDirPreset::Neutral));
    for depth in 0..3u32 {
        for h in [
            Horizontal::First,
            Horizontal::SameFollow,
            Horizontal::SameReverse,
        ] {
            for v in [
                Vertical::Ambient,
                Vertical::FollowParent,
                Vertical::ReverseOpen,
            ] {
                for delta in [Dir::Plus, Dir::Minus] {
                    let r = role(h, v, delta);
                    assert_eq!(
                        dir_weight(&r, depth, &cfg),
                        1.0,
                        "neutral 套 w_dir≠1.0 破坏 bit-exact==v0 (h={:?} v={:?} δ={:?} depth={})",
                        h,
                        v,
                        delta,
                        depth
                    );
                }
            }
        }
    }
}

/// ★GPT 命名冲突裁决 §六：V 回三分类后，sign=−1 槽经 V 路径不可达（ReverseOpen 整体豁免 CASE 1）。
/// η_adv 经 V 路径不可达（Follow sign=−1 槽）；η_same 经 V 路径**可达**（Adversary FollowParent sign=+1 槽）。
#[test]
fn w_dir_sign_neg_slot_unreachable_via_v_axis() {
    let mut cfg = VoiceConfig::default();
    // ReverseOpen（反父方向，含同级别+次级别）：CASE 1 豁免恒 1.0，不论 preset（§7.5 s_g=s_α）。
    let sd = role(Horizontal::First, Vertical::ReverseOpen, Dir::Minus);
    assert_eq!(dir_weight(&sd, 0, &cfg), 1.0, "ReverseOpen Neutral 豁免");
    cfg.theta_dir = ThetaDirPreset::Follow { eta_adv: vec![0.5] };
    assert_eq!(
        dir_weight(&sd, 0, &cfg),
        1.0,
        "ReverseOpen Follow 仍豁免——η_adv 经 V 路径不可达（GPT §六）"
    );
    cfg.theta_dir = ThetaDirPreset::Adversary {
        eta_same: vec![0.6],
    };
    assert_eq!(dir_weight(&sd, 0, &cfg), 1.0,
            "ReverseOpen Adversary 仍豁免——ReverseOpen CASE 1 整体豁免（η_same 对 FollowParent sign=+1 槽可达）");
}

/// w_grade（prereg-wg (c) 因子化）：消费 role.grade，G 轴 sizing 权重。
#[test]
fn w_grade_consumes_g_axis() {
    let mut cfg = VoiceConfig::default();
    let sl_same = OperationRole {
        h: Horizontal::First,
        v: Vertical::ReverseOpen,
        delta: Dir::Minus,
        grade: GradeRel::SameLevel,
    };
    let sl_sub = OperationRole {
        h: Horizontal::First,
        v: Vertical::ReverseOpen,
        delta: Dir::Minus,
        grade: GradeRel::SubLevel,
    };
    // default [1.0, 1.0] identity ⟹ SameLevel/SubLevel 都返 1.0（bit-exact）。
    assert_eq!(
        w_grade(&sl_same, &cfg),
        1.0,
        "default w_grade[SameLevel]=1.0 identity"
    );
    assert_eq!(
        w_grade(&sl_sub, &cfg),
        1.0,
        "default w_grade[SubLevel]=1.0 identity"
    );
    // 非 identity ⟹ SameLevel/SubLevel 返不同值（G 轴独立区分）。
    cfg.w_grade = [0.8, 0.4];
    assert_eq!(w_grade(&sl_same, &cfg), 0.8, "w_grade[SameLevel]=0.8");
    assert_eq!(w_grade(&sl_sub, &cfg), 0.4, "w_grade[SubLevel]=0.4");
    // C4 实质担忧：同级别反父 vs 次级别反父在 sizing 区分（两者 V 轴同 ReverseOpen）。
    assert_ne!(
        w_grade(&sl_same, &cfg),
        w_grade(&sl_sub, &cfg),
        "G 轴区分 SameLevel AgainstParent vs SubLevel ReverseOpen（V 轴不区分）"
    );
    // 与 theta_dir preset 正交（G 轴独立消费，不绑 Follow/Adversary）。
    cfg.theta_dir = ThetaDirPreset::Follow { eta_adv: vec![0.5] };
    assert_eq!(
        w_grade(&sl_same, &cfg),
        0.8,
        "w_grade 与 theta_dir preset 正交"
    );
}

/// leg_target sizing 公式含 w_grade 因子（prereg-wg (c)：w = depth × dir × w_grade）。
/// default [1.0,1.0] bit-exact；非 identity ⟹ root_leg（grade=SameLevel）units 按倍数缩放。
#[test]
fn leg_target_sizing_includes_w_grade_factor() {
    let l1 = nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up]);
    let tower = rc_tower(vec![Vec::new(), vec![l1]]);
    let elements = extract_elements(&tower);
    // default w_grade=[1.0,1.0] ⟹ root_leg units=600（bit-exact == v0）。
    let c = cfg();
    let root_leg = leg_target(&elements, 0, 1000.0, &c);
    assert!(
        (root_leg.units - 600.0).abs() < 1e-9,
        "default w_grade identity ⟹ units=600（bit-exact）"
    );
    assert_eq!(
        root_leg.role.grade,
        GradeRel::SameLevel,
        "root leg grade=SameLevel"
    );
    // w_grade[SameLevel]=2.0 ⟹ root_leg units=600×2.0=1200（leg_target 真的乘了 w_grade）。
    let mut c2 = cfg();
    c2.w_grade = [2.0, 1.0];
    let root_leg2 = leg_target(&elements, 0, 1000.0, &c2);
    assert!(
        (root_leg2.units - 1200.0).abs() < 1e-9,
        "w_grade[SameLevel]=2.0 ⟹ units=600×2.0=1200"
    );
}

/// CASE 3 分级查表行为（Follow/Adversary 两套的非中性槽生效 + 根级豁免）。
#[test]
fn w_dir_case3_lookup_and_root_exemption() {
    let mut cfg = VoiceConfig::default();
    // FollowParent（δ=σ_higher，sign=+1，CASE 3 可达）。
    let fp_plus = role(Horizontal::First, Vertical::FollowParent, Dir::Plus);
    let fp_minus = role(Horizontal::First, Vertical::FollowParent, Dir::Minus);
    // Ambient（σ_higher=0，CASE 2 根级豁免，须 1.0 不论套）。
    let amb = role(Horizontal::First, Vertical::Ambient, Dir::Plus);

    // Follow 套：顺上级(sign=+1)=1.0，逆上级(sign=−1)=η_adv=0.7。
    cfg.theta_dir = ThetaDirPreset::Follow {
        eta_adv: vec![0.7, 0.7, 0.7],
    };
    assert_eq!(dir_weight(&fp_plus, 0, &cfg), 1.0, "Follow 顺上级保权");
    assert_eq!(
        dir_weight(&amb, 0, &cfg),
        1.0,
        "根级 Ambient 豁免（CASE 2）"
    );

    // Adversary 套：顺上级(sign=+1)=η_same=0.6，逆上级(sign=−1)=1.0。
    cfg.theta_dir = ThetaDirPreset::Adversary {
        eta_same: vec![0.6, 0.6, 0.6],
    };
    assert_eq!(
        dir_weight(&fp_plus, 0, &cfg),
        0.6,
        "Adversary 顺上级降权 η_same"
    );
    assert_eq!(
        dir_weight(&amb, 0, &cfg),
        1.0,
        "根级 Ambient 豁免（CASE 2， adversary）"
    );

    // FollowParent：δ=σ_higher ⟹ sign=+1（δ=Plus/Minus 均同向于各自 σ_higher）。
    // sign=−1 槽经 V 路径不可达 ⟹ 见 w_dir_sign_neg_slot_unreachable_via_v_axis。
    cfg.theta_dir = ThetaDirPreset::Adversary {
        eta_same: vec![0.6, 0.6, 0.6],
    };
    assert_eq!(
        dir_weight(&fp_minus, 0, &cfg),
        0.6,
        "FollowParentδ=Minus 仍 sign=+1（σ_higher=−δ 同号）"
    );

    // 越界 level 视 1.0（保权，不意外零化）。
    cfg.theta_dir = ThetaDirPreset::Adversary {
        eta_same: vec![0.6],
    }; // 只给 depth 0
    assert_eq!(dir_weight(&fp_plus, 5, &cfg), 1.0, "越界 level 保权 1.0");
}

/// 腿方向 = ε_e（M17 σ_{ν(e)}=ε_e）；单位 = base × w_depth（M28 深度权重）。
#[test]
fn leg_target_side_and_depth_weighted_units() {
    let l1 = nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up]);
    let tower = rc_tower(vec![Vec::new(), vec![l1]]);
    let elements = extract_elements(&tower);
    let c = cfg();
    // 根腿（depth 0）：side=ε_root=Long，units=1000×w[0]=1000×0.60=600。
    let root_leg = leg_target(&elements, 0, 1000.0, &c);
    assert_eq!(root_leg.side, VoiceSide::Long);
    assert!(
        (root_leg.units - 600.0).abs() < 1e-9,
        "根 depth 0 权重 0.60"
    );
    // 顶层根角色：(First, Ambient, Plus)——去根化无 RootRole。
    assert_eq!(
        root_leg.role,
        role(Horizontal::First, Vertical::Ambient, Dir::Plus)
    );
    // 子腿（depth 1）：units=1000×w[1]=1000×0.30=300。
    let sub_leg = leg_target(&elements, 1, 1000.0, &c);
    assert!((sub_leg.units - 300.0).abs() < 1e-9, "子 depth 1 权重 0.30");
    // 子1 是短差（V=ReverseOpen），方向 Short（短差反向子声部腿）。
    let reverse_open_leg = leg_target(&elements, 2, 1000.0, &c);
    assert_eq!(reverse_open_leg.side, VoiceSide::Short);
    assert_eq!(reverse_open_leg.role.v, Vertical::ReverseOpen);
}

// ── §6 净额执行（毛腿 → 净持仓，对冲腿部分抵消父仓）──────────────────────────

/// ★净额合并：多腿+空腿净额抵消（ReverseOpen 空腿抵消父多腿，M11 短差对冲）。
#[test]
fn net_target_units_hedges_parent_with_reverse_open() {
    // 根多腿 600 + 子0 顺势多腿 300 + 子1 短差空腿 300 ⟹ 净 = 600+300−300 = 600。
    let legs = vec![
        LegTarget {
            e_idx: 0,
            side: VoiceSide::Long,
            units: 600.0,
            role: role(Horizontal::First, Vertical::Ambient, Dir::Plus),
        },
        LegTarget {
            e_idx: 1,
            side: VoiceSide::Long,
            units: 300.0,
            role: role(Horizontal::First, Vertical::FollowParent, Dir::Plus),
        },
        LegTarget {
            e_idx: 2,
            side: VoiceSide::Short,
            units: 300.0,
            role: role(Horizontal::SameReverse, Vertical::ReverseOpen, Dir::Minus),
        },
    ];
    let net = net_target_units(&legs);
    assert!(
        (net - 600.0).abs() < 1e-9,
        "净 = 600+300−300（短差空腿对冲）"
    );
    // 毛敞口 = |600|+|300|+|300| = 1200（双开毛敞口 > 净额 600，net_le_gross）。
    let gross = gross_target_units(&legs);
    assert!(
        (gross - 1200.0).abs() < 1e-9,
        "毛敞口 1200 > 净 600（双开毛收益级覆盖）"
    );
    assert!(net.abs() <= gross, "净 ≤ 毛（risk.rs net_le_gross）");
}

/// 精确对冲：单位数相同的多空腿 ⟹ 净额 0（毛敞口满额，net_le_gross 极端情形）。
#[test]
fn net_zero_when_hedged_equal() {
    let legs = vec![
        LegTarget {
            e_idx: 0,
            side: VoiceSide::Long,
            units: 400.0,
            role: role(Horizontal::First, Vertical::Ambient, Dir::Plus),
        },
        LegTarget {
            e_idx: 1,
            side: VoiceSide::Short,
            units: 400.0,
            role: role(Horizontal::SameReverse, Vertical::ReverseOpen, Dir::Minus),
        },
    ];
    assert!((net_target_units(&legs)).abs() < 1e-9, "等单位多空 ⟹ 净 0");
    assert!(
        (gross_target_units(&legs) - 800.0).abs() < 1e-9,
        "毛 800（双开满额）"
    );
}

/// ★P0-3 overlay_net_delta：ΔN_t = ReverseOpen 腿净贡献 = net(全腿)−net(剔 ReverseOpen)。
#[test]
fn overlay_net_delta_is_reverse_open_contribution() {
    // 根多 600 + 顺势子多 300 + 短差子空 300 ⟹ 全净=600，剔短差净=900 ⟹ ΔN=600−900=−300。
    let legs = vec![
        LegTarget {
            e_idx: 0,
            side: VoiceSide::Long,
            units: 600.0,
            role: role(Horizontal::First, Vertical::Ambient, Dir::Plus),
        },
        LegTarget {
            e_idx: 1,
            side: VoiceSide::Long,
            units: 300.0,
            role: role(Horizontal::First, Vertical::FollowParent, Dir::Plus),
        },
        LegTarget {
            e_idx: 2,
            side: VoiceSide::Short,
            units: 300.0,
            role: role(Horizontal::SameReverse, Vertical::ReverseOpen, Dir::Minus),
        },
    ];
    let dn = overlay_net_delta(&legs);
    assert!((dn + 300.0).abs() < 1e-9, "ΔN = 短差空腿净贡献 = −300");
    // 恒等式：ΔN = net(全腿) − net(剔 ReverseOpen)。
    let net_base: f64 = legs
        .iter()
        .filter(|l| l.role.v != Vertical::ReverseOpen)
        .map(|l| {
            if l.side == VoiceSide::Long {
                l.units
            } else {
                -l.units
            }
        })
        .sum();
    assert!(
        (net_target_units(&legs) - net_base - dn).abs() < 1e-9,
        "ΔN 恒等式"
    );
    // 无 ReverseOpen ⟹ ΔN=0（overlay 不改净头寸，多空对冲.pdf b2 净额不可见）。
    assert!(
        overlay_net_delta(&legs[..2]).abs() < 1e-9,
        "无短差腿 ⟹ ΔN=0"
    );
}

// ── G7 毛头寸约束（#122 终裁 + codex decide 5b46：逐根 KKT 投影）─────────────

/// ★单根退化 = 等比例缩放（decide 5b46：方案A是单根退化情形）。真嵌套塔（父+顺势子+短差子），
/// 默认 VoiceConfig（disable_reverse_open=false）——多空双开生产默认场景。
#[test]
fn gross_cap_single_root_scales_proportionally() {
    let l1 = nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up]);
    let tower = rc_tower(vec![Vec::new(), vec![l1]]);
    let elements = extract_elements(&tower);
    let c = cfg();
    // legs = [根 600 L, 顺势子 300 L, 短差子 300 S] ⟹ 净 600、毛 1200。
    let mut legs: Vec<LegTarget> = [0usize, 1, 2]
        .iter()
        .map(|&i| leg_target(&elements, i, 1000.0, &c))
        .collect();
    let risk = RiskConfig {
        enforce_gross_cap: true,
        ..RiskConfig::default()
    }; // γ=1 ⟹ Ḡ=1000
    let zeroed = apply_gross_cap(&ElementView::new(&elements), &mut legs, 1000.0, &risk);
    assert!(zeroed.is_empty(), "部分缩放无零化腿");
    // 单根 ⟹ c = Ḡ/G = 1000/1200 = 5/6，全腿等比例（比率约束族内 LexArgmin 主键投影）。
    assert!((legs[0].units - 500.0).abs() < 1e-9, "根 600×5/6");
    assert!((legs[1].units - 250.0).abs() < 1e-9, "子 300×5/6");
    assert!(
        (legs[2].units - 250.0).abs() < 1e-9,
        "短差 300×5/6（同单位数比率保持）"
    );
    assert!(
        (gross_target_units(&legs) - 1000.0).abs() < 1e-9,
        "毛投影到边界 Ḡ"
    );
    assert!(
        (net_target_units(&legs) - 500.0).abs() < 1e-9,
        "净随比例缩 600×5/6"
    );
}

/// ★#122 终裁反例坐实：等单位多空双开净 p̃=0（净检查恒过）、毛 G=1200 超限——腿层毛约束
/// 正确捕获并压缩到 Ḡ，净保持 0（缩放不破坏对冲结构）。
#[test]
fn gross_cap_constrains_hedged_net_zero() {
    let els = vec![
        gce(0, 0, None, VoiceSide::Long),
        gce(0, 1, None, VoiceSide::Short),
    ];
    let mut legs = vec![
        gleg(0, VoiceSide::Long, 600.0),
        gleg(1, VoiceSide::Short, 600.0),
    ];
    assert!(
        (net_target_units(&legs)).abs() < 1e-9,
        "前提：净 0（净检查恒过）"
    );
    assert!(
        (gross_target_units(&legs) - 1200.0).abs() < 1e-9,
        "前提：毛 1200"
    );
    let risk = RiskConfig {
        enforce_gross_cap: true,
        gamma: 0.5,
        ..RiskConfig::default()
    }; // Ḡ=500
    assert!(apply_gross_cap(&ElementView::new(&els), &mut legs, 1000.0, &risk).is_empty());
    // 两根等阈值 t=1200 ⟹ 同 c = 1−700/1200 = 5/12 ⟹ 各 250。
    assert!((legs[0].units - 250.0).abs() < 1e-9);
    assert!((legs[1].units - 250.0).abs() < 1e-9);
    assert!(
        (gross_target_units(&legs) - 500.0).abs() < 1e-9,
        "毛压到 Ḡ=500"
    );
    assert!(
        (net_target_units(&legs)).abs() < 1e-9,
        "净仍 0（对冲结构保持）"
    );
}

/// ★多根非均匀 water-filling（decide 5b46 方案B 核心）：子树内比率保持（同 c_r），跨根
/// 按阈值 t_r=2A_r/G_r 非均匀分配（大子树 c 高）——非全腿同一比例。
#[test]
fn gross_cap_multi_root_waterfilling_nonuniform() {
    // 根0（id (1,0)）：父腿 600 L + 短差子腿 300 S（parent_id→根0）⟹ G_r0=900, A_r0=45e4, t0=1000。
    // 根1（id (1,1)）：单腿 300 L ⟹ G_r1=300, A_r1=9e4, t1=600。
    let root0 = ElementId {
        level: 1,
        ordinal: 0,
    };
    let els = vec![
        gce(1, 0, None, VoiceSide::Long),
        gce(1, 1, None, VoiceSide::Long),
        gce(0, 0, Some(root0), VoiceSide::Short),
    ];
    let mut legs = vec![
        gleg(0, VoiceSide::Long, 600.0),
        gleg(1, VoiceSide::Long, 300.0),
        gleg(2, VoiceSide::Short, 300.0), // 根0 的子腿（分组沿 parent_id 归根0）
    ];
    let risk = RiskConfig {
        enforce_gross_cap: true,
        gamma: 0.6,
        ..RiskConfig::default()
    }; // Ḡ=600
    assert!(apply_gross_cap(&ElementView::new(&els), &mut legs, 1000.0, &risk).is_empty());
    // μ = (1200−600)/(900/1000+300/600) = 600/1.4 = 3000/7；c0 = 1−μ/1000 = 4/7；c1 = 1−μ/600 = 2/7。
    assert!(
        (legs[0].units - 600.0 * 4.0 / 7.0).abs() < 1e-9,
        "根0 父腿 ×4/7"
    );
    assert!(
        (legs[2].units - 300.0 * 4.0 / 7.0).abs() < 1e-9,
        "根0 子腿同 c（子树内比率保持）"
    );
    assert!(
        (legs[1].units - 300.0 * 2.0 / 7.0).abs() < 1e-9,
        "根1 ×2/7（非均匀：小子树缩更多）"
    );
    assert!(
        (gross_target_units(&legs) - 600.0).abs() < 1e-9,
        "毛 = Ḡ（约束取等）"
    );
}

/// water-filling 零化分支：cap 足够小时低阈值根被整体归零（c_r=0），高阈值根承接余量。
#[test]
fn gross_cap_waterfilling_zeroes_low_threshold_root() {
    let els = vec![
        gce(1, 0, None, VoiceSide::Long),
        gce(1, 1, None, VoiceSide::Long),
    ];
    let mut legs = vec![
        gleg(0, VoiceSide::Long, 600.0),
        gleg(1, VoiceSide::Long, 300.0),
    ];
    let risk = RiskConfig {
        enforce_gross_cap: true,
        gamma: 0.2,
        ..RiskConfig::default()
    }; // Ḡ=200
    let zeroed = apply_gross_cap(&ElementView::new(&els), &mut legs, 1000.0, &risk);
    // t0=1200, t1=600；全活 μ=700>600 ⟹ 根1 零化；重解 μ=800≤1200 ⟹ c0=1/3。
    assert!((legs[0].units - 200.0).abs() < 1e-9, "高阈值根 600×1/3 = Ḡ");
    assert_eq!(legs[1].units, 0.0, "低阈值根整体归零");
    assert!((gross_target_units(&legs) - 200.0).abs() < 1e-9);
    assert_eq!(zeroed, vec![1], "零化腿 e_idx 上报（幽灵腿防护消费）");
}

/// 边界：Ḡ=0（γ=0）⟹ 全腿归零（毛可行集退化 {0}）；G≤Ḡ / G=0 ⟹ 逐位不动。
#[test]
fn gross_cap_boundary_zero_cap_and_no_trigger() {
    let els = vec![
        gce(1, 0, None, VoiceSide::Long),
        gce(1, 1, None, VoiceSide::Short),
    ];
    // Ḡ=0：全零 + 全部上报零化。
    let mut legs = vec![
        gleg(0, VoiceSide::Long, 600.0),
        gleg(1, VoiceSide::Short, 300.0),
    ];
    let risk0 = RiskConfig {
        enforce_gross_cap: true,
        gamma: 0.0,
        ..RiskConfig::default()
    };
    let zeroed = apply_gross_cap(&ElementView::new(&els), &mut legs, 1000.0, &risk0);
    assert!(legs.iter().all(|l| l.units == 0.0), "Ḡ=0 ⟹ 全腿归零");
    assert_eq!(zeroed, vec![0, 1], "Ḡ=0 ⟹ 全腿上报零化");
    // G ≤ Ḡ：逐位不动（bit-exact ==，非近似）。
    let orig = vec![
        gleg(0, VoiceSide::Long, 600.0),
        gleg(1, VoiceSide::Short, 300.0),
    ];
    let mut legs2 = orig.clone();
    let risk2 = RiskConfig {
        enforce_gross_cap: true,
        gamma: 2.0,
        ..RiskConfig::default()
    }; // Ḡ=2000>900
    assert!(apply_gross_cap(&ElementView::new(&els), &mut legs2, 1000.0, &risk2).is_empty());
    assert_eq!(legs2, orig, "未超限 ⟹ 不缩放（逐位相同）");
    // G=0（空腿集）：no-op 不 panic。
    let mut empty: Vec<LegTarget> = vec![];
    assert!(apply_gross_cap(&ElementView::new(&els), &mut empty, 1000.0, &risk0).is_empty());
}

/// ★生产接线集成：coverage_step_from_buckets 默认路径（None / enforce=false）逐位不变；
/// enforce=true 且触发 ⟹ p̃ 被腿层毛约束压缩（净额折叠前，事后诊断门不可能做到）。
#[test]
fn gross_cap_integration_default_bit_exact_enabled_scales() {
    let reg = super::super::super::persistent::PersistentRegistry::new();
    let buckets = Buckets {
        close: vec![],
        open: vec![
            cand(0, 1, VoiceSide::Long, 0),
            cand(1, 2, VoiceSide::Long, 1),
            cand(2, 3, VoiceSide::Short, 2),
        ],
        record: vec![],
    };
    let els = flat_elements(&buckets.open);
    // 基线（risk=None）：三根腿各 600 ⟹ 净 600+600−600=600，毛 1800。
    let (_, p_none) = coverage_step_from_buckets(
        view_split(&els, 0),
        &[],
        0.0,
        &buckets,
        1000.0,
        &cfg(),
        None,
        &reg,
    );
    assert!((p_none - 600.0).abs() < 1e-9);
    // enforce_gross_cap=false（default）：与 None 逐位相同（约束未配置=不激活）。
    let risk_off = RiskConfig::default();
    let (_, p_off) = coverage_step_from_buckets(
        view_split(&els, 0),
        &[],
        0.0,
        &buckets,
        1000.0,
        &cfg(),
        Some(&risk_off),
        &reg,
    );
    assert_eq!(p_off, p_none, "default 不激活 ⟹ bit-exact");
    // enforce=true, γ=0.9 ⟹ Ḡ=900 < 1800：三等根 c=0.5 ⟹ 净 600×0.5=300。
    let risk_on = RiskConfig {
        enforce_gross_cap: true,
        gamma: 0.9,
        ..RiskConfig::default()
    };
    let (active_on, p_on) = coverage_step_from_buckets(
        view_split(&els, 0),
        &[],
        0.0,
        &buckets,
        1000.0,
        &cfg(),
        Some(&risk_on),
        &reg,
    );
    assert!((p_on - 300.0).abs() < 1e-9, "毛约束在净额折叠前压缩 p̃");
    // 部分缩放（c=0.5>0）真实开仓 ⟹ 腿保留在 next_active（账本身份与订单效果一致）。
    assert_eq!(active_on.len(), 3, "部分缩放腿保留");
}

/// ★幽灵腿防护（裁定4，#124 force_flat 同款模式禁止复制）：被毛约束整体零化（Ḡ=0）的
/// 开仓腿不入 next_active——目标 0 的开仓从未建仓，不得作为持仓身份跨 bar 延续。
#[test]
fn gross_cap_zeroed_open_legs_do_not_enter_active_set() {
    let reg = super::super::super::persistent::PersistentRegistry::new();
    let buckets = Buckets {
        close: vec![],
        open: vec![
            cand(0, 1, VoiceSide::Long, 0),
            cand(1, 2, VoiceSide::Short, 1),
        ],
        record: vec![],
    };
    let els = flat_elements(&buckets.open);
    // Ḡ=0（γ=0，enforce=true）：全部开仓腿零化 ⟹ p̃=0 且 next_active 空（无幽灵腿）。
    let risk0 = RiskConfig {
        enforce_gross_cap: true,
        gamma: 0.0,
        ..RiskConfig::default()
    };
    let (active, p) = coverage_step_from_buckets(
        view_split(&els, 0),
        &[],
        0.0,
        &buckets,
        1000.0,
        &cfg(),
        Some(&risk0),
        &reg,
    );
    assert_eq!(p, 0.0, "Ḡ=0 ⟹ p̃=0");
    assert!(
        active.is_empty(),
        "零化开仓腿不入 next_active（从未建仓 ⟹ 无持仓身份延续）"
    );
}

// ── §3 活动集递归原语（λ_e 区间，Lean M16 对齐——非生产入场，见 §3 GAP-5 note）已在上方测 ──

// ════════════════════════════════════════════════════════════════════════════
//  §M4 级别帽（#783 返工：clamp 前移到投影前）apply_level_cap 单测
// ════════════════════════════════════════════════════════════════════════════

/// 超帽级别被按 `clamped_ℓ/net_ℓ` 逐级等比缩放，未超帽级别原样透传；binding 计数逐级。
#[test]
fn level_cap_scales_binding_levels_proportionally() {
    let els = vec![
        gce(0, 0, None, VoiceSide::Long),
        gce(1, 0, None, VoiceSide::Long),
        gce(1, 1, None, VoiceSide::Long),
    ];
    // net_0 = 80，net_1 = 6+4 = 10；cap_0 = 0.5·1.0·100 = 50、cap_1 = 0.1·1.0·100 = 10。
    let mut legs = vec![
        gleg(0, VoiceSide::Long, 80.0),
        gleg(1, VoiceSide::Long, 6.0),
        gleg(2, VoiceSide::Long, 4.0),
    ];
    let risk = RiskConfig {
        level_weights: vec![0.5, 0.1],
        ..RiskConfig::default()
    };
    let stats = apply_level_cap(&ElementView::new(&els), &mut legs, 100.0, &risk);
    assert!((legs[0].units - 50.0).abs() < 1e-9, "level0 80→50");
    assert_eq!(legs[1].units, 6.0, "level1 未超帽原样");
    assert_eq!(legs[2].units, 4.0, "level1 未超帽原样");
    assert_eq!(stats.n_binding_levels, 1);
    assert_eq!(stats.n_levels, 2);
    assert!(
        (net_target_units(&legs) - 60.0).abs() < 1e-9,
        "账户层净目标 = Σ 裁剪后各级净额"
    );
}

/// 负向对称裁剪：空腿各级净额被对称裁到 ±cap_ℓ，逐级因子同号（重内单向后各级同号）。
#[test]
fn level_cap_clips_short_levels_symmetrically() {
    let els = vec![
        gce(0, 0, None, VoiceSide::Short),
        gce(1, 0, None, VoiceSide::Short),
    ];
    let mut legs = vec![
        gleg(0, VoiceSide::Short, 80.0),
        gleg(1, VoiceSide::Short, 30.0),
    ];
    let risk = RiskConfig {
        level_weights: vec![0.5, 0.1],
        ..RiskConfig::default()
    };
    let stats = apply_level_cap(&ElementView::new(&els), &mut legs, 100.0, &risk);
    assert!((legs[0].units - 50.0).abs() < 1e-9, "level0 −80→−50");
    assert!((legs[1].units - 10.0).abs() < 1e-9, "level1 −30→−10");
    assert_eq!(stats.n_binding_levels, 2);
    assert_eq!(stats.n_levels, 2);
}

/// 未超帽 ⟹ 逐位不动（bit-exact ==，非近似）。
#[test]
fn level_cap_non_binding_is_bit_exact() {
    let els = vec![
        gce(0, 0, None, VoiceSide::Long),
        gce(1, 0, None, VoiceSide::Long),
    ];
    let orig = vec![
        gleg(0, VoiceSide::Long, 30.0),
        gleg(1, VoiceSide::Long, 5.0),
    ];
    let mut legs = orig.clone();
    let risk = RiskConfig {
        level_weights: vec![0.5, 0.1],
        ..RiskConfig::default()
    };
    let stats = apply_level_cap(&ElementView::new(&els), &mut legs, 100.0, &risk);
    assert_eq!(legs, orig, "未超帽 ⟹ 不缩放");
    assert_eq!(stats.n_binding_levels, 0);
}

/// 表外级别权重 ⟹ cap_ℓ=0 ⟹ 该级腿归零（「该级别禁止持仓」非「未启用」）。
#[test]
fn level_cap_zero_weight_zeroes_unweighted_level() {
    let els = vec![
        gce(0, 0, None, VoiceSide::Long),
        gce(1, 0, None, VoiceSide::Long),
    ];
    let mut legs = vec![
        gleg(0, VoiceSide::Long, 30.0),
        gleg(1, VoiceSide::Long, 5.0),
    ];
    let risk = RiskConfig {
        level_weights: vec![0.5], // 只配 level0
        ..RiskConfig::default()
    };
    let stats = apply_level_cap(&ElementView::new(&els), &mut legs, 100.0, &risk);
    assert_eq!(legs[0].units, 30.0, "level0 未超帽原样");
    assert_eq!(legs[1].units, 0.0, "level1 无权重 ⟹ cap=0 ⟹ 归零");
    assert_eq!(stats.n_binding_levels, 1);
    assert_eq!(stats.n_levels, 2);
}

// ════════════════════════════════════════════════════════════════════════════
//  §重内单向（ADR 0014 裁定一，SPEC #847 S1 / #879）enforce_chong_unidirectional 单测
// ════════════════════════════════════════════════════════════════════════════

/// 持仓为多的重：反向（空）腿不建持仓、折成同向腿减仓；同向腿按 f=(L−R)/L 缩放。
#[test]
fn chong_uni_long_position_opposing_leg_becomes_reduction() {
    // L = 100 + 60 = 160（两级多腿），R = 40（一级空腿）；持仓多（chong_pos>0）。
    let mut legs = vec![
        gleg(0, VoiceSide::Long, 100.0),
        gleg(1, VoiceSide::Long, 60.0),
        gleg(2, VoiceSide::Short, 40.0),
    ];
    let st = enforce_chong_unidirectional(&mut legs, 10.0);
    assert_eq!(st.direction, 1);
    // f = (160−40)/160 = 0.75；同向腿等比缩、反向腿零化。
    assert!((st.factor - 0.75).abs() < 1e-12);
    assert!((legs[0].units - 75.0).abs() < 1e-9);
    assert!((legs[1].units - 45.0).abs() < 1e-9);
    assert_eq!(legs[2].units, 0.0, "反向腿零化：不建持仓、不产生声部");
    assert_eq!(st.n_opposing_zeroed, 1);
    assert_eq!(st.n_same_kept, 2);
    // 净目标 = L−R = 120——与旧净额逐位相同（减仓语义保短差的经济效果）。
    assert!((net_target_units(&legs) - 120.0).abs() < 1e-9);
    // 毛敞口 = 净敞口（单向 ⟹ 无隐藏对冲）。
    assert!((gross_target_units(&legs) - 120.0).abs() < 1e-9);
}

/// 不穿零：持仓多但 R > L ⟹ 目标钳到 0（先归零），不当 bar 翻空。
#[test]
fn chong_uni_never_crosses_zero() {
    let mut legs = vec![
        gleg(0, VoiceSide::Long, 50.0),
        gleg(1, VoiceSide::Short, 120.0),
    ];
    let st = enforce_chong_unidirectional(&mut legs, 3.0); // 仍持多
    assert_eq!(st.direction, 1);
    assert_eq!(st.factor, 0.0, "R>L ⟹ f 钳 0：全部减仓到 0，不穿零");
    assert_eq!(net_target_units(&legs), 0.0);
    // 旧口径此处净目标 = 50−120 = −70（直接穿零翻空）——新口径恒 ≥0。
    assert!(legs.iter().all(|l| l.units == 0.0));
}

/// 空仓的重：方向由当步合成多数侧定；少数侧零化。
#[test]
fn chong_uni_flat_majority_side_wins() {
    let mut legs = vec![
        gleg(0, VoiceSide::Long, 30.0),
        gleg(1, VoiceSide::Short, 90.0),
        gleg(2, VoiceSide::Short, 10.0),
    ];
    let st = enforce_chong_unidirectional(&mut legs, 0.0); // 空仓
    assert_eq!(st.direction, -1, "空仓 ⟹ 多数侧（空 100 > 多 30）定方向");
    // f = (100−30)/100 = 0.7；空腿缩放、多腿零化。
    assert!((st.factor - 0.7).abs() < 1e-12);
    assert_eq!(legs[0].units, 0.0);
    assert!((legs[1].units - 63.0).abs() < 1e-9);
    assert!((legs[2].units - 7.0).abs() < 1e-9);
    // 净目标 = −70 = 旧净额（30−100）——空仓时多数侧合成与旧净额同值。
    assert!((net_target_units(&legs) - (-70.0)).abs() < 1e-9);
}

/// 空仓且两侧合成严格相等 ⟹ 无方向，两侧全零化、本步不开仓（确定性裁决，不掷硬币）。
#[test]
fn chong_uni_flat_tie_no_position() {
    let mut legs = vec![
        gleg(0, VoiceSide::Long, 50.0),
        gleg(1, VoiceSide::Short, 50.0),
    ];
    let st = enforce_chong_unidirectional(&mut legs, 0.0);
    assert_eq!(st.direction, 0);
    assert_eq!(net_target_units(&legs), 0.0);
    assert!(legs.iter().all(|l| l.units == 0.0));
}

/// Flat 腿不受变换影响（净贡献本就为 0）；持仓空的重对称成立。
#[test]
fn chong_uni_flat_leg_untouched_and_short_position_symmetric() {
    let mut legs = vec![
        gleg(0, VoiceSide::Flat, 999.0),
        gleg(1, VoiceSide::Short, 80.0),
        gleg(2, VoiceSide::Long, 30.0),
    ];
    let st = enforce_chong_unidirectional(&mut legs, -5.0); // 持空
    assert_eq!(st.direction, -1);
    assert_eq!(legs[0].units, 999.0, "Flat 腿不动");
    // f = (80−30)/80 = 0.625
    assert!((legs[1].units - 50.0).abs() < 1e-9);
    assert_eq!(legs[2].units, 0.0);
    assert!((net_target_units(&legs) - (-50.0)).abs() < 1e-9);
}

/// 无反向腿时 f=1：逐位不变（同向加仓/延续不受本门影响）。
#[test]
fn chong_uni_same_direction_only_bit_exact() {
    let mut legs = vec![
        gleg(0, VoiceSide::Long, 100.0),
        gleg(1, VoiceSide::Long, 60.0),
    ];
    let st = enforce_chong_unidirectional(&mut legs, 1.0);
    assert_eq!(st.factor, 1.0);
    assert_eq!(st.n_opposing_zeroed, 0);
    assert!((legs[0].units - 100.0).abs() < 1e-12 && (legs[1].units - 60.0).abs() < 1e-12);
    assert!((net_target_units(&legs) - 160.0).abs() < 1e-12);
}

// ════════════════════════════════════════════════════════════════════════════
//  §对账测试锁（#1050 第一段，ADR 0024 裁定三·形态 B）
//
//  契约锚 Lean `Origin.LedgerReconciliation`（买卖点alpha2.pdf p5–6 §6/§7 逐式）：
//   - `ledger_identity_no_cost`：无成本时毛腿收益求和 ≡ 净额逐 bar 求和
//     Σ_v σ_v q_v (P_{ρ_v} − P_{λ_v}) = Σ_{t<T} N_t ΔP_t（telescoping）。
//   - `section7_two_leg_sum` / `hedged_two_leg_zero`：G_p+G_c = σ(Q−H)ΔP；H=Q ⟹ 0。
//   - `cost_reconciliation_with_net_cost`：成本按腿分配（hAlloc）后毛成本求和 ≡ 净成本求和。
//   - `ledger_identity_with_cost`：毛账净额（毛收益−毛成本）≡ 净账净额。
//
//  坐标口径（本票指定参考）：leg.rs `net_target_units`/`gross_target_units`
//  （毛腿→净持仓折叠坐标）与 risk.rs `LeverageCaps`/`VoiceNotional`（毛/净敞口美元空间，
//  N≤G 三角不等式）。本段只加测试锁：不改任何生产判定路径、不碰 OmsType::Netting。
// ════════════════════════════════════════════════════════════════════════════

/// 对账测试场景里的单腿（带自己入出场 bar 的毛腿；生产 LegTarget 只含当 bar 目标，
/// 跨 bar 身份在腿级账本里由 entry/exit 追踪——这里是 Lean `LedgerLeg` 的测试镜像）。
struct ReconcileLeg {
    side: VoiceSide,
    units: f64,
    entry: usize,
    exit: usize,
}

impl ReconcileLeg {
    /// σ_v：方向符号（Long→+1 / Short→−1 / Flat→0，对齐 Lean `LedgerLeg.signedQ`）。
    fn sign(&self) -> f64 {
        match self.side {
            VoiceSide::Long => 1.0,
            VoiceSide::Short => -1.0,
            VoiceSide::Flat => 0.0,
        }
    }

    /// 单腿毛收益 G_v = σ_v q_v (P_{ρ_v} − P_{λ_v})（Lean `LedgerLeg.entryPnl`）。
    fn entry_pnl(&self, prices: &[f64]) -> f64 {
        self.sign() * self.units * (prices[self.exit] - prices[self.entry])
    }

    /// 转生产坐标 `LegTarget`（`e_idx` 只是本 bar 树内偏移，`net_target_units`/`gross_target_units`
    /// 只读 side/units——role 取 Ambient 基线不参与折叠）。
    fn as_target(&self, e_idx: usize) -> LegTarget {
        LegTarget {
            e_idx,
            side: self.side,
            units: self.units,
            role: role(Horizontal::First, Vertical::Ambient, Dir::Plus),
        }
    }

    fn active_at(&self, bar: usize) -> bool {
        self.entry <= bar && bar < self.exit
    }
}

/// 三腿对账场景：父多头腿 [0,4)、子空头短差腿 [1,3)（H<Q）、晚入场多头腿 [2,4)。
/// 价格路径含涨跌（毛腿各自吃端点差，净额逐 bar 吃增量）。
fn reconciliation_scenario() -> (Vec<ReconcileLeg>, Vec<f64>) {
    (
        vec![
            ReconcileLeg {
                side: VoiceSide::Long,
                units: 2.0,
                entry: 0,
                exit: 4,
            },
            ReconcileLeg {
                side: VoiceSide::Short,
                units: 1.0,
                entry: 1,
                exit: 3,
            },
            ReconcileLeg {
                side: VoiceSide::Long,
                units: 0.5,
                entry: 2,
                exit: 4,
            },
        ],
        vec![100.0, 103.0, 101.5, 107.0, 110.0],
    )
}

/// ★对账锁①（Lean `ledger_identity_no_cost`）：毛腿收益求和 ≡ 净额逐 bar 求和。
/// 逐 bar 用 `net_target_units` 把活动腿折成净持仓（本票指定参考坐标）吃 ΔP_t；
/// 并逐 bar 断言 |net| ≤ gross（risk.rs `net_le_gross` 的 units 空间坐标）。
#[test]
fn ledger_reconciliation_gross_leg_pnl_equals_net_bar_pnl_no_cost() {
    let (legs, prices) = reconciliation_scenario();
    let bars = prices.len() - 1;

    let gross_pnl: f64 = legs.iter().map(|l| l.entry_pnl(&prices)).sum();

    let mut net_pnl = 0.0;
    for bar in 0..bars {
        let active: Vec<LegTarget> = legs
            .iter()
            .enumerate()
            .filter(|(_, l)| l.active_at(bar))
            .map(|(i, l)| l.as_target(i))
            .collect();
        let net = net_target_units(&active);
        let gross = gross_target_units(&active);
        assert!(
            net.abs() <= gross + 1e-12,
            "bar {bar}: |net|={} ≤ gross={} 三角不等式（坐标镜像 risk.rs net_le_gross）",
            net.abs(),
            gross
        );
        net_pnl += net * (prices[bar + 1] - prices[bar]);
    }

    let eps = 1e-9;
    assert!(
        (gross_pnl - net_pnl).abs() < eps,
        "毛腿收益求和 {gross_pnl} ≠ 净额逐 bar 求和 {net_pnl}（telescoping 恒等式被破坏）"
    );
}

/// ★对账锁②（Lean `section7_two_leg_sum`/`hedged_two_leg_zero`，PDF p6 §7 逐式）：
/// 单 bar 两腿：G_p+G_c = σ(Q−H)ΔP；H=Q ⟹ 0；H<Q ⟹ 收益来自剩余净敞口 σ(Q−H)。
#[test]
fn ledger_reconciliation_section7_two_leg_offset_identity() {
    let dp = 7.0;
    let parent = ReconcileLeg {
        side: VoiceSide::Long,
        units: 2.0,
        entry: 0,
        exit: 1,
    };
    let child_hedged = ReconcileLeg {
        side: VoiceSide::Short,
        units: 2.0,
        entry: 0,
        exit: 1,
    };
    let child_half = ReconcileLeg {
        side: VoiceSide::Short,
        units: 1.0,
        entry: 0,
        exit: 1,
    };

    // H=Q ⟹ G_p+G_c = σ(Q−Q)ΔP = 0，净额折叠同样为 0。
    let two: Vec<LegTarget> = (0..2)
        .map(|i| [&parent, &child_hedged][i].as_target(i))
        .collect();
    assert_eq!(net_target_units(&two), 0.0);
    let gross_hedged: f64 = [&parent, &child_hedged]
        .iter()
        .map(|l| l.entry_pnl(&[0.0, dp]))
        .sum();
    assert_eq!(gross_hedged, 0.0);

    // H=1<Q=2 ⟹ 剩余净敞口 N = σ(Q−H) = 1；G_p+G_c = 1·ΔP = N·ΔP。
    let two_half: Vec<LegTarget> = (0..2)
        .map(|i| [&parent, &child_half][i].as_target(i))
        .collect();
    let net_half = net_target_units(&two_half);
    assert_eq!(net_half, 1.0);
    let gross_half: f64 = [&parent, &child_half]
        .iter()
        .map(|l| l.entry_pnl(&[0.0, dp]))
        .sum();
    assert_eq!(gross_half, net_half * dp);
}

/// ★对账锁③（Lean `cost_reconciliation_with_net_cost`/`ledger_identity_with_cost`）：
/// 成本按腿分配（每 bar 净成本 C_t = Σ_l c_{l,t}，hAlloc）后毛成本求和 ≡ 净成本求和；
/// 且毛账净额（毛收益−毛成本）≡ 净账净额（逐 bar 净收益−逐 bar 净成本）。
#[test]
fn ledger_reconciliation_cost_allocation_gross_equals_net() {
    let (legs, prices) = reconciliation_scenario();
    let bars = prices.len() - 1;

    // 分配矩阵 c[l][t]：3 腿 × 4 bar（不活动 bar 的腿分配记为 0——成本只跟着活动腿走）。
    let c: Vec<Vec<f64>> = vec![
        vec![0.5, 0.2, 0.3, 0.1],   // 腿0 [0,4) 全期活动
        vec![0.0, 0.4, 0.2, 0.0],   // 腿1 [1,3)
        vec![0.0, 0.0, 0.25, 0.35], // 腿2 [2,4)
    ];
    for (l, row) in c.iter().enumerate() {
        for (t, &v) in row.iter().enumerate() {
            assert!(
                v == 0.0 || legs[l].active_at(t),
                "场景构造错误：非活动腿 {l} 在 bar {t} 被分配了成本 {v}"
            );
        }
    }
    let c_t: Vec<f64> = (0..bars)
        .map(|t| c.iter().map(|row| row[t]).sum::<f64>())
        .collect();

    let gross_cost: f64 = c.iter().map(|row| row.iter().sum::<f64>()).sum();
    let net_cost: f64 = c_t.iter().sum();
    let eps = 1e-12;
    assert!(
        (gross_cost - net_cost).abs() < eps,
        "毛成本求和 {gross_cost} ≠ 净成本求和 {net_cost}（hAlloc 下求和换序被破坏）"
    );

    // 全对账：毛收益−毛成本 ≡ 净收益−净成本（Lean ledger_identity_with_cost 的测试锁形态）。
    let gross_pnl: f64 = legs.iter().map(|l| l.entry_pnl(&prices)).sum();
    let mut net_pnl = 0.0;
    for bar in 0..bars {
        let active: Vec<LegTarget> = legs
            .iter()
            .enumerate()
            .filter(|(_, l)| l.active_at(bar))
            .map(|(i, l)| l.as_target(i))
            .collect();
        net_pnl += net_target_units(&active) * (prices[bar + 1] - prices[bar]);
    }
    assert!(
        ((gross_pnl - gross_cost) - (net_pnl - net_cost)).abs() < 1e-9,
        "毛账净额 {} ≠ 净账净额 {}",
        gross_pnl - gross_cost,
        net_pnl - net_cost
    );
}

/// ★对账锁④（本票指定参考坐标 risk.rs `LeverageCaps`）：
/// 美元空间毛/净敞口 + 毛/净杠杆帽：双开 long 300 + short 200 ⟹ N=100 ≤ G=500、
/// L^N ≤ L^G；毛/净帽须同时满足（净帽不能替代毛帽——Lean `net_ok_not_imply_gross_ok` 镜像）。
#[test]
fn ledger_reconciliation_leverage_caps_coordinates() {
    let voices = [
        VoiceNotional {
            side: VoiceSide::Long,
            notional_mag: 300,
        },
        VoiceNotional {
            side: VoiceSide::Short,
            notional_mag: 200,
        },
    ];
    assert_eq!(gross_notional(&voices), 500);
    assert_eq!(net_notional(&voices), 100);
    let m = leverage_metrics(&voices, 100.0);
    assert!(m.net <= m.gross, "N_t ≤ G_t 三角不等式");
    assert!(m.net_lev <= m.gross_lev, "L^N_t ≤ L^G_t（正权益除法保序）");

    let caps_ok = LeverageCaps {
        gross_cap: 6.0,
        net_cap: 2.0,
    };
    assert!(leverage_ok(m, caps_ok), "毛/净帽同时满足时应通过");

    // 净帽过宽、毛帽收紧：只查净会漏毛（G=500/E=5 > 4 违反毛帽）。
    let caps_tight_gross = LeverageCaps {
        gross_cap: 4.0,
        net_cap: 2.0,
    };
    assert!(
        !leverage_ok(m, caps_tight_gross),
        "毛帽违反时必须拒绝（净帽不替代毛帽）"
    );
}
