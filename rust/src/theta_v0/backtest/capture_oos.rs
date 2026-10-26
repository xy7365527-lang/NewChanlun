//! S3 捕获率回测（task #11）——充分性检验，R3/S2 明确未证部分。
//!
//! **档处置（验收统计推断核·档2-修砍单，`chanlun/escalate/chi-line-falsification-ruling-20260728.md`
//! §1②，2026-07-28）**：捕获率回测作为新协议推断仪器已砍。诊断件保留（禁删）；历史裁决照旧有效。
//! 登记详见 `chanlun/review-results/prob-inference-disposition-registry-20260728.md`。
//!
//! ## 预声明口径（跑批前冻结，treasury-execution-plan-20260707.md S3）
//!
//! - **切分协议**：复用 `prereg_windows::PREREG_WINDOWS` BTC anchored walk-forward 窗
//!   （与 wverify_run.rs 同协议）。训练窗定 λ_gap，测试窗只读。clipped 窗报告但不入主判据（W1）。
//! - **λ_gap 定参规则（唯一，无搜索）**：λ_w = clamp(训练窗段 gap 分布 p95, [0.05%, 1.0%])。
//!   分位法承接 S2 敏感性曲线；域端点 = S2 报告建议域。p95 规则的构造含义：训练窗内停出率 ≈5%。
//! - **因果入场**：增量解析 `ParseLayerIncr::append`，段 j 确认 ⟺ `segments_confirmed_len`
//!   高水位越过 j+1。段 j 确认于 bar t ⟹ 当前段 = j+1，方向 = 段 j 反向，入场价 = close[t]。
//!   出场 = 段 j+1 确认 bar 的 close，或 λ 止损先触发（intrabar，按止损价成交，缺口风险未计——
//!   诚实声明 §4）。止损后空仓至下一段确认，不复入。入场 bar 当根不检查止损（入场在收盘）。
//! - **指标**：捕获率 = dir·(exit−entry)/(H−L)（分母 = 终局段理论价差）；净 g_n =
//!   dir·(exit−entry)/entry − fee·(entry+exit)/entry，fee=3e-4（config.rs:228-230，与 R3/S1 同）。
//! - **η⋆ 范围声明**：R1 闭式 η⋆=λ_gap·ρ·W·Σw+κQ 中 ρ/κ 未冻结——本报告只检验 λ_gap 分量
//!   （OOS 停出率 vs 训练窗预测 5%），完整 η⋆ 阈值裁决留 S4 前冻结 ρ/κ。
//!
//! ## 认识论等级
//!
//! **L3 候选**（真实数据 + 因果入场 + 预注册窗口 OOS）；但见诚实声明——
//! 训练窗 λ 用终局批量段（端点确认含窗尾少量后视，量级 = 确认滞后），主判据不受影响。
//!
//! ## 诚实声明（跑前写死）
//!
//! 1. 止损取劣侧成交：fill = 止损价与当根 open 的劣侧（多头 min / 空头 max），
//!    覆盖跳空穿越——停出损失是下界口径。（bughunt F-10 修复口径，2026-07-10）
//! 2. confirmed 前缀可回退（#88 复活级联）：已开仓交易照因果保留，回退次数如实报告。
//! 3. 测试窗外交易（首个 test 窗前/末窗后）只进逐年描述表，无 λ 止损，不入主判据。
//! 4. untradable bar 上**不成交**（bughunt F-10 修复口径）：入场跳过、段确认出场与
//!    止损顺延至下一可交易 bar（止损按劣侧），全部计数如实报告。
//! 5. 窗界平仓（bughunt F-09 修复口径）：持仓所属 test 窗结束后的首根 bar 上按最近
//!    可交易 close 强制平仓，窗 k 统计不吸收窗 k+1 时段路径；计数如实报告。
//! 6. 训练窗 λ 标定的段特征路径截断至 train_end 当日末根 bar（bughunt F-08 修复口径），
//!    杜绝特征读取越界；段身份仍来自终局解析（见上，量级 = 确认滞后，如实声明）。
//! 7. 段解析无效（#1585 / X-39，承接 #1581）：增量层带段端门 fail-closed 标记的 bar 上拒读段账本——
//!    不按段确认开平仓、高水位不动（价格止损/窗界强平/顺延出场照常，不读段结构）；持仓期间
//!    遇标记的交易记为受污染，剔出全部聚合，只计数并报出标记 bar 区间。终局批量解析带标记时，
//!    λ 定参与理论分母只读失败段起点前的可读前缀。本回测只产内存报告，不写任何生产账。

#[cfg(test)]
mod tests {
    use super::super::super::config::ThetaConfig;
    use super::super::super::parser::{parse_layer, ParseLayer, ParseLayerIncr};
    use super::super::super::types::{Bar, Direction, Segment};
    use super::super::data;
    use super::super::prereg_windows::{WfWindow, OOS_START, PREREG_WINDOWS};
    use super::super::segment_gn::tests::parse_invalid_note;

    const FEE: f64 = 3e-4; // 单边，config.rs:228-230 冻结口径
    const LAMBDA_LO: f64 = 5e-4; // S2 建议域下沿 0.05%
    const LAMBDA_HI: f64 = 1e-2; // S2 建议域上沿 1.0%
    const LAMBDA_Q: f64 = 0.95; // 分位法定参（预声明，无搜索）

    fn quantile(sorted: &[f64], q: f64) -> f64 {
        assert!(!sorted.is_empty());
        sorted[((sorted.len() - 1) as f64 * q).round() as usize]
    }

    fn opposite(d: Direction) -> Direction {
        match d {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
        }
    }

    /// F-10：untradable bar 上被阻塞的出场，顺延至下一可交易 bar 成交。
    enum Pending {
        /// 段确认出场被阻塞——下一可交易 bar 按 close 成交。
        Seg,
        /// 止损触发被阻塞——下一可交易 bar 按 (止损价, open) 劣侧成交。
        Stop(f64),
    }

    struct OpenTrade {
        seg_idx: usize,
        entry_bar: usize,
        entry: f64,
        dir: Direction,
        lambda: Option<f64>,
        win: Option<(u32, bool)>, // (窗序号, clipped)
        pending: Option<Pending>, // F-10：被 untradable bar 阻塞的出场
        parse_invalid: bool,      // #1585：持仓期间遇段解析无效标记
    }

    struct TradeRec {
        seg_idx: usize,
        entry_bar: usize,
        entry: f64,
        exit: f64,
        dir: Direction,
        stopped: bool,
        win: Option<(u32, bool)>,
        parse_invalid: bool, // #1585：受段解析无效污染 ⟹ 不入任何聚合
    }

    impl TradeRec {
        fn dirf(&self) -> f64 {
            match self.dir {
                Direction::Up => 1.0,
                Direction::Down => -1.0,
            }
        }
        /// 净 g_n（收益率分数）：方向收益 − 双腿费。
        fn gn(&self) -> f64 {
            self.dirf() * (self.exit - self.entry) / self.entry
                - FEE * (self.entry + self.exit) / self.entry
        }
        /// 已兑现价差（价格单位，可为负）。
        fn realized(&self) -> f64 {
            self.dirf() * (self.exit - self.entry)
        }
    }

    struct Agg {
        n: usize,
        stops: usize,
        gns: Vec<f64>,
        realized_sum: f64,
        theory_sum: f64,
        captures: Vec<f64>,
    }

    impl Agg {
        fn new() -> Self {
            Agg {
                n: 0,
                stops: 0,
                gns: vec![],
                realized_sum: 0.0,
                theory_sum: 0.0,
                captures: vec![],
            }
        }
        fn push(&mut self, t: &TradeRec, theory: Option<f64>) {
            self.n += 1;
            self.stops += t.stopped as usize;
            self.gns.push(t.gn());
            if let Some(hl) = theory {
                self.realized_sum += t.realized();
                self.theory_sum += hl;
                self.captures.push(t.realized() / hl);
            }
        }
        fn line(&self) -> String {
            if self.n == 0 {
                return "n=0".to_string();
            }
            let mut gs = self.gns.clone();
            gs.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let mut cs = self.captures.clone();
            cs.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let mean = self.gns.iter().sum::<f64>() / self.n as f64;
            format!(
                "n={} 停出率={:.2}% 捕获率(聚合)={:.2}% 捕获率(中位)={:.2}% gn中位={:.4}% gn均值={:.4}%",
                self.n,
                self.stops as f64 / self.n as f64 * 100.0,
                if self.theory_sum > 0.0 { self.realized_sum / self.theory_sum * 100.0 } else { f64::NAN },
                if cs.is_empty() { f64::NAN } else { quantile(&cs, 0.5) * 100.0 },
                quantile(&gs, 0.5) * 100.0,
                mean * 100.0,
            )
        }
    }

    struct TradeSummary {
        dir_mismatch: usize,
        no_theory: usize,
        tainted: usize,
        per_win: Vec<Agg>,
        pooled_wf: Agg,
        pooled_oos: Agg,
        yearly: std::collections::BTreeMap<String, Agg>,
    }

    fn summarize_trades(
        trades: &[TradeRec],
        fin: &ParseLayer,
        windows: &[WfWindow],
        dates: &[String],
    ) -> TradeSummary {
        // ── 理论分母连接 + 方向对拍 ──
        let mut dir_mismatch = 0usize;
        let mut no_theory = 0usize;
        let theory = |tr: &TradeRec| -> Option<f64> {
            final_seg(fin, tr.seg_idx)
                .map(|s| (s.start_price.max(s.end_price) - s.start_price.min(s.end_price)) as f64)
        };
        for tr in trades {
            match final_seg(fin, tr.seg_idx) {
                Some(s) if s.direction != tr.dir => dir_mismatch += 1,
                None => no_theory += 1,
                _ => {}
            }
        }

        // ── 聚合 ──
        let mut per_win: Vec<Agg> = windows.iter().map(|_| Agg::new()).collect();
        let mut pooled_wf = Agg::new(); // 非 clipped 全 wf 窗
        let mut pooled_oos = Agg::new(); // 非 clipped 且 test_start ≥ OOS_START
        let mut yearly: std::collections::BTreeMap<String, Agg> = Default::default();
        // #1585：受段解析无效污染的交易不入任何聚合（只计数）。
        let tainted = trades.iter().filter(|tr| tr.parse_invalid).count();
        for tr in trades.iter().filter(|tr| !tr.parse_invalid) {
            let th = theory(tr);
            if let Some((wi, clipped)) = tr.win {
                per_win[wi as usize].push(tr, th);
                if !clipped {
                    pooled_wf.push(tr, th);
                    if windows[wi as usize].test_start >= OOS_START {
                        pooled_oos.push(tr, th);
                    }
                }
            }
            let y = dates[tr.entry_bar].get(..4).unwrap_or("????").to_string();
            yearly.entry(y).or_insert_with(Agg::new).push(tr, th);
        }

        TradeSummary {
            dir_mismatch,
            no_theory,
            tainted,
            per_win,
            pooled_wf,
            pooled_oos,
            yearly,
        }
    }

    /// #1585（X-39）：本 bar 段账本能否读来按段确认开平仓。带段端门 fail-closed 标记 ⟹ 拒读
    /// （`Err(失败笔号)`：本 bar 不按段确认开平仓，高水位不动）；否则 `Ok(confirmed 前缀长度)`。
    fn readable_confirmed_len(layer: &ParseLayer) -> Result<usize, usize> {
        match layer.segments_parse_invalid_from {
            Some(from) => Err(from),
            None => Ok(layer.segments_confirmed_len),
        }
    }

    /// #1585（X-39）：交易 `seg_idx` 对应的终局段（理论分母与方向对拍）。终局账本带标记 ⟹ 只在
    /// 失败段起点前的可读前缀里找，越过前缀 ⟹ `None`（计入「无理论分母」）。
    fn final_seg(fin: &ParseLayer, seg_idx: usize) -> Option<&Segment> {
        fin.segments_valid_prefix().get(seg_idx)
    }

    /// #1585（X-39）：λ 定参的段 gap 样本——终局账本可读前缀里起点落在训练窗（`in_train(start_index)`）
    /// 的段，段内逆向穿越深度（相对入场价分数，gap 口径与 S2 逐字一致）。F-08：特征路径截断至
    /// `cap`（train_end 当日末根 bar，含）——训练段的 end_index 可越过 train_end，λ 标定不得读取
    /// 其后的 test 期路径 bar。
    fn train_gaps(
        fin: &ParseLayer,
        bars: &[Bar],
        in_train: impl Fn(usize) -> bool,
        cap: usize,
    ) -> Vec<f64> {
        fin.segments_valid_prefix()
            .iter()
            .filter(|s| in_train(s.start_index))
            .map(|s| {
                let entry = s.start_price as f64;
                let path = &bars[s.start_index..=s.end_index.min(cap)];
                let g = match s.direction {
                    Direction::Up => {
                        (entry - path.iter().map(|b| b.low).min().unwrap() as f64) / entry
                    }
                    Direction::Down => {
                        (path.iter().map(|b| b.high).max().unwrap() as f64 - entry) / entry
                    }
                };
                g.max(0.0)
            })
            .collect()
    }

    /// 因果重放的逐 bar 状态（主回测循环与 #1585 负控共用同一份）。
    struct Replay {
        hw: usize, // segments_confirmed_len 高水位
        retreats: usize,
        open: Option<OpenTrade>,
        trades: Vec<TradeRec>,
        last_tradable_close: Option<f64>, // F-09/F-10：最近可交易 close
        invalid_runs: Vec<(usize, usize, usize)>, // #1585：(bar 起, bar 止, 失败笔)
        deferred_exits: usize,            // F-10：出场被 untradable bar 顺延计数
        untradable_entry_skips: usize,    // F-10：入场被 untradable bar 跳过计数
    }

    impl Replay {
        fn new() -> Self {
            Replay {
                hw: 0,
                retreats: 0,
                open: None,
                trades: Vec::new(),
                last_tradable_close: None,
                invalid_runs: Vec::new(),
                deferred_exits: 0,
                untradable_entry_skips: 0,
            }
        }

        fn step(
            &mut self,
            t: usize,
            bar: &Bar,
            layer: &ParseLayer,
            window_ended: impl Fn(u32) -> bool,
            entry_ctx: impl Fn(usize) -> (Option<(u32, bool)>, Option<f64>),
        ) -> usize {
            // #1585：先标记持仓，再允许价格退出；首次无效 bar 的退出同样剔出聚合。
            if layer.segments_parse_invalid_from.is_some() {
                if let Some(ot) = &mut self.open {
                    ot.parse_invalid = true;
                }
            }
            let mut boundary_closes = 0;
            let worse = |dir: Direction, a: f64, b: f64| -> f64 {
                match dir {
                    Direction::Up => a.min(b),
                    Direction::Down => a.max(b),
                }
            };
            // F-09：窗界平仓——持仓所属 test 窗结束后的首根 bar 上，按最近可交易 close
            // 强平（挂起止损则按劣侧），窗 k 统计不吸收窗 k+1 时段路径。窗外交易
            // （win=None）不受窗界约束，仅进逐年描述表。
            if let Some(ot) = &self.open {
                if let Some((wi, _)) = ot.win {
                    if window_ended(wi) {
                        let ot = self.open.take().unwrap();
                        let ltc = self
                            .last_tradable_close
                            .expect("入场 bar 必可交易（F-10），窗界强平必有可交易 close");
                        let (exit, stopped) = match ot.pending {
                            Some(Pending::Stop(px)) => (worse(ot.dir, px, ltc), true),
                            _ => (ltc, false),
                        };
                        boundary_closes += 1;
                        self.trades.push(TradeRec {
                            seg_idx: ot.seg_idx,
                            entry_bar: ot.entry_bar,
                            entry: ot.entry,
                            exit,
                            dir: ot.dir,
                            stopped,
                            win: ot.win,
                            parse_invalid: ot.parse_invalid,
                        });
                    }
                }
            }
            // F-10：挂起出场在下一可交易 bar 成交（段确认出场按 close，止损按劣侧）。
            if !bar.untradable {
                if self.open.as_ref().is_some_and(|ot| ot.pending.is_some()) {
                    let ot = self.open.take().unwrap();
                    let (exit, stopped) = match ot.pending {
                        Some(Pending::Seg) => (bar.close as f64, false),
                        Some(Pending::Stop(px)) => (worse(ot.dir, px, bar.open as f64), true),
                        None => unreachable!(),
                    };
                    self.trades.push(TradeRec {
                        seg_idx: ot.seg_idx,
                        entry_bar: ot.entry_bar,
                        entry: ot.entry,
                        exit,
                        dir: ot.dir,
                        stopped,
                        win: ot.win,
                        parse_invalid: ot.parse_invalid,
                    });
                }
            }
            // 止损（intrabar，入场 bar 当根不查——入场在收盘之后）
            if let Some(ot) = &mut self.open {
                if let (Some(lam), true, true) = (ot.lambda, ot.entry_bar < t, ot.pending.is_none())
                {
                    let stop_px = match ot.dir {
                        Direction::Up => ot.entry * (1.0 - lam),
                        Direction::Down => ot.entry * (1.0 + lam),
                    };
                    let hit = match ot.dir {
                        Direction::Up => (bar.low as f64) <= stop_px,
                        Direction::Down => (bar.high as f64) >= stop_px,
                    };
                    if hit && bar.untradable {
                        // F-10：untradable bar 不成交——止损挂起，顺延劣侧成交。
                        ot.pending = Some(Pending::Stop(stop_px));
                        self.deferred_exits += 1;
                    } else if hit {
                        // F-10：劣侧成交——跳空穿越时按当根 open（更劣者）成交。
                        let fill = worse(ot.dir, stop_px, bar.open as f64);
                        let ot = self.open.take().unwrap();
                        self.trades.push(TradeRec {
                            seg_idx: ot.seg_idx,
                            entry_bar: ot.entry_bar,
                            entry: ot.entry,
                            exit: fill,
                            dir: ot.dir,
                            stopped: true,
                            win: ot.win,
                            parse_invalid: ot.parse_invalid,
                        });
                    }
                }
            }
            self.seg_step(t, bar, layer, entry_ctx);
            boundary_closes
        }

        /// 逐 bar 的「#1585 闸 + 段确认开平仓」，在价格侧出场（窗界强平/顺延出场/止损）之后调用。
        /// `entry_ctx(t)` 给开仓所需的 (窗, λ)。段解析无效 ⟹ 拒读段账本：记标记区间、
        /// 跳过按段确认开平仓（高水位不动），只补最近可交易 close。
        fn seg_step(
            &mut self,
            t: usize,
            bar: &Bar,
            layer: &ParseLayer,
            entry_ctx: impl Fn(usize) -> (Option<(u32, bool)>, Option<f64>),
        ) {
            let c = match readable_confirmed_len(layer) {
                Ok(c) => c,
                Err(from) => {
                    match self.invalid_runs.last_mut() {
                        Some(r) if r.1 + 1 == t && r.2 == from => r.1 = t,
                        _ => self.invalid_runs.push((t, t, from)),
                    }
                    if !bar.untradable {
                        self.last_tradable_close = Some(bar.close as f64);
                    }
                    return;
                }
            };
            if c < self.hw {
                self.retreats += 1; // #88 复活级联回退——因果决策照留，如实计数
            }
            if c > self.hw {
                for j in self.hw..c {
                    // 段 j 确认于 bar t
                    if let Some(ot) = &self.open {
                        // F-10：出场被顺延的持仓可落后于确认进度，不算生命周期破缺。
                        assert!(
                            ot.seg_idx >= j || ot.pending.is_some(),
                            "开仓段 {} 早于确认段 {}，生命周期破缺",
                            ot.seg_idx,
                            j
                        );
                    }
                    if self
                        .open
                        .as_ref()
                        .is_some_and(|ot| ot.seg_idx == j && ot.pending.is_none())
                    {
                        if bar.untradable {
                            // F-10：untradable bar 不成交——段确认出场挂起，顺延 close 成交。
                            self.open.as_mut().unwrap().pending = Some(Pending::Seg);
                            self.deferred_exits += 1;
                        } else {
                            let ot = self.open.take().unwrap();
                            self.trades.push(TradeRec {
                                seg_idx: ot.seg_idx,
                                entry_bar: ot.entry_bar,
                                entry: ot.entry,
                                exit: bar.close as f64,
                                dir: ot.dir,
                                stopped: false,
                                win: ot.win,
                                parse_invalid: ot.parse_invalid,
                            });
                        }
                    }
                    if self.open.is_none() && !bar.untradable {
                        let (win, lambda) = entry_ctx(t);
                        self.open = Some(OpenTrade {
                            seg_idx: j + 1,
                            entry_bar: t,
                            entry: bar.close as f64,
                            dir: opposite(layer.segments[j].direction),
                            lambda,
                            win,
                            pending: None,
                            parse_invalid: false,
                        });
                    } else if self.open.is_none() {
                        // F-10：untradable bar 不成交——该入场机会跳过（不顺延，保守口径）。
                        self.untradable_entry_skips += 1;
                    }
                }
                self.hw = c;
            }
            if !bar.untradable {
                self.last_tradable_close = Some(bar.close as f64);
            }
        }
    }

    /// S3 主回测：`cargo test --release --lib -- --ignored --nocapture s3_capture_oos_real_btc`
    /// 可选 `S3_BARS=<n>` 截断冒烟。
    #[test]
    #[ignore = "S3 捕获率回测；需 BTC 全历史（329MB）+ 全量增量重放；手动 --ignored 跑"]
    fn s3_capture_oos_real_btc() {
        let cfg = ThetaConfig::default();
        let mut ds = data::load_by_symbol("BTC", &cfg)
            .expect("需 BTC 数据（analysis/data_cache/btc_1m_full.json）");
        if let Some(k) = std::env::var(crate::theta_v0::env_registry::S3_BARS)
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
        {
            ds.bars.truncate(k);
            ds.dates.truncate(k);
        }
        let windows = PREREG_WINDOWS
            .iter()
            .find(|sw| sw.symbol == "BTC")
            .expect("BTC 不在 PREREG_WINDOWS（预注册缺口，非静默兜底）")
            .wf_anchored;

        // ── 终局批量解析：理论价差分母 + 训练窗 λ 定参（与增量终态 bit-exact）──
        let fin = parse_layer(&ds.bars, &cfg);
        // #1585：终局账本带标记 ⟹ λ 定参与理论分母只读失败段起点前的可读前缀。
        let fin_note = parse_invalid_note(&fin);
        let day = |bar: usize| ds.dates[bar].get(..10).unwrap_or("");

        // λ_w = clamp(train 段 gap p95, [LO, HI])；gap 口径与 S2 逐字一致（`train_gaps`）。
        let mut lambda_of_win: Vec<Option<f64>> = Vec::with_capacity(windows.len());
        for w in windows {
            // F-08：train_end 当日末根 bar 下标（dates 升序，partition_point = 首个越界位）。
            let n_train_bars = ds
                .dates
                .partition_point(|d| d.get(..10).unwrap_or("") <= w.train_end);
            let cap = n_train_bars.saturating_sub(1);
            let in_train = |start: usize| {
                let d = day(start);
                d >= w.train_start && d <= w.train_end
            };
            let mut gaps = train_gaps(&fin, &ds.bars, in_train, cap);
            gaps.sort_by(|a, b| a.partial_cmp(b).unwrap());
            lambda_of_win.push(if gaps.is_empty() {
                None
            } else {
                Some(quantile(&gaps, LAMBDA_Q).clamp(LAMBDA_LO, LAMBDA_HI))
            });
        }
        let win_of_day = |d: &str| -> Option<(u32, bool)> {
            windows
                .iter()
                .find(|w| d >= w.test_start && d <= w.test_end)
                .map(|w| (w.i, w.clipped))
        };

        // ── 因果重放 ──
        let mut incr = ParseLayerIncr::new(&cfg);
        let mut rp = Replay::new();
        let mut boundary_closes = 0usize; // F-09：窗界强平计数
        let entry_ctx = |t: usize| {
            let win = win_of_day(day(t));
            (win, win.and_then(|(wi, _)| lambda_of_win[wi as usize]))
        };
        for (t, bar) in ds.bars.iter().enumerate() {
            let layer = incr.append(*bar);
            boundary_closes += rp.step(
                t,
                bar,
                &layer,
                |wi| day(t) > windows[wi as usize].test_end,
                &entry_ctx,
            );
        }
        let dropped_open = rp.open.is_some() as usize;
        let Replay {
            retreats,
            trades,
            invalid_runs,
            deferred_exits,
            untradable_entry_skips,
            ..
        } = rp;

        let TradeSummary {
            dir_mismatch,
            no_theory,
            tainted,
            per_win,
            pooled_wf,
            pooled_oos,
            yearly,
        } = summarize_trades(&trades, &fin, windows, &ds.dates);

        println!(
            "== S3 捕获率回测（因果重放，bars={} segments={}）==",
            ds.bars.len(),
            fin.segments.len()
        );
        println!(
            "trades={} 回退={} 方向失配={} 无理论分母={} 尾部未平={} 窗界强平={} 出场顺延={} 入场跳过(untradable)={}",
            trades.len(), retreats, dir_mismatch, no_theory, dropped_open,
            boundary_closes, deferred_exits, untradable_entry_skips,
        );
        if let Some(note) = &fin_note {
            println!("⚠ 终局账本 {note}（λ 定参/理论分母只读可读前缀）");
        }
        println!(
            "段解析无效（segment_parse_invalid）：增量标记 bar 区间(起,止,失败笔)={:?} 受污染交易={}（不入聚合）",
            invalid_runs, tainted
        );
        println!("-- 逐窗（λ 由各自 train p95 定，clipped 不入主判据）--");
        for (w, agg) in windows.iter().zip(&per_win) {
            println!(
                "win{} {}→{}{} λ={} {}",
                w.i,
                w.test_start,
                w.test_end,
                if w.clipped { " [clipped]" } else { "" },
                lambda_of_win[w.i as usize].map_or("None".into(), |l| format!("{:.3}%", l * 100.0)),
                agg.line(),
            );
        }
        println!("-- 池化 wf（非 clipped 全窗）-- {}", pooled_wf.line());
        println!(
            "-- 池化 OOS（test_start ≥ {OOS_START}，非 clipped，S4 主判据基）-- {}",
            pooled_oos.line()
        );
        println!("-- 逐年（含窗外交易，描述性）--");
        for (y, agg) in &yearly {
            println!("{} {}", y, agg.line());
        }
    }

    /// #1585（X-39）负控：锁主回测接线（`Replay::step`／`final_seg`／`train_gaps` 即主循环所调）。
    /// 生产增量逐 bar 重放到一根「有新确认段且有持仓」的 bar，给该 bar 的层注入段端门 fail-closed
    /// 标记 ⟹ 本 bar 不按段确认开平仓（高水位、交易、持仓不动）、持仓打污染、标记区间入报告；终局账本
    /// 标记于中间某段起点笔 ⟹ 理论分母与 λ 样本只到该段之前。正控：同一层无标记时照常确认开平仓、
    /// 整本账可读。
    #[test]
    fn tb1585_s3_refuses_segment_confirmation_under_marker() {
        let cfg = ThetaConfig::default();
        let bars = crate::theta_v0::parser::tb1581_tests::tb1581_bars(0x1581_0000, 4_000);
        let no_win = |_: usize| (None, None);

        // ── 逐 bar 增量：主循环同一个 step ──
        let mut incr = ParseLayerIncr::new(&cfg);
        let mut rp = Replay::new();
        let mut probe = None;
        for (t, bar) in bars.iter().enumerate() {
            let layer = incr.append(*bar);
            assert_eq!(layer.segments_parse_invalid_from, None, "生产口径无门失败");
            if probe.is_none()
                && rp.open.is_some()
                && rp.trades.len() >= 2
                && layer.segments_confirmed_len > rp.hw
            {
                probe = Some((t, *bar, layer.clone(), rp.hw, rp.trades.len()));
            }
            rp.step(t, bar, &layer, |_| false, no_win);
        }
        assert!(
            rp.trades.len() > 2 && rp.hw > 0,
            "正控：无标记时按段确认开平仓"
        );
        assert!(rp.trades.iter().all(|tr| !tr.parse_invalid));
        assert!(rp.invalid_runs.is_empty());

        let (t, bar, layer, hw0, n0) = probe.expect("随机 K 线须有持仓中的确认 bar");
        // 同一 bar 回到注入前的状态重放：先正控（无标记）再负控（有标记）。
        let replay_to = |stop: usize| {
            let mut incr = ParseLayerIncr::new(&cfg);
            let mut rp = Replay::new();
            for (u, b) in bars[..stop].iter().enumerate() {
                let l = incr.append(*b);
                rp.step(u, b, &l, |_| false, no_win);
            }
            rp
        };
        let mut pos = replay_to(t);
        assert_eq!((pos.hw, pos.trades.len()), (hw0, n0));
        pos.step(t, &bar, &layer, |_| false, no_win);
        assert!(pos.hw > hw0, "正控：该 bar 无标记时确认新段");

        let mut neg = replay_to(t);
        let open_seg = neg.open.as_ref().map(|ot| ot.seg_idx);
        let mut marked = layer.clone();
        marked.segments_parse_invalid_from = Some(0);
        neg.step(t, &bar, &marked, |_| false, no_win);
        assert_eq!(neg.hw, hw0, "标记 bar 高水位不动");
        assert_eq!(neg.trades.len(), n0, "标记 bar 不按段确认平仓");
        assert_eq!(
            neg.open.as_ref().map(|ot| ot.seg_idx),
            open_seg,
            "标记 bar 不开新仓"
        );
        assert!(
            neg.open.as_ref().is_some_and(|ot| ot.parse_invalid),
            "持仓打污染"
        );
        assert_eq!(neg.invalid_runs, vec![(t, t, 0)], "标记区间入报告");

        // ── 终局账本：理论分母与 λ 样本只读可读前缀 ──
        let fin = parse_layer(&bars, &cfg);
        let cap = bars.len() - 1;
        let k = fin.segments.len() / 2;
        assert!(k > 0, "正控：随机 K 线须有段");
        assert!(final_seg(&fin, k).is_some());
        assert_eq!(
            train_gaps(&fin, &bars, |_| true, cap).len(),
            fin.segments.len()
        );
        assert!(parse_invalid_note(&fin).is_none());
        let from = fin
            .strokes
            .iter()
            .position(|s| s.start_index == fin.segments[k].start_index)
            .expect("段起点必为某笔起点");
        let mut fin_marked = fin.clone();
        fin_marked.segments_parse_invalid_from = Some(from);
        assert!(
            parse_invalid_note(&fin_marked).is_some_and(|n| n.contains("segment_parse_invalid"))
        );
        assert!(final_seg(&fin_marked, k - 1).is_some());
        assert!(final_seg(&fin_marked, k).is_none(), "失败段起无理论分母");
        assert_eq!(
            train_gaps(&fin_marked, &bars, |_| true, cap).len(),
            k,
            "λ 样本只到失败段之前"
        );
    }
    #[test]
    fn tb1585_first_invalid_bar_taints_each_price_exit() {
        let cfg = ThetaConfig::default();
        let bar = Bar {
            source_index: 1,
            timestamp: 1,
            open: 100,
            high: 101,
            low: 90,
            close: 95,
            volume: 1.0,
            untradable: false,
        };
        for marked in [false, true] {
            for exit_kind in 0..3 {
                let mut layer = parse_layer(&[], &cfg);
                layer.segments_parse_invalid_from = marked.then_some(0);
                let mut rp = Replay::new();
                rp.last_tradable_close = Some(102.0);
                rp.open = Some(OpenTrade {
                    seg_idx: 0,
                    entry_bar: 0,
                    entry: 100.0,
                    dir: Direction::Up,
                    lambda: Some(0.05),
                    win: Some((0, false)),
                    pending: (exit_kind == 1).then_some(Pending::Seg),
                    parse_invalid: false,
                });
                let boundary = rp.step(1, &bar, &layer, |_| exit_kind == 0, |_| (None, None));
                assert!(rp.open.is_none());
                assert_eq!(rp.trades.len(), 1);
                let tr = &rp.trades[0];
                assert_eq!(tr.parse_invalid, marked, "exit_kind={exit_kind}");
                assert_eq!(tr.exit, if exit_kind == 0 { 102.0 } else { 95.0 });
                assert_eq!(tr.stopped, exit_kind == 2);
                assert_eq!(boundary, usize::from(exit_kind == 0));
                let windows = [WfWindow {
                    i: 0,
                    train_start: "2022-01-01",
                    train_end: "2022-12-31",
                    test_start: OOS_START,
                    test_end: "2023-06-30",
                    clipped: false,
                }];
                let summary =
                    summarize_trades(&rp.trades, &layer, &windows, &[OOS_START.to_string()]);
                let included = usize::from(!marked);
                assert_eq!(summary.per_win[0].n, included, "逐窗");
                assert_eq!(summary.pooled_wf.n, included, "wf");
                assert_eq!(summary.pooled_oos.n, included, "OOS");
                assert_eq!(
                    summary.yearly.values().map(|agg| agg.n).sum::<usize>(),
                    included,
                    "逐年"
                );
                assert_eq!(summary.tainted, usize::from(marked));
                assert_eq!(summary.no_theory, 1, "诊断保留全部交易");
                assert_eq!(summary.dir_mismatch, 0);
                assert_eq!(
                    rp.invalid_runs,
                    if marked { vec![(1, 1, 0)] } else { vec![] }
                );
            }
        }
    }
}
