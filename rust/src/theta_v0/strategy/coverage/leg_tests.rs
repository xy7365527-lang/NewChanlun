use super::*;
use super::super::test_support::*;
use crate::theta_v0::types::Direction;
use super::super::super::interp::Buckets;

    /// 验收点 2：ReverseOpen 合法性断言——`w_dir(role.v==ReverseOpen) ≡ 1.0`（§7.5 s_g=s_α 定理 1）。
    /// 须在**所有三套预注册**下成立（CASE 1 优先于 CASE 3 查表，不被任何套缩放/归零）。
    #[test]
    fn w_dir_reverse_open_exempt_across_all_presets() {
        let presets = vec![
            ThetaDirPreset::Neutral,
            ThetaDirPreset::Follow { eta_adv: vec![0.5, 0.5, 0.5] },
            ThetaDirPreset::Adversary { eta_same: vec![0.5, 0.5, 0.5] },
        ];
        let mut cfg = VoiceConfig::default();
        for depth in 0..3u32 {
            for delta in [Dir::Plus, Dir::Minus] {
                // ReverseOpen 腿：18 类中所有 h（First/SameFollow/SameReverse）× ReverseOpen × ±δ。
                for h in [Horizontal::First, Horizontal::SameFollow, Horizontal::SameReverse] {
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
            for h in [Horizontal::First, Horizontal::SameFollow, Horizontal::SameReverse] {
                for v in [Vertical::Ambient, Vertical::FollowParent, Vertical::ReverseOpen] {
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
        assert_eq!(dir_weight(&sd, 0, &cfg), 1.0,
            "ReverseOpen Follow 仍豁免——η_adv 经 V 路径不可达（GPT §六）");
        cfg.theta_dir = ThetaDirPreset::Adversary { eta_same: vec![0.6] };
        assert_eq!(dir_weight(&sd, 0, &cfg), 1.0,
            "ReverseOpen Adversary 仍豁免——ReverseOpen CASE 1 整体豁免（η_same 对 FollowParent sign=+1 槽可达）");
    }

    /// w_grade（prereg-wg (c) 因子化）：消费 role.grade，G 轴 sizing 权重。
    #[test]
    fn w_grade_consumes_g_axis() {
        let mut cfg = VoiceConfig::default();
        let sl_same = OperationRole { h: Horizontal::First, v: Vertical::ReverseOpen, delta: Dir::Minus, grade: GradeRel::SameLevel };
        let sl_sub = OperationRole { h: Horizontal::First, v: Vertical::ReverseOpen, delta: Dir::Minus, grade: GradeRel::SubLevel };
        // default [1.0, 1.0] identity ⟹ SameLevel/SubLevel 都返 1.0（bit-exact）。
        assert_eq!(w_grade(&sl_same, &cfg), 1.0, "default w_grade[SameLevel]=1.0 identity");
        assert_eq!(w_grade(&sl_sub, &cfg), 1.0, "default w_grade[SubLevel]=1.0 identity");
        // 非 identity ⟹ SameLevel/SubLevel 返不同值（G 轴独立区分）。
        cfg.w_grade = [0.8, 0.4];
        assert_eq!(w_grade(&sl_same, &cfg), 0.8, "w_grade[SameLevel]=0.8");
        assert_eq!(w_grade(&sl_sub, &cfg), 0.4, "w_grade[SubLevel]=0.4");
        // C4 实质担忧：同级别反父 vs 次级别反父在 sizing 区分（两者 V 轴同 ReverseOpen）。
        assert_ne!(w_grade(&sl_same, &cfg), w_grade(&sl_sub, &cfg),
            "G 轴区分 SameLevel AgainstParent vs SubLevel ReverseOpen（V 轴不区分）");
        // 与 theta_dir preset 正交（G 轴独立消费，不绑 Follow/Adversary）。
        cfg.theta_dir = ThetaDirPreset::Follow { eta_adv: vec![0.5] };
        assert_eq!(w_grade(&sl_same, &cfg), 0.8, "w_grade 与 theta_dir preset 正交");
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
        assert!((root_leg.units - 600.0).abs() < 1e-9, "default w_grade identity ⟹ units=600（bit-exact）");
        assert_eq!(root_leg.role.grade, GradeRel::SameLevel, "root leg grade=SameLevel");
        // w_grade[SameLevel]=2.0 ⟹ root_leg units=600×2.0=1200（leg_target 真的乘了 w_grade）。
        let mut c2 = cfg();
        c2.w_grade = [2.0, 1.0];
        let root_leg2 = leg_target(&elements, 0, 1000.0, &c2);
        assert!((root_leg2.units - 1200.0).abs() < 1e-9, "w_grade[SameLevel]=2.0 ⟹ units=600×2.0=1200");
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
        cfg.theta_dir = ThetaDirPreset::Follow { eta_adv: vec![0.7, 0.7, 0.7] };
        assert_eq!(dir_weight(&fp_plus, 0, &cfg), 1.0, "Follow 顺上级保权");
        assert_eq!(dir_weight(&amb, 0, &cfg), 1.0, "根级 Ambient 豁免（CASE 2）");

        // Adversary 套：顺上级(sign=+1)=η_same=0.6，逆上级(sign=−1)=1.0。
        cfg.theta_dir = ThetaDirPreset::Adversary { eta_same: vec![0.6, 0.6, 0.6] };
        assert_eq!(dir_weight(&fp_plus, 0, &cfg), 0.6, "Adversary 顺上级降权 η_same");
        assert_eq!(dir_weight(&amb, 0, &cfg), 1.0, "根级 Ambient 豁免（CASE 2， adversary）");

        // FollowParent：δ=σ_higher ⟹ sign=+1（δ=Plus/Minus 均同向于各自 σ_higher）。
        // sign=−1 槽经 V 路径不可达 ⟹ 见 w_dir_sign_neg_slot_unreachable_via_v_axis。
        cfg.theta_dir = ThetaDirPreset::Adversary { eta_same: vec![0.6, 0.6, 0.6] };
        assert_eq!(dir_weight(&fp_minus, 0, &cfg), 0.6, "FollowParentδ=Minus 仍 sign=+1（σ_higher=−δ 同号）");

        // 越界 level 视 1.0（保权，不意外零化）。
        cfg.theta_dir = ThetaDirPreset::Adversary { eta_same: vec![0.6] }; // 只给 depth 0
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
        assert!((root_leg.units - 600.0).abs() < 1e-9, "根 depth 0 权重 0.60");
        // 顶层根角色：(First, Ambient, Plus)——去根化无 RootRole。
        assert_eq!(root_leg.role, role(Horizontal::First, Vertical::Ambient, Dir::Plus));
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
            LegTarget { e_idx: 0, side: VoiceSide::Long, units: 600.0, role: role(Horizontal::First, Vertical::Ambient, Dir::Plus) },
            LegTarget { e_idx: 1, side: VoiceSide::Long, units: 300.0, role: role(Horizontal::First, Vertical::FollowParent, Dir::Plus) },
            LegTarget { e_idx: 2, side: VoiceSide::Short, units: 300.0, role: role(Horizontal::SameReverse, Vertical::ReverseOpen, Dir::Minus) },
        ];
        let net = net_target_units(&legs);
        assert!((net - 600.0).abs() < 1e-9, "净 = 600+300−300（短差空腿对冲）");
        // 毛敞口 = |600|+|300|+|300| = 1200（双开毛敞口 > 净额 600，net_le_gross）。
        let gross = gross_target_units(&legs);
        assert!((gross - 1200.0).abs() < 1e-9, "毛敞口 1200 > 净 600（双开毛收益级覆盖）");
        assert!(net.abs() <= gross, "净 ≤ 毛（risk.rs net_le_gross）");
    }

    /// 精确对冲：单位数相同的多空腿 ⟹ 净额 0（毛敞口满额，net_le_gross 极端情形）。
    #[test]
    fn net_zero_when_hedged_equal() {
        let legs = vec![
            LegTarget { e_idx: 0, side: VoiceSide::Long, units: 400.0, role: role(Horizontal::First, Vertical::Ambient, Dir::Plus) },
            LegTarget { e_idx: 1, side: VoiceSide::Short, units: 400.0, role: role(Horizontal::SameReverse, Vertical::ReverseOpen, Dir::Minus) },
        ];
        assert!((net_target_units(&legs)).abs() < 1e-9, "等单位多空 ⟹ 净 0");
        assert!((gross_target_units(&legs) - 800.0).abs() < 1e-9, "毛 800（双开满额）");
    }

    /// ★P0-3 overlay_net_delta：ΔN_t = ReverseOpen 腿净贡献 = net(全腿)−net(剔 ReverseOpen)。
    #[test]
    fn overlay_net_delta_is_reverse_open_contribution() {
        // 根多 600 + 顺势子多 300 + 短差子空 300 ⟹ 全净=600，剔短差净=900 ⟹ ΔN=600−900=−300。
        let legs = vec![
            LegTarget { e_idx: 0, side: VoiceSide::Long, units: 600.0, role: role(Horizontal::First, Vertical::Ambient, Dir::Plus) },
            LegTarget { e_idx: 1, side: VoiceSide::Long, units: 300.0, role: role(Horizontal::First, Vertical::FollowParent, Dir::Plus) },
            LegTarget { e_idx: 2, side: VoiceSide::Short, units: 300.0, role: role(Horizontal::SameReverse, Vertical::ReverseOpen, Dir::Minus) },
        ];
        let dn = overlay_net_delta(&legs);
        assert!((dn + 300.0).abs() < 1e-9, "ΔN = 短差空腿净贡献 = −300");
        // 恒等式：ΔN = net(全腿) − net(剔 ReverseOpen)。
        let net_base: f64 = legs.iter().filter(|l| l.role.v != Vertical::ReverseOpen)
            .map(|l| if l.side == VoiceSide::Long { l.units } else { -l.units }).sum();
        assert!((net_target_units(&legs) - net_base - dn).abs() < 1e-9, "ΔN 恒等式");
        // 无 ReverseOpen ⟹ ΔN=0（overlay 不改净头寸，多空对冲.pdf b2 净额不可见）。
        assert!(overlay_net_delta(&legs[..2]).abs() < 1e-9, "无短差腿 ⟹ ΔN=0");
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
        let mut legs: Vec<LegTarget> =
            [0usize, 1, 2].iter().map(|&i| leg_target(&elements, i, 1000.0, &c)).collect();
        let risk = RiskConfig { enforce_gross_cap: true, ..RiskConfig::default() }; // γ=1 ⟹ Ḡ=1000
        let zeroed = apply_gross_cap(&ElementView::new(&elements), &mut legs, 1000.0, &risk);
        assert!(zeroed.is_empty(), "部分缩放无零化腿");
        // 单根 ⟹ c = Ḡ/G = 1000/1200 = 5/6，全腿等比例（比率约束族内 LexArgmin 主键投影）。
        assert!((legs[0].units - 500.0).abs() < 1e-9, "根 600×5/6");
        assert!((legs[1].units - 250.0).abs() < 1e-9, "子 300×5/6");
        assert!((legs[2].units - 250.0).abs() < 1e-9, "短差 300×5/6（同单位数比率保持）");
        assert!((gross_target_units(&legs) - 1000.0).abs() < 1e-9, "毛投影到边界 Ḡ");
        assert!((net_target_units(&legs) - 500.0).abs() < 1e-9, "净随比例缩 600×5/6");
    }

    /// ★#122 终裁反例坐实：等单位多空双开净 p̃=0（净检查恒过）、毛 G=1200 超限——腿层毛约束
    /// 正确捕获并压缩到 Ḡ，净保持 0（缩放不破坏对冲结构）。
    #[test]
    fn gross_cap_constrains_hedged_net_zero() {
        let els = vec![
            gce(0, 0, None, VoiceSide::Long),
            gce(0, 1, None, VoiceSide::Short),
        ];
        let mut legs = vec![gleg(0, VoiceSide::Long, 600.0), gleg(1, VoiceSide::Short, 600.0)];
        assert!((net_target_units(&legs)).abs() < 1e-9, "前提：净 0（净检查恒过）");
        assert!((gross_target_units(&legs) - 1200.0).abs() < 1e-9, "前提：毛 1200");
        let risk = RiskConfig { enforce_gross_cap: true, gamma: 0.5, ..RiskConfig::default() }; // Ḡ=500
        assert!(apply_gross_cap(&ElementView::new(&els), &mut legs, 1000.0, &risk).is_empty());
        // 两根等阈值 t=1200 ⟹ 同 c = 1−700/1200 = 5/12 ⟹ 各 250。
        assert!((legs[0].units - 250.0).abs() < 1e-9);
        assert!((legs[1].units - 250.0).abs() < 1e-9);
        assert!((gross_target_units(&legs) - 500.0).abs() < 1e-9, "毛压到 Ḡ=500");
        assert!((net_target_units(&legs)).abs() < 1e-9, "净仍 0（对冲结构保持）");
    }

    /// ★多根非均匀 water-filling（decide 5b46 方案B 核心）：子树内比率保持（同 c_r），跨根
    /// 按阈值 t_r=2A_r/G_r 非均匀分配（大子树 c 高）——非全腿同一比例。
    #[test]
    fn gross_cap_multi_root_waterfilling_nonuniform() {
        // 根0（id (1,0)）：父腿 600 L + 短差子腿 300 S（parent_id→根0）⟹ G_r0=900, A_r0=45e4, t0=1000。
        // 根1（id (1,1)）：单腿 300 L ⟹ G_r1=300, A_r1=9e4, t1=600。
        let root0 = ElementId { level: 1, ordinal: 0 };
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
        let risk = RiskConfig { enforce_gross_cap: true, gamma: 0.6, ..RiskConfig::default() }; // Ḡ=600
        assert!(apply_gross_cap(&ElementView::new(&els), &mut legs, 1000.0, &risk).is_empty());
        // μ = (1200−600)/(900/1000+300/600) = 600/1.4 = 3000/7；c0 = 1−μ/1000 = 4/7；c1 = 1−μ/600 = 2/7。
        assert!((legs[0].units - 600.0 * 4.0 / 7.0).abs() < 1e-9, "根0 父腿 ×4/7");
        assert!((legs[2].units - 300.0 * 4.0 / 7.0).abs() < 1e-9, "根0 子腿同 c（子树内比率保持）");
        assert!((legs[1].units - 300.0 * 2.0 / 7.0).abs() < 1e-9, "根1 ×2/7（非均匀：小子树缩更多）");
        assert!((gross_target_units(&legs) - 600.0).abs() < 1e-9, "毛 = Ḡ（约束取等）");
    }

    /// water-filling 零化分支：cap 足够小时低阈值根被整体归零（c_r=0），高阈值根承接余量。
    #[test]
    fn gross_cap_waterfilling_zeroes_low_threshold_root() {
        let els = vec![gce(1, 0, None, VoiceSide::Long), gce(1, 1, None, VoiceSide::Long)];
        let mut legs = vec![gleg(0, VoiceSide::Long, 600.0), gleg(1, VoiceSide::Long, 300.0)];
        let risk = RiskConfig { enforce_gross_cap: true, gamma: 0.2, ..RiskConfig::default() }; // Ḡ=200
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
        let els = vec![gce(1, 0, None, VoiceSide::Long), gce(1, 1, None, VoiceSide::Short)];
        // Ḡ=0：全零 + 全部上报零化。
        let mut legs = vec![gleg(0, VoiceSide::Long, 600.0), gleg(1, VoiceSide::Short, 300.0)];
        let risk0 = RiskConfig { enforce_gross_cap: true, gamma: 0.0, ..RiskConfig::default() };
        let zeroed = apply_gross_cap(&ElementView::new(&els), &mut legs, 1000.0, &risk0);
        assert!(legs.iter().all(|l| l.units == 0.0), "Ḡ=0 ⟹ 全腿归零");
        assert_eq!(zeroed, vec![0, 1], "Ḡ=0 ⟹ 全腿上报零化");
        // G ≤ Ḡ：逐位不动（bit-exact ==，非近似）。
        let orig = vec![gleg(0, VoiceSide::Long, 600.0), gleg(1, VoiceSide::Short, 300.0)];
        let mut legs2 = orig.clone();
        let risk2 = RiskConfig { enforce_gross_cap: true, gamma: 2.0, ..RiskConfig::default() }; // Ḡ=2000>900
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
        let (_, p_none) =
            coverage_step_from_buckets(view_split(&els, 0), &[], &buckets, 1000.0, &cfg(), None, &reg);
        assert!((p_none - 600.0).abs() < 1e-9);
        // enforce_gross_cap=false（default）：与 None 逐位相同（约束未配置=不激活）。
        let risk_off = RiskConfig::default();
        let (_, p_off) = coverage_step_from_buckets(
            view_split(&els, 0), &[], &buckets, 1000.0, &cfg(), Some(&risk_off), &reg,
        );
        assert_eq!(p_off, p_none, "default 不激活 ⟹ bit-exact");
        // enforce=true, γ=0.9 ⟹ Ḡ=900 < 1800：三等根 c=0.5 ⟹ 净 600×0.5=300。
        let risk_on = RiskConfig { enforce_gross_cap: true, gamma: 0.9, ..RiskConfig::default() };
        let (active_on, p_on) = coverage_step_from_buckets(
            view_split(&els, 0), &[], &buckets, 1000.0, &cfg(), Some(&risk_on), &reg,
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
            open: vec![cand(0, 1, VoiceSide::Long, 0), cand(1, 2, VoiceSide::Short, 1)],
            record: vec![],
        };
        let els = flat_elements(&buckets.open);
        // Ḡ=0（γ=0，enforce=true）：全部开仓腿零化 ⟹ p̃=0 且 next_active 空（无幽灵腿）。
        let risk0 = RiskConfig { enforce_gross_cap: true, gamma: 0.0, ..RiskConfig::default() };
        let (active, p) = coverage_step_from_buckets(
            view_split(&els, 0), &[], &buckets, 1000.0, &cfg(), Some(&risk0), &reg,
        );
        assert_eq!(p, 0.0, "Ḡ=0 ⟹ p̃=0");
        assert!(active.is_empty(), "零化开仓腿不入 next_active（从未建仓 ⟹ 无持仓身份延续）");
    }

    // ── §3 活动集递归原语（λ_e 区间，Lean M16 对齐——非生产入场，见 §3 GAP-5 note）已在上方测 ──

