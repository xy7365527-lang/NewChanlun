    use super::m8::*;
    use super::*;

    /// ★#419 RED：OKLO 不伪造 walk-forward；m8 只消费 PREREG_WINDOWS 已冻结的 §2.4 单段 OOS。
    /// BTC 未设路径仍保留原 p3fold + 前两个 OOS anchored 窗。**认识论 L0**（路由契约）。
    #[test]
    fn m8_symbol_routes_oklo_to_preregistered_single_oos() {
        assert_eq!(resolve_m8_symbol(None), "BTC");
        let btc = m8_windows("BTC");
        assert_eq!(btc.first().map(|w| w.0.as_str()), Some("p3fold"));

        assert_eq!(resolve_m8_symbol(Some("oklo")), "OKLO");
        let oklo_reg = PREREG_WINDOWS.iter().find(|w| w.symbol == "OKLO").unwrap();
        let oklo = m8_windows("OKLO");
        assert_eq!(
            oklo,
            vec![(
                "oklo_oos".to_string(),
                oklo_reg.oos.0.to_string(),
                oklo_reg.oos.1.to_string(),
            )]
        );
        assert!(oklo_reg.wf_anchored.is_empty(), "OKLO 明确无 walk-forward");
    }

    /// ★#419：未知 symbol fail-loud，禁止静默退回 BTC 后给报告打 OKLO 标签。
    #[test]
    #[should_panic(expected = "M8_SYMBOL")]
    fn m8_symbol_unknown_fails_loud() {
        let _ = resolve_m8_symbol(Some("NOPE"));
    }

    /// ★#484 / #481 MED-3：m8 报告只定义 BTC L2 与 OKLO L1 两种认识论；
    /// 即使品种已在通用数据/窗口表登记，未定义报告口径也必须 fail-loud。
    #[test]
    #[should_panic(expected = "仅支持 BTC/OKLO")]
    fn m8_registered_but_unsupported_symbol_fails_loud() {
        let _ = resolve_m8_symbol(Some("ES"));
    }

    /// ★#484 / #481 MED-3：heading 的认识论随受支持品种渲染，OKLO 禁挂 L2 实测。
    #[test]
    fn m8_epistemology_headings_match_symbol() {
        let (btc_intro, btc_layers) = m8_epistemology("BTC");
        assert!(btc_intro.contains("认识论 L2"));
        assert!(btc_layers.contains("L2 实测"));

        let (oklo_intro, oklo_layers) = m8_epistemology("OKLO");
        assert!(oklo_intro.contains("认识论 L1"));
        assert!(oklo_layers.contains("L1 实测"));
        assert!(!oklo_layers.contains("L2 实测"));
    }

    /// ★#490 MED-3 RED：报告 header 必须随实际 `voice_exec` 投影动态陈述执行域。
    #[test]
    fn m8_execution_projection_header_matches_voice_exec_state() {
        assert_eq!(
            m8_execution_projection_label(true),
            "声部执行投影 + 净额影子账本"
        );
        assert_eq!(
            m8_execution_projection_label(false),
            "净额执行 + overlay 旁路/账本"
        );
    }

    /// ★#492（#481 二轮复审 MED）RED：审计表 trades/fill 必须成对显式标注各自域，
    /// 禁止 `VOICE_EXEC=1` 时把净额影子账本 fill 写成未标域的旧称。
    #[test]
    fn m8_fee_audit_headings_match_voice_exec_state() {
        assert_eq!(
            m8_fee_audit_headings(true),
            ("声部投影trades", "净额影子fill")
        );
        assert_eq!(m8_fee_audit_headings(false), ("净额trades", "净额fill"));
    }

    /// ★#490 MED-2 RED：未标定 BTC 费用全落 unclassified_venue 时，审计行必须显式展示，
    /// 禁止形成“可见科目全 0、Σ总费非 0”的误读面。
    #[test]
    fn m8_fee_row_renders_unclassified_venue() {
        let fee = super::super::treasury::FeeAudit {
            n_fills: 1,
            notional: 4_510_009_151.37,
            unclassified_venue: 1_353_002.745411,
            total_fee: 1_353_002.745411,
            ..Default::default()
        };
        let row = format_m8_fee_audit_row("wf8", 7, fee);
        let cells = row.split('|').map(str::trim).collect::<Vec<_>>();
        assert_eq!(
            (m8_fee_audit_headings(true).1, cells[3]),
            ("净额影子fill", "1"),
            "VOICE_EXEC=1 时第 3 列须显式标作净额影子账本 fill"
        );
        assert_eq!(
            cells[10], "1353002.745411",
            "第 10 列须为 unclassified_venue"
        );
        assert_eq!(cells[12], "1353002.745411", "Σ总费须保持原值");
    }

    /// ★#484 复审：过滤零匹配或登记窗为空时不得谎称“已逐窗硬断言”。
    #[test]
    #[should_panic(expected = "没有完成任何可审计窗口")]
    fn m8_zero_audited_windows_fails_loud() {
        assert_m8_audit_coverage("BTC", Some("missing"), 0);
    }

    /// ★#419：产物路径默认兼容旧值；本票可显式隔离到 `/tmp/419_*`。
    #[test]
    fn m8_report_path_default_and_override() {
        assert_eq!(
            resolve_m8_report_path(None),
            "/tmp/m8_e2e_all_systems_oos.md"
        );
        assert_eq!(
            resolve_m8_report_path(Some("/tmp/419_m8_oklo_treasury.md")),
            "/tmp/419_m8_oklo_treasury.md"
        );
    }

    /// ★#419 RED：OKLO 层2/3结算必须按真实逐窗值渲染，禁止复用 BTC 静态“极负 /
    /// CostReduction”模板与数据行自相矛盾。**认识论 L0**（报告一致性契约）。
    #[test]
    fn m8_oklo_layer23_settlement_tracks_actual_positive_stage_ii() {
        use super::super::super::strategy::ledger::TStage;
        let (l2, l3) = m8_layer23_settlement(
            "OKLO",
            &[("oklo_oos".to_string(), 46_635.0, TStage::CapitalRecovered)],
        );
        assert!(l2.contains("+46635"), "层2须落真实 net_r：{l2}");
        assert!(l3.contains("II(已回本)"), "层3须落真实 Stage：{l3}");
        assert!(!l2.contains("极负"), "正值不得渲染成极负：{l2}");
        assert!(
            !l3.contains("CostReduction"),
            "Stage II 不得渲染成 Stage I：{l3}"
        );
    }

    /// ★#388 T2：**标量可得档**的层4 两格（LCB_OOS(R) / 三态）渲染与降级前**逐字符相同**——
    /// 臂R（`fee_schedule=None`）产物不因本次改造漂移一个字节。三分支全覆盖
    /// （M8:168：LCB>0 ⟹ CONFIRMED；R>0∧LCB≤0 ⟹ INCONCLUSIVE；R≤0 ⟹ 无）。
    /// **认识论 L0**（渲染契约，零数据依赖）。
    ///
    /// ★#423 第二阶段：`Some` 分支的适用档从「未标定档」扩到「标量可得档」（+ 按金额对称标定档），
    /// 渲染分支本身未改 ⟹ 本测断言逐字不变（臂R bit-exact 的承保点）。
    #[test]
    fn layer4_cells_uncalibrated_render_is_unchanged() {
        assert_eq!(layer4_cells(Some(1234.5), 9000.0), ("+1234".to_string(), "CONFIRMED".to_string()));
        assert_eq!(layer4_cells(Some(-2105181.0), 6851062.0), ("-2105181".to_string(), "INCONCLUSIVE".to_string()));
        assert_eq!(layer4_cells(Some(-3550861.0), -1191916.0), ("-3550861".to_string(), "无(R≤0)".to_string()));
    }

    /// ★#388 T2 / #385 Implementation Decisions（「臂 D/C 的相关读数须标注不可用或改述」）：
    /// **标量无良定义档**⟹ 随机对照系派生的两格一律标"不可用"，**不喂近似费率**、
    /// **不落一个看似有效的数**。**认识论 L0**（契约）。
    ///
    /// ★#423 第二阶段：本标注的适用范围从「标定档」收窄到「标量无良定义档」（按股档 /
    /// 按金额非对称档）——按金额对称标定档自第一阶段起标量可得、走 `Some` 分支给真数。
    /// 档位→分支的映射由 `layer4_notice_*` 四测按真实 datum / 手工构造档逐档钉住。
    #[test]
    fn layer4_cells_calibrated_marks_unavailable() {
        let (lcb, verdict) = layer4_cells(None, 6851062.0);
        assert_eq!(lcb, SCALAR_UNDEFINED_UNAVAILABLE);
        assert_eq!(verdict, SCALAR_UNDEFINED_UNAVAILABLE);
        assert!(SCALAR_UNDEFINED_UNAVAILABLE.contains("不可用"), "标注须自解释");
        assert!(
            SCALAR_UNDEFINED_UNAVAILABLE.contains("#374"),
            "标注须可追溯到有效域收窄票"
        );
        assert!(
            SCALAR_UNDEFINED_UNAVAILABLE.contains("#423"),
            "标注须可追溯到适用范围收窄票（分叉后不再覆盖标定档全体）"
        );
        assert!(
            !SCALAR_UNDEFINED_UNAVAILABLE.contains("标定档"),
            "标注正文不得把适用范围说成「标定档」——按金额对称标定档的读数照常产出"
        );
        // R 的正负不影响结论——三态判据本身依赖 LCB，缺 LCB 即无判据（不用 R 顶替）。
        assert_eq!(layer4_cells(None, -1.0), layer4_cells(None, 1.0));
    }

    /// ★#423 第二阶段：层4 声明段与层4 数值**同一个真值**（`scalar_cost_rate_opt` 的可得性），
    /// 不是两个判据。本组四测覆盖三分叉全部形态。
    ///
    /// **未标定档（臂R）⟹ 不出声明段**（`None`）——臂R 产物逐字节不变（bit-exact 中性）。
    /// **认识论 L0**（渲染契约，零数据依赖）。
    #[test]
    fn layer4_notice_absent_for_uncalibrated_arm() {
        let exec = ExecConfig::default();
        assert!(exec.fee_schedule.is_none(), "前置：default = 未标定档");
        assert_eq!(layer4_scalar_caliber_notice(&exec), None, "未标定档不出声明段");
    }

    /// **按金额对称档（本批臂D = Binance 现货 VIP0，maker=taker=10bp）⟹ 标量可得**：
    /// 声明段须说「照常产出」且**不含**不可用标注；层4 两格给真数、结算行走常规判据。
    ///
    /// **认识论 L1**（★#423 收尾轮 B 订正，原标 L2「读真实 datum 文件的档位形态」）。订正理由：
    /// 读真实 datum ≠ L2。L2 要求真实数据上的**假设检验**（可产生否定性结果）；本测断言的是
    /// 「档位形态 ⟹ 声明段/两格/结算行三处渲染一致」这条**渲染与分叉契约**，两格里的 LCB 数字
    /// 是写死的字面量（`-2105181.0`），不来自任何跑批 ⟹ 不可否证任何市场假设，信息增量为零。
    /// **本测能否证的假设：无**（同文件同型先例 `fee_datum_spec_resolves_repo_datum` 标 L1）。
    #[test]
    fn layer4_notice_declares_available_for_symmetric_notional() {
        let exec = ExecConfig {
            fee_schedule: Some(parse_fee_datum_spec(
                "venue_fee_binance_spot_20260726.json:BTC:VIP0",
            )),
            ..Default::default()
        };
        let scalar = super::super::treasury::scalar_cost_rate_opt(&exec);
        assert_eq!(
            scalar,
            Some(1e-3 + exec.slippage_bps / 10_000.0 + exec.tax_bps / 10_000.0),
            "对称按金额档：标量 = 档 10bp + 未标定滑点 + tax(=0)"
        );
        let notice = layer4_scalar_caliber_notice(&exec).expect("标定档须出声明段");
        assert!(
            !notice.contains(SCALAR_UNDEFINED_UNAVAILABLE),
            "标量可得档的声明段不得含不可用标注（那是声明与数值自相矛盾）：{notice}"
        );
        assert!(notice.contains("照常产出"), "须明说本臂层4 读数照常产出：{notice}");
        // 数值侧同一真值：层4 两格给真数，结算行走常规判据（与未标定臂同函数同口径）。
        assert_eq!(
            layer4_cells(scalar.map(|_| -2105181.0), 6851062.0),
            ("-2105181".to_string(), "INCONCLUSIVE".to_string())
        );
        assert_eq!(layer4_verdict_line(scalar), layer4_verdict_line(Some(3e-4)));
    }

    /// **按股档（IBKR Pro per-share）⟹ 标量无良定义**：声明段须含不可用标注 + 点名 per-share
    /// 理由；层4 两格标不可用、结算行给「无结论」。
    ///
    /// **认识论 L1**（★#423 收尾轮 B 订正，原标 L2「真实 IBKR datum」）。订正理由同上一测：
    /// 输入虽为真实 IBKR 费率表，被测命题却是「per-share 形态 ⟹ 三处渲染标不可用」的渲染契约，
    /// 不承载任何可被市场否证的假设 ⟹ L1。
    /// **本测能否证的假设：无**（同文件同型先例 `fee_datum_spec_resolves_repo_datum` 标 L1）。
    #[test]
    fn layer4_notice_declares_unavailable_for_per_share() {
        let exec = ExecConfig {
            fee_schedule: Some(parse_fee_datum_spec(
                "venue_fee_ibkr_pro_20260726.json:OKLO:PRO_TIERED_LE_300K_SHARES",
            )),
            ..Default::default()
        };
        let scalar = super::super::treasury::scalar_cost_rate_opt(&exec);
        assert_eq!(scalar, None, "按股档无常数等效费率");
        let notice = layer4_scalar_caliber_notice(&exec).expect("标定档须出声明段");
        assert!(notice.contains(SCALAR_UNDEFINED_UNAVAILABLE), "须标不可用：{notice}");
        assert!(notice.contains("per-share"), "须点名按股档理由：{notice}");
        assert_eq!(
            layer4_cells(scalar, 6851062.0),
            (SCALAR_UNDEFINED_UNAVAILABLE.to_string(), SCALAR_UNDEFINED_UNAVAILABLE.to_string())
        );
        assert!(layer4_verdict_line(scalar).contains("无结论"), "结算行须为无结论");
    }

    /// **按金额非对称档（maker≠taker）⟹ 落回无良定义**：角色维不消失 ⟹ 常数不由 datum 内容
    /// 唯一确定。声明段须标不可用。**认识论 L0**（手工构造档，覆盖 datum 内不存在的形态）。
    #[test]
    fn layer4_notice_declares_unavailable_for_asymmetric_notional() {
        use super::super::super::venue_fee::{FeeUnit, VenueFeeSchedule};
        let exec = ExecConfig {
            fee_schedule: Some(VenueFeeSchedule {
                venue: "SYNTH".into(),
                symbol: "BTC".into(),
                tier: "ASYM".into(),
                unit: FeeUnit::Notional { maker_bps: 5.0, taker_bps: 10.0 },
                datum_sha256: "0".repeat(64),
            }),
            ..Default::default()
        };
        let scalar = super::super::treasury::scalar_cost_rate_opt(&exec);
        assert_eq!(scalar, None, "非对称按金额档无良定义");
        let notice = layer4_scalar_caliber_notice(&exec).expect("标定档须出声明段");
        assert!(notice.contains(SCALAR_UNDEFINED_UNAVAILABLE), "须标不可用：{notice}");
        assert!(notice.contains("maker≠taker"), "须点名非对称理由：{notice}");
        assert!(layer4_verdict_line(scalar).contains("无结论"));
    }

    /// ★#423 收尾轮 F：**随机对照三值的落盘出口**——`Θ>随机` / `p_shift` / `p_indep` 三格。
    ///
    /// 覆盖三件事：(1) 标量无良定义档（`None`）三格标不可用，与层4 两格**同一个标注串**（单一来源）；
    /// (2) 标量可得档给真数，`theta_beats_random` 渲染「是/否」、p 值四位小数；
    /// (3) 对照退化时附退化标注——p=1.0 是真算出来的，但不得被读作「未能否证」。
    /// **认识论 L0**（渲染契约，零数据依赖：`Significance` 由字面量构造）。
    #[test]
    fn layer4_random_control_cells_render_contract() {
        use super::super::metrics::Significance;
        // (1) 无良定义档：三格均标不可用（不填 0、不留空）。
        let (b, ps, pi) = layer4_random_control_cells(None);
        assert_eq!(b, SCALAR_UNDEFINED_UNAVAILABLE);
        assert_eq!(ps, SCALAR_UNDEFINED_UNAVAILABLE);
        assert_eq!(pi, SCALAR_UNDEFINED_UNAVAILABLE);

        let base = Significance {
            boot_mean_total_pnl: 0.0,
            boot_pvalue_pnl_le_0: 0.5,
            boot_ci95_lo: -1.0,
            boot_ci95_hi: 1.0,
            sharpe: 0.0,
            sharpe_se: 0.0,
            sharpe_ci95_lo: 0.0,
            sharpe_ci95_hi: 0.0,
            shift_mean_return: 0.0,
            shift_pvalue: 0.6234,
            indep_mean_return: 0.0,
            indep_pvalue: 0.0421,
            theta_return_same_caliber: 0.0,
            theta_return_mtm: 0.0,
            theta_beats_random: false,
            controls_degenerate: false,
        };
        // (2) 标量可得 + 非退化：真数照出。
        let (b, ps, pi) = layer4_random_control_cells(Some(&base));
        assert_eq!((b.as_str(), ps.as_str(), pi.as_str()), ("否", "0.6234", "0.0421"));
        let win = Significance { theta_beats_random: true, ..base.clone() };
        assert_eq!(layer4_random_control_cells(Some(&win)).0, "是");

        // (3) 退化：三格各附退化标注（p 值仍照实渲染，不改数也不抹掉）。
        let deg = Significance {
            shift_pvalue: 1.0,
            indep_pvalue: 1.0,
            controls_degenerate: true,
            ..base.clone()
        };
        let (b, ps, pi) = layer4_random_control_cells(Some(&deg));
        assert_eq!((b.as_str(), ps.as_str(), pi.as_str()), ("否(对照退化)", "1.0000(退化)", "1.0000(退化)"));
    }

    /// ★#423 收尾轮 F：**声明段与新增三列自洽**——加列后声明段不得再说随机对照系「本表不列」。
    ///
    /// 为什么必须有这条：无良定义档的声明段原文写「`metrics::significance` 派生的随机对照系……
    /// **本表不列**，本档下一律不产」。收尾轮 F 给三值加了落盘出口 ⟹ 本表**列**了这三列（标不可用）。
    /// 若声明段不同步，同一份报告里声明与表格自相矛盾（090 声明膨胀的镜像：声明**萎缩**）。
    /// **认识论 L0**（措辞契约，零数据依赖）。
    #[test]
    fn layer4_notice_matches_random_control_columns() {
        let per_share = ExecConfig {
            fee_schedule: Some(parse_fee_datum_spec(
                "venue_fee_ibkr_pro_20260726.json:OKLO:PRO_TIERED_LE_300K_SHARES",
            )),
            ..Default::default()
        };
        let notice = layer4_scalar_caliber_notice(&per_share).expect("标定档须出声明段");
        assert!(
            !notice.contains("本表不列"),
            "加列后不得再声明「本表不列」——三列已在表内（标不可用）：{notice}"
        );
        assert!(
            notice.contains("Θ>随机") && notice.contains("p_shift") && notice.contains("p_indep"),
            "声明段须点名这三列的列名，读者才能机械核对表头：{notice}"
        );

        let symmetric = ExecConfig {
            fee_schedule: Some(parse_fee_datum_spec(
                "venue_fee_binance_spot_20260726.json:BTC:VIP0",
            )),
            ..Default::default()
        };
        let notice = layer4_scalar_caliber_notice(&symmetric).expect("标定档须出声明段");
        assert!(
            notice.contains("Θ>随机") && notice.contains("★#423 收尾轮 F"),
            "标量可得档的声明段须登记这三列是本轮**新增输出**（旧产物没有这些列，不是漂移）：{notice}"
        );
    }

    /// ★#388 T2：`M8_FEE_DATUM` spec 解析——仓内 datum 文件（路径即契约）逐字段落地，
    /// sha256 随 datum 内容而来（口径标签 `[L2费率标定: datum <前12位>]` 的来源）。
    /// **认识论 L1**（读真实 datum 文件的装载算术；不主张任何 alpha）。
    #[test]
    fn fee_datum_spec_resolves_repo_datum() {
        let s = parse_fee_datum_spec("venue_fee_binance_spot_20260726.json:BTC:VIP0");
        assert_eq!(s.venue, "BINANCE_SPOT");
        assert_eq!(s.symbol, "BTC");
        assert_eq!(s.tier, "VIP0");
        assert_eq!(s.datum_sha256.len(), 64);
        let o = parse_fee_datum_spec("venue_fee_ibkr_pro_20260726.json:OKLO:PRO_TIERED_LE_300K_SHARES");
        assert_eq!(o.venue, "IBKR_PRO_US_EQUITY");
        assert_eq!(o.symbol, "OKLO");
    }

    /// ★#389 T3：`M8_LEVEL_CAP` 未设 ⟹ **no-op**——帽字段逐位不变（臂R/臂D 的 bit-exact 中性
    /// 由此承保；回归门 `scripts/check_armR_trades_digest.py` 是它的端到端实证）。
    /// **认识论 L0**（配置契约，零数据依赖）。
    #[test]
    fn m8_level_cap_unset_is_noop() {
        let mut cfg = ThetaConfig::default();
        apply_m8_level_cap(&mut cfg, None);
        assert!(!cfg.risk.enforce_level_cap, "未设 env 不得开帽");
        assert!(cfg.risk.level_weights.is_empty(), "未设 env 不得注入权重（default 空表）");
    }

    /// ★#389 T3：`M8_LEVEL_CAP=true` ⟹ 开帽 + 注入 **#310 既有配置**（6 级各 0.05，Σ=0.3≤1）。
    /// 参数归属：`w_ℓ ∈ Θ_risk`（#310 票体逐字，禁冒充缠论可导）。**认识论 L0**。
    #[test]
    fn m8_level_cap_true_applies_310_weights() {
        use super::super::super::strategy::level_risk::{level_weights_sum, level_weights_sum_le_one};
        let mut cfg = ThetaConfig::default();
        apply_m8_level_cap(&mut cfg, Some("true"));
        assert!(cfg.risk.enforce_level_cap, "帽臂须开 enforce_level_cap");
        assert_eq!(cfg.risk.level_weights, vec![0.05; 6], "权重须为 #310 既有配置");
        assert!((level_weights_sum(&cfg.risk) - 0.3).abs() < 1e-12, "Σw_ℓ=0.3");
        assert!(level_weights_sum_le_one(&cfg.risk), "Σw_ℓ≤1 机器断言（#310 验收）");
        // 与 runner.rs 两处 #310/#351 在册验收配置同值（单一来源化的见证）。
        assert_eq!(M8_LEVEL_CAP_WEIGHTS_310.to_vec(), vec![0.05; 6]);
    }

    /// ★#389 T3：非法值 ⟹ **fail-loud**。静默退回帽关会让帽臂产物 = 臂D 读数却按帽臂登记
    /// （口径谎报，090 声明膨胀）。**认识论 L0**。
    #[test]
    #[should_panic(expected = "M8_LEVEL_CAP")]
    fn m8_level_cap_invalid_value_fails_loud() {
        let mut cfg = ThetaConfig::default();
        apply_m8_level_cap(&mut cfg, Some("1"));
    }

    /// ★#388 T2：spec 三段式格式非法 ⟹ fail-loud（禁静默按未标定档跑，那会让报告标签谎报）。
    #[test]
    #[should_panic(expected = "M8_FEE_DATUM")]
    fn fee_datum_spec_malformed_fails_loud() {
        let _ = parse_fee_datum_spec("venue_fee_binance_spot_20260726.json:BTC");
    }

    /// ★#388 T2：datum 文件不存在 ⟹ fail-loud（`load_datum` 的 Err 不被吞）。
    #[test]
    #[should_panic(expected = "M8_FEE_DATUM")]
    fn fee_datum_spec_missing_file_fails_loud() {
        let _ = parse_fee_datum_spec("venue_fee_does_not_exist.json:BTC:VIP0");
    }

    /// ★#388 T2：(symbol, tier) 未在册 ⟹ fail-loud（`VenueFeeBook::resolve` 的 Err 不被吞，
    /// 禁静默借别档）。
    #[test]
    #[should_panic(expected = "M8_FEE_DATUM")]
    fn fee_datum_spec_unknown_tier_fails_loud() {
        let _ = parse_fee_datum_spec("venue_fee_binance_spot_20260726.json:BTC:VIP99");
    }

    /// ★#388 T2：**品种借档拦截实证**（#360 `data::load_by_symbol` 的 fail-loud 承保本 env 钩子）——
    /// 把 OKLO 的 per-share 档配给 BTC 数据集 ⟹ `Err`，跑批在数据加载期就停，不会静默产出
    /// 「按股收费的 BTC」这种无意义费用。本测**读仓内 datum 文件**（经 `parse_fee_datum_spec`
    /// → `load_datum` 读 `venue_fee_ibkr_pro_20260726.json`），只是**不依赖市场数据文件**——
    /// 品种校验先于 `data_dir().join(file)` 读盘（`data.rs` 的 `load_by_symbol`：
    /// `if let Some(sched)` 校验块在 299 行，读盘在 308 行），故不加载 BTC 全量 bar。
    ///
    /// **语义重叠登记（#418 LOW-4）**：`data::tests::fee_schedule_symbol_must_match_dataset`
    /// （#360 已有，`data.rs:321`）覆盖同一条「品种不符 ⟹ Err」断言。本测保留的增量在于覆盖
    /// **经 env spec 解析出档位**（`parse_fee_datum_spec`）这条路径——#360 那条直接构造
    /// schedule，不穿过 env 钩子。两测断言相同、入口不同，故并存不算冗余。
    #[test]
    fn fee_datum_cross_symbol_borrow_is_blocked() {
        let mut cfg = ThetaConfig::default();
        cfg.exec.fee_schedule =
            Some(parse_fee_datum_spec("venue_fee_ibkr_pro_20260726.json:OKLO:PRO_TIERED_LE_300K_SHARES"));
        let err = data::load_by_symbol("BTC", &cfg).expect_err("跨品种借档须被拦截");
        assert!(err.contains("品种不符"), "错误须点明品种不符，实得：{err}");
    }

    /// δ-free 精确重算自检（Task #186，L1 管线正确性）：round-trip dump→load bit-exact + δ-free
    /// perm_p 池化正确性。合成 records（level0/bsp3/σ+1 两 δ 同号高残差 = beta 漂移签名）：δ-free
    /// 池化后仍显著（同号不抵消，perm_p 小）；对照买卖异号（真方向 alpha）池化抵消 ⟹ perm_p 不显著。
    #[test]
    fn deltafree_dump_roundtrip_and_pooling() {
        use super::super::mu_estimator::PositionState;
        use crate::theta_v0::types::BspBits;
        // ① round-trip：dump 写盘→load 还原逐字节相等（f64 bit 模式精确）。
        let mk = |delta: i8, resid: f64, tb: u32, et: ExitType| {
            let bits = if delta > 0 { BspBits { buy3: true, ..Default::default() } } else { BspBits { sell3: true, ..Default::default() } };
            let class = MuClass::from_certificate(0, delta, bits, 1, PositionState::Root);
            ResidualTrade { class, resid_base: resid, cost: 0.1, h_bucket: 0, time_block: tb, d: 1.0, exit_type: et }
        };
        // exit_type 循环覆盖 5 变体 ⟹ round-trip 实际穿过新诊断列（否则该列是死代码）。
        let ets = [ExitType::CloseRoot, ExitType::ReduceCore, ExitType::CloseShortDiff, ExitType::RiskExit, ExitType::Hold];
        let recs: Vec<ResidualTrade> = (0..40)
            .map(|i| mk(if i % 2 == 0 { 1 } else { -1 }, 3.14159_f64 * (i as f64 + 1.0), (i % 2) as u32, ets[i % 5]))
            .collect();
        let path = std::env::temp_dir().join("deltafree_roundtrip_test.tsv");
        let p = path.to_str().unwrap();
        // 直接调 dump 逻辑：门控经 env，故临时置位。
        std::env::set_var("DELTAFREE_DUMP", p);
        dump_deltafree_pertrade(&recs);
        std::env::remove_var("DELTAFREE_DUMP");
        let loaded = load_deltafree_dump(p);
        assert_eq!(loaded.len(), recs.len(), "round-trip 笔数");
        for (a, b) in recs.iter().zip(&loaded) {
            assert_eq!(a.resid_base.to_bits(), b.resid_base.to_bits(), "resid_base bit-exact");
            assert_eq!(a.cost.to_bits(), b.cost.to_bits(), "cost bit-exact");
            assert_eq!(a.class.delta, b.class.delta);
            assert_eq!(a.class.bsp_class(), b.class.bsp_class());
            assert_eq!(a.class.parent_dir, b.class.parent_dir);
            assert_eq!((a.h_bucket, a.time_block), (b.h_bucket, b.time_block));
            assert_eq!(a.exit_type, b.exit_type, "exit_type 诊断列 round-trip 无损");
        }

        // ② δ-free 池化：两 δ 同号高残差（beta 签名）。买 r=+10、卖 r=+10 ⟹ Y_buy=+10,Y_sell=−10
        //    池化 mean=0；但 §3.1 的 beta 判据是「δ-free 基 mean 由同号 resid 不抵消」——用 resid_base
        //    同号构造：买卖 resid 都=+10 ⟹ 池化残差基不随 δ 置换改变符号结构。这里验 perm_p 机制运行
        //    （同批置换、只读出侧池化）：H0 独立 δ ⟹ 池化 perm_p 不显著（>0.05）。
        let mut h0: Vec<ResidualTrade> = Vec::new();
        for i in 0..60 {
            h0.push(mk(if i % 2 == 0 { 1 } else { -1 }, (i % 5) as f64 - 2.0, 0, ExitType::Hold));
        }
        let pp = perm_test::stratified_delta_perm_p_deltafree(&h0, perm_test::N_PERM, perm_test::PERM_SEED);
        // A1：δ-free 基键含 force_state 第 8 维——mk 走 from_certificate ⟹ force_state=None。
        assert!(pp.contains_key(&(0, 3, 1, None)), "δ-free 基键 (0,3,+1,None) 应存在");
        assert!(pp[&(0, 3, 1, None)] > 0.05, "H0 独立 δ ⟹ δ-free 池化 perm_p 不显著: {}", pp[&(0, 3, 1, None)]);
    }

    /// L1 自检（预注册 §3 承重不变量）：单品种 time_block 值域必须 < SYMBOL_STRIDE，否则跨品种 stratum
    /// 碰撞 ⟹ δ 在跨品种间 shuffle ⟹ beta 污染。最大窗数（ES/CL/GC=15）·WF_TIME_STRIDE + 窗内块上界
    /// 必须落在一个 SYMBOL_STRIDE 内；7 品种偏移不溢出 u32。破了这条 = 跨品种隔离失效。
    #[test]
    fn symbol_stride_isolates_strata() {
        let max_windows = PREREG_WINDOWS.iter().map(|w| w.wf_anchored.len()).max().unwrap() as u32;
        // 窗内 time_block = entry_bar/43_200；6 月窗 1min ≤ ~260K bar ⟹ 窗内块上界 ~10 < WF_TIME_STRIDE。
        let per_symbol_span = max_windows * WF_TIME_STRIDE + WF_TIME_STRIDE;
        assert!(per_symbol_span < SYMBOL_STRIDE, "单品种 time_block 值域 {per_symbol_span} 须 < SYMBOL_STRIDE {SYMBOL_STRIDE}（跨品种 stratum 隔离）");
        let max_offset = (L3_UNIVERSE.len() as u32 - 1) * SYMBOL_STRIDE + per_symbol_span;
        assert!(max_offset < u32::MAX, "7 品种最大 time_block 偏移不溢出 u32");
        assert!(!L3_UNIVERSE.contains(&"OKLO"), "OKLO 剔除（Observation 池，无 walk-forward OOS）");
    }

    /// L1 自检（q4 harness 日期算术，零信息增量）：前推 6 月跨年/钳日 + 前一天跨月。
    #[test]
    fn q4_date_helpers_hand_calc() {
        assert_eq!(q4_shift_back_6m("2023-02-17"), "2022-08-17"); // 跨年
        assert_eq!(q4_shift_back_6m("2023-08-17"), "2023-02-17");
        assert_eq!(q4_shift_back_6m("2023-08-31"), "2023-02-28"); // 钳日
        assert_eq!(q4_prev_day("2023-02-17"), "2023-02-16");
        assert_eq!(q4_prev_day("2023-03-01"), "2023-02-28"); // 跨月钳位
        assert_eq!(q4_prev_day("2023-01-01"), "2022-12-28"); // 跨年钳位
    }

    /// L1 自检（prereg-l3norm §5，管线正确性零信息增量）：σ̂ 计算核心 = 逐 bar 差分样本 std。
    /// 手算对照：pxs=[1,2,4,7] ⟹ diffs=[1,2,3] mean=2 var=((1)+0+(1))/2=1 std=1。防 σ̂ 估计 bug。
    #[test]
    fn sigma_stdev_matches_hand_calc() {
        assert!((stdev_consecutive_diffs(&[1.0, 2.0, 4.0, 7.0]) - 1.0).abs() < 1e-12);
        assert_eq!(stdev_consecutive_diffs(&[5.0]), 0.0, "单点无差分 ⟹ 0（守卫）");
        assert_eq!(stdev_consecutive_diffs(&[]), 0.0, "空序列 ⟹ 0（守卫）");
    }

    /// on2-est2 profile（Task #187）：拆解 R5 est×2 段的耗时归属。q4_fullpi_policy 每窗对同一
    /// train.bars 调 `build_mu_from_bars` **两次**（fullpi/plain）——两次 config 仅 risk/margin 不同，
    /// 而逐 bar 分类（parser+tower）只依赖缠论字段（与 risk/margin 无关）⟹ 两次分类逐 bar 完全相同。
    /// 本 profile 测三段墙钟：①单次全 bar 分类 pass（IncrementalClassifier）②单次 build_mu 全程
    /// ③连跑两次 build_mu（est×2 现状）——定位分类占比与「共享分类可省」的上界。
    /// `cargo test --release --lib theta_v0::backtest::wverify_run::tests::profile_est2_shared_classify -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn profile_est2_shared_classify() {
        use super::super::incremental::IncrementalClassifier;
        use std::time::Instant;
        let cfg = ThetaConfig::default();
        let ds = data::load_by_symbol("BTC", &cfg).expect("BTC 数据加载");
        let train = ds.slice_date_window("2022-07-01", "2022-12-31"); // p3fold train（最短段）
        let n = train.bars.len();
        eprintln!("[est2-profile] BTC p3fold train n={n} bar");

        // ① 单次全 bar 分类 pass（黑洞化 tower_generation 防 DCE）。
        let t0 = Instant::now();
        let mut clf = IncrementalClassifier::new(&train.bars, &cfg);
        let mut sink = 0u64;
        for i in 0..n {
            let (_cls, _tower) = clf.classify_at(i);
            sink = sink.wrapping_add(clf.tower_generation()).wrapping_add(clf.forest_epoch());
        }
        let classify_ms = t0.elapsed().as_secs_f64() * 1e3;
        eprintln!("[est2-profile] ① 单次全 bar 分类 pass = {classify_ms:.1} ms (sink={sink})");

        // ② 单次 build_mu 全程（含分类 pass + fill loop）。
        let mut chi = ThetaConfig::default();
        chi.risk.chi_theta = Some(0.0);
        chi.risk.chi_z_alpha = 1.645;
        chi.risk.enforce_gross_cap = true;
        super::super::super::classifier::stage_profile::reset();
        let t1 = Instant::now();
        let (est_a, _) = build_mu_from_bars(&train.bars, &chi, 0);
        let build_one_ms = t1.elapsed().as_secs_f64() * 1e3;
        eprintln!("[est2-profile] ② 单次 build_mu(fullpi) = {build_one_ms:.1} ms (n_classes={})", est_a.n_classes());
        super::super::super::classifier::stage_profile::dump();

        // ③ est×2 现状：连跑两次（fullpi + plain）。
        let t2 = Instant::now();
        let (_e1, _) = build_mu_from_bars(&train.bars, &chi, 0);
        let (_e2, _) = build_mu_from_bars(&train.bars, &cfg, 0);
        let build_two_ms = t2.elapsed().as_secs_f64() * 1e3;
        eprintln!("[est2-profile] ③ est×2 连跑两次 build_mu = {build_two_ms:.1} ms");

        let fill_ms = (build_one_ms - classify_ms).max(0.0);
        eprintln!(
            "[est2-profile] 归属：分类={classify_ms:.1}ms ({:.0}%) fill={fill_ms:.1}ms | est×2 中分类冗余={classify_ms:.1}ms 共享后上界省≈{:.0}%",
            classify_ms / build_one_ms * 100.0,
            classify_ms / build_two_ms * 100.0,
        );

        // ④ 双曲线 scaling exp（n/2 vs n，分别测分类与 fill loop）——定 O(n²) 靶归属。
        let half = &train.bars[..n / 2];
        let t3 = Instant::now();
        let mut clf_h = IncrementalClassifier::new(half, &cfg);
        let mut sink_h = 0u64;
        for i in 0..half.len() {
            let _ = clf_h.classify_at(i);
            sink_h = sink_h.wrapping_add(clf_h.forest_epoch());
        }
        let classify_half_ms = t3.elapsed().as_secs_f64() * 1e3;
        let t4 = Instant::now();
        let (_eh, _) = build_mu_from_bars(half, &chi, 0);
        let build_half_ms = t4.elapsed().as_secs_f64() * 1e3;
        let ratio = (n as f64 / (n / 2) as f64).log2();
        let exp_build = (build_one_ms / build_half_ms).log2() / ratio;
        let exp_classify = (classify_ms / classify_half_ms).log2() / ratio;
        let fill_full = (build_one_ms - classify_ms).max(1e-9);
        let fill_half = (build_half_ms - classify_half_ms).max(1e-9);
        let exp_fill = (fill_full / fill_half).log2() / ratio;
        eprintln!(
            "[est2-profile] ④ 双曲线 (sink_h={sink_h}): build_mu exp={exp_build:.3} | 分类 exp={exp_classify:.3}（{classify_half_ms:.0}→{classify_ms:.0}ms）| fill loop exp={exp_fill:.3}（{fill_half:.0}→{fill_full:.0}ms）",
        );
    }

    /// δ-free 主裁决键守卫（语义级，a3；编译期形状守卫见 `_DELTAFREE_KEY_SHAPE_FROZEN`）：
    /// 断言 [`deltafree_verdict`] 分桶键为 (level, bsp_class, parent_dir, force_state) 四元组
    /// 且**不含 δ 分量**，防止未来把 δ 加回主裁决键。
    /// ① 仅 δ 不同（其余四分量全同）的记录必须池化进同一主裁决桶——若键混入 δ 会裂成 2 桶 ⟹ FAIL；
    /// ② 四键分量各自变异必须各裂新桶（含 force_state Some(态) 变化与 Some→None 的诚实缺维区分）。
    /// 注：[`bucket_verdict`] 是含 δ 的描述性报告桶，非主裁决，不在本守卫范围。
    #[test]
    fn deltafree_verdict_key_is_delta_free_four_tuple() {
        use super::super::mu_estimator::PositionState;
        use crate::theta_v0::types::BspBits;

        // 构造：给定 (level, bsp主类, δ, parent_dir, force_state) 的残差记录。bits 按 δ 选买/卖侧
        // （同 perm_test::tests::rt 模式）——bsp_class() 对买卖同类归并，键不受 δ 侧影响。
        let rt = |level: u32, bsp: u8, delta: i8, parent_dir: i8, fs: Option<ForceStateA5>, resid: f64| {
            let bits = match (bsp, delta > 0) {
                (1, true) => BspBits { buy1: true, ..Default::default() },
                (1, false) => BspBits { sell1: true, ..Default::default() },
                (2, true) => BspBits { buy2: true, ..Default::default() },
                (2, false) => BspBits { sell2: true, ..Default::default() },
                (_, true) => BspBits { buy3: true, ..Default::default() },
                (_, false) => BspBits { sell3: true, ..Default::default() },
            };
            let mut class = MuClass::from_certificate(level, delta, bits, parent_dir, PositionState::Root);
            class.force_state = fs; // from_certificate 诚实 None，测试显式注入第 8 维
            ResidualTrade { class, resid_base: resid, cost: 0.0, h_bucket: 0, time_block: 0, d: 1.0, exit_type: ExitType::Hold }
        };
        // 桶数 = V+F+I 三态计数总和（deltafree_verdict 每桶恰产一个 AlphaState）。
        let n_buckets = |records: &[ResidualTrade]| {
            let (_, _, (nv, nf, ni), _) = deltafree_verdict(records);
            nv + nf + ni
        };

        // ① δ 池化：仅 δ 不同、(level=1,bsp=1,σ_p=+1,force=Dominated) 全同 → 恰 1 桶。
        let base_fs = Some(ForceStateA5::Dominated);
        let pooled: Vec<ResidualTrade> = (0..8)
            .map(|i| rt(1, 1, if i % 2 == 0 { 1 } else { -1 }, 1, base_fs, i as f64 * 0.7 - 2.0))
            .collect();
        assert_eq!(
            n_buckets(&pooled), 1,
            "仅 δ 不同的记录必须池化进同一 δ-free 主裁决桶（=1）；>1 ⟹ 主裁决键混入了 δ 分量"
        );

        // ② 四键分量各自分桶：基桶 + 5 种单分量变异（每变异桶买卖两 δ 侧各 2 笔，保 A1 ⊥δ 交换自由度）。
        let mut recs = pooled.clone();
        for i in 0..4 {
            let d = if i % 2 == 0 { 1 } else { -1 };
            let r = i as f64 * 0.7 - 2.0;
            recs.push(rt(2, 1, d, 1, base_fs, r)); // level 变异
            recs.push(rt(1, 2, d, 1, base_fs, r)); // bsp_class 变异
            recs.push(rt(1, 1, d, -1, base_fs, r)); // parent_dir 变异
            recs.push(rt(1, 1, d, 1, Some(ForceStateA5::Dominates), r)); // force_state 态变异
            recs.push(rt(1, 1, d, 1, None, r)); // force_state Some→None（诚实缺维须独立成桶）
        }
        assert_eq!(
            n_buckets(&recs), 6,
            "四键分量 level/bsp_class/parent_dir/force_state（含 None）各自变异须各裂新桶：1 基桶 + 5 变异桶"
        );
    }
