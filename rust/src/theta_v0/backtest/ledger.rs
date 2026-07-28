//! ★账本类型层 seam（B-M4，#86；#90 TwLedgerThread 收敛）：typed ledger 载体 +
//! 入场冻结快照 + 持仓转移轨迹 + TW 账本线程，自 runner.rs 纯移动
//!（设计 chanlun/plans/runner-rs-seam-designs-20260721.md §M4）。
//!
//! ★三方合并（kimi-nest-mainline-20260717）：并入 ours 独有 `VoiceVerdictRec`
//!（#201 阶段 B 裁决账本轨载体；其余类型两侧逐字一致 ⟹ seam 版即并集）。
//!
//! 对外路径经 `runner` 门面 `pub use` 保持不变（`runner::TypedTrade` 等）。

use super::super::strategy::ledger::{tw_step, RiskPolicy, TwEvent, TwState};
use super::super::{classifier, strategy};
use super::metrics;

/// 腿级 typed 交易记录（G4 #134：《完整的策略.pdf》§9 typed exit 的统计层载体）。
///
/// 由**生产 π fill loop**（[`pi_theta_fill_loop`]，唯一状态机）逐腿输出——腿进 `next_active`
/// 开条目、腿离场（interpret 规则2 反向关闭 / §13 AncOK 剪 / 窗口终点 censored）关条目，
/// `exit_type` 经 [`super::super::strategy::interp::reverse_exit_type`] 单源判据产出。
/// 下游 `build_mu_from_bars` 从本记录构造 `MuObservation`/`ResidualTrade`（替换 PDF §9
/// 点名废弃的 τ^reverse 平行简化状态机，codex-q1-spec G4 终裁）。
///
/// **价格口径**：`entry_px`/`exit_px` = 腿进/出 active 的**决策 bar close**（F_t 可测，名义
/// 单位口径，与旧训练路径同价格语义——G4 只改出场时点规则，不改价格口径）；非账户 fill 价
/// （腿级无独立成交，净持仓聚合后账户级 P&L 归 equity_curve 路径）。
///
/// **RiskExit 通道**（#124 P1 已落地）：`force_flat` ⟹ 组合层上游短路清活动腿，经
/// `StepTrace.risk_exits` 产 `RiskExit`（幽灵腿堵口）。**CloseOverlay 通道**（#124 裁定4）：
/// TW StageII 重叠腿经 `StepTrace.overlay_closes` 产 `CloseReverseOpen`（生产触发可达性受
/// 账本语义约束，见 fill loop TW 初始化注释）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TypedTrade {
    /// 开腿候选的全互斥分类 z（`z_of_candidate` 塔真值，与生产 χ 查询同口径）。
    pub entry_z: super::mu_estimator::MuClass,
    /// 腿声部身份（`ActiveLeg.id`，跨 bar 稳定 ElementId）。
    pub voice_id: classifier::recursive_tower::ElementId,
    /// 腿进 active 的决策 bar。
    pub entry_bar: usize,
    /// 腿离场决策 bar（`Hold` censored = 窗口末可交易 bar）。
    pub exit_bar: usize,
    /// PDF §9 typed exit（interp.rs 单源五枚举）。
    pub exit_type: super::super::strategy::interp::ExitType,
    /// 入场决策 bar close（×tick，名义单位口径）。
    pub entry_px: f64,
    /// 离场决策 bar close（×tick）。
    pub exit_px: f64,
    /// 离场是否来自 §13 结构剪枝（AncOK 连带剪/Stale prune，父驱动同 bar 连带、非独立反向
    /// 信号触发）——ws-g5interp flag：结构剪枝的 P&L 分布与真信号平仓不同，混桶偏 μ。本标记
    /// 使 μ 侧**可分离**；`build_mu_from_bars` 现口径仍全部入 μ（排除/分桶是新统计决策，
    /// 归 #135 prereg，不在此静默改口径）。反向关闭/RiskExit/censored Hold 恒 false。
    pub via_structural_prune: bool,
    /// ★A9（Task #166，级别容器.pdf p14/§13）：position instance 严格身份四元组
    /// `hash(carrier, entry_certificate, side, generation)`。同一 carrier（`voice_id`）先后多次
    /// campaign 由 `generation` 单调区分——`voice_id` 会碰撞（close→reopen 复用同 ElementId），
    /// `position_node_id` 不碰撞（generation +1）。供跨笔同 carrier campaign 归因/去重（当前 μ 层
    /// 按 `entry_z` 逐笔独立观测，不消费本字段——它是身份完备性的账本层载体，非 μ 统计输入）。
    pub position_node_id: super::super::strategy::interp::PositionNodeId,
    /// ★A6（prereg-rev2-20260704，codex-ruling-696）：入场结构止损距离 d=|entry_px−stop|（美元）。
    /// μ_R=E[Y/d] co-primary 门的分母，ex-ante 可得（risk.rs structural_stop 在决策 bar 因果算）。
    /// `None` = 不可得（μ_R 剔除，raw μ 保留）。透传自 `LedgerOpen::entry_stop_dist`，
    /// `build_mu_from_bars` 塞进 [`ResidualTrade::d`](super::mu_estimator::ResidualTrade)。
    pub entry_stop_dist: Option<f64>,
    /// ★A7（Task #165，《完整的策略.pdf》§6 z「两次快照」+ §9 typed exit）：出场时刻的 z 快照。
    ///
    /// 与 `entry_z` 是**同一 [`MuClass`](super::mu_estimator::MuClass) 类型的两次快照**（时刻不同、
    /// 账本态不同）：结构身份维（level/δ/i_class/parent_dir/horizontal/force_state/σ_higher/门链维）
    /// 入场冻结不重采样（PDF §6 `σ_higher: 入场时上级方向`——按定义入场值），唯一逐 bar 变化的账本态
    /// 三元 `{t_stage, eta_bucket, risk_mode}` 刷新到出场 bar 决策点真值。由 [`super::selector::exit_z_of`]
    /// 单源构造（`entry_z` + 出场 bar `ext_i`），构造被迫唯一（在无触发候选的出场点重分类结构维 =
    /// 伪造不存在的候选 = 声明膨胀，231号）。
    ///
    /// **消费侧未定（A7 裁量分离）**：μ 估计器按 `(entry_z, exit_z, exit_type)` 分桶的语义是设计裁量，
    /// 待 codex 裁决——当前 μ 层不消费本字段（`build_mu_from_bars` 仍按 `entry_z` 逐笔独立观测），
    /// 本字段是出场侧完备性的账本层载体（同 `position_node_id` A9 先例）。
    pub exit_z: super::mu_estimator::MuClass,
    /// ★B1（步骤4，dw-sizing-diag-20260705，codex review conditional 修复）：**入场时刻 sizing
    /// target 快照**——开腿当步 `SepLeg.q_units`（=`base_units×w_depth×w_dir`，含 dir_weight；post
    /// gross-cap；coverage.rs:2321 从 `LegTarget.units` 透传）。**非逐 bar fill 后实际腿级持仓**——是
    /// 入场决策点的目标单位，不是执行期 fill 累计。**不进 μ estimand**（μ 保持单位边际 qty=1.0，696 域，
    /// `build_mu_from_bars` 不读本字段）。
    ///
    /// **诊断边界（codex review）**：本字段供"入场 sizing target 逐笔分布"诊断——对比 μ 单位边际
    /// qty=1.0，看 dir_weight 改变了哪些腿的入场规模。**账户级 execution R 分解由 `r_decomp` 负责**
    /// （runner.rs:1489 `RDecomposition::assemble`，`cum_price_pnl=Σ units·Δpx` 真实账户 sizing 加权），
    /// 本字段**不用于逐笔 execution P&L**——真要逐笔 execution P&L 需 per-bar exposure / fill ledger，
    /// 非本字段（本字段仅入场 target 快照）。
    pub units: f64,
}

/// 声部裁决账本记录（#201 阶段 B：per-voice 裁决序列的 runner 落账单元，**schema 冻结**——
/// 冻结口径见 [`super::super::strategy::coverage::VoiceVerdict`] doc，本结构只加 `bar` 轴）。
///
/// 由**生产 π fill loop**（[`pi_theta_fill_loop`]，唯一状态机）逐 bar 输出：消费组合层
/// `StepTrace.verdicts`，每 bar 每**解释器裁决域**持仓声部恰一枚（`prev_active =
/// verdicts ⊎ silent_drops` 划分；§13 结构剪除腿不进本轨——其生命周期事件在 typed
/// ledger 轨 `via_structural_prune=true` 显式）。**显式 Hold 是本轨核心**：延续持有的声部
/// 此前无任何 typed 记录（Hold 隐式），本票起逐 bar 落 `ExitType::Hold` 显式裁决。
///
/// **加轨不减轨**：本轨与 typed ledger（成交/生命周期记录）、账户镜像、opsem dump、TW
/// 账本全部正交——Hold 无成交，故不入 `TypedTrade`（其是平仓记录）；窗口终点 censored
/// 强平仍是 typed ledger 轨语义（`ExitType::Hold` 关条目），不在本轨（本轨只载 bar 内
/// 解释器裁决）。阶段 C 换内核在本契约上对齐。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoiceVerdictRec {
    /// 裁决 bar（fill loop 主循环 `i`，决策 bar）。
    pub bar: usize,
    /// 裁决对象腿（声部身份 level/dir/ElementId 全在）。
    pub leg: super::super::strategy::interp::ActiveLeg,
    /// typed 裁决（单源五枚举；Hold = 显式持有）。
    pub exit_type: super::super::strategy::interp::ExitType,
}

/// ★`TypedTrade` 账本行 schema 版本（每次账本层增列 +1；显式版本标记）。
///
/// **版本史**：v3 增 `exit_z`（出场 z 快照，A7 #165）；v4 增 `units`（腿级 sizing 目标，B1 步骤4）。
///
/// **B30 prereg 前置清单联动声明（不静默，team-lead 令）**：μ **样本** schema
/// （[`MuObservation`](super::mu_estimator::MuObservation) = `{class, x_gamma}`）**未变**——
/// A7（`exit_z`）/B1（`units`）只在账本层增列，μ 消费侧仍按 `entry_z` 单位边际（qty=1.0，696 域）。
/// 任何后续把 `exit_z`/`units` 接入 μ 分桶键或 sizing 加权的工位（codex 裁决后）须新开 prereg
/// 冻结口径，不得静默改 estimand（formalization-validity-domain / B30 前置清单 / 696）。
pub const TYPED_TRADE_SCHEMA_VERSION: u32 = 4;

/// ledger 在飞条目（开腿登记，关腿时结算为 [`TypedTrade`]）。
pub(super) struct LedgerOpen {
    pub(super) entry_bar: usize,
    pub(super) entry_px: f64,
    pub(super) entry_z: super::mu_estimator::MuClass,
    /// ★A6（prereg-rev2-20260704）：入场结构止损距离 d=|entry_px−stop|（美元，ex-ante）。
    /// `Some(d)` = 决策 bar 因果 BspPoint + structural_stop 可算；`None` = structural_stop 返 None
    /// （非该方向交易点）/ BspPoint 缺失 ⟹ 下游 μ_R 剔除（诚实缺口，231号）。
    pub(super) entry_stop_dist: Option<f64>,
    /// ★族A 修复：入场冻结的结构止损值（[`super::super::types::Tick`]，开仓 bar 由
    /// [`entry_structural_stop`] 一次性算）。逐 bar 风控门 [`k_theta_risk_gate`] 读此冻结值判
    /// `stop_hit`——消除旧路径用 drifted `leg.source_index`（carrier 走势 ρ）回查 `classification`
    /// 的覆盖度缺陷（exitfix-research §族A）。对齐 nautilus `record_held_voice`（入场一次性写
    /// HeldVoice.stop）。`None` = 该方向无结构止损（非交易点，与 `entry_stop_dist` 同口径）。
    pub(super) entry_stop: Option<super::super::types::Tick>,
    /// 入场角色垂直轴（腿声部身份入场时固定）——`reverse_exit_type`/silent drop 判据输入。
    pub(super) entry_v: super::super::strategy::coverage::Vertical,
    /// ★A9（Task #166，级别容器.pdf p14/§13）：position instance 严格身份四元组
    /// `hash(carrier, entry_certificate, side, generation)`。入场时刻冻结（carrier=腿 id、
    /// 证书=开仓 Candidate 坐标、side=Candidate 方向、generation=同 carrier campaign 高水位）。
    /// codex a9-posnode 裁定 C：身份归**账本生命周期层**（本结构 + `TypedTrade`），不进 `ActiveLeg`
    /// 结构层——`ActiveLeg` 每 bar 从树重建拿不到 campaign 状态。
    pub(super) position_node_id: super::super::strategy::interp::PositionNodeId,
    /// ★opsem-dump（基因 073a）：入场时刻操作语义快照——env-gated `OPSEM_DUMP_DIR` 启用时由
    /// [`OpsemDump::write_trade`] 消费；未启用路径 `Default::default()` 零字段零开销（bit-exact）。
    /// 不进生产语义/μ 桶键/J_Θ 排序——纯只读外化（同 `dump_deltafree_pertrade` 先例）。
    pub(super) opsem: OpsemEntrySnapshot,
    /// ★B1（步骤4）：腿级 sizing 目标（透传 `SepLeg::q_units`，含 dir_weight）。开腿登记时从
    /// `step_trace.sep_legs` 冻结，关腿结算透传 `TypedTrade::units`。不进 μ estimand（696 域）。
    pub(super) units: f64,
}

/// 入场时刻操作语义快照（仅 env-gated dump 消费，零生产影响）。
#[derive(Default, Clone)]
pub(super) struct OpsemEntrySnapshot {
    /// 入场候选 level（Candidate.level，ℓ_g）。
    pub(super) cand_level: u32,
    /// 入场候选 source_index（bsp 触发点 L0 K 序）。
    pub(super) cand_source_index: usize,
    /// 入场候选 bsp bits（6 bit 非互斥）。
    pub(super) cand_bits: u8,
    /// #542：生产者签发的三类完整身份；随决策层开仓候选冻结，缺席时 dump 显式写 null。
    pub(super) third_class_entry:
        Option<super::super::types::ThirdClassEntryIdentity>,
    /// 入场候选方向 σ_g（VoiceSide 编码：Long/Short/Flat）。
    pub(super) cand_dir: &'static str,
    /// 入场候选最小成立类号（1/2/3，u8::MAX=无）。
    pub(super) cand_bsp_class: u8,
    /// 入场候选 N^δ 区间套确认（nest_confirmed）。
    pub(super) cand_nest_confirmed: bool,
    /// ★R5-c：入场候选区间套深度 Ndepth（structural_nest_depth 读数，= 从执行级向上连续包含
    /// source_index 的塔层数，与生产门 build_nest_certificate rungs.len() 同口径）。dump 专用——
    /// **不进 entry_z/MuClass/μ 桶键**（R5-1 铁律），替代旧 `entry_z.nest_depth`（π 路径恒 None）。
    pub(super) nest_depth: u8,
    /// 入场候选角色 R(g)=(H,V,δ) 的 18 类索引字符串。
    pub(super) cand_role: &'static str,
    /// 入场候选 A 段 MACD 面积（ForceProxies.seg_a.macd_area；None=无力度源，非一类背驰候选）。
    pub(super) seg_a_macd_area: Option<f64>,
    /// 入场候选 C 段 MACD 面积（ForceProxies.seg_c.macd_area；None=同上）。
    pub(super) seg_c_macd_area: Option<f64>,
    /// 入场候选 A 段 DIF 峰绝对值。
    pub(super) seg_a_dif_peak: Option<f64>,
    /// 入场候选 C 段 DIF 峰绝对值。
    pub(super) seg_c_dif_peak: Option<f64>,
    /// 入场候选 β^div 力度支配态（Weak=Dominated=背驰；None=无力度源）。
    pub(super) force_state: Option<&'static str>,
    /// 入场时刻解释器喂入候选集大小（χ 过滤后 step_gamma_trade.len()）。
    pub(super) gamma_count: usize,
    /// ★L1-P1：χ 门是否参与本步过滤（π overlay 生产路径 chi=None ⟹ false）。
    pub(super) chi_filter_active: bool,
    /// ★L2-P4：B1 sizing 是否成功从 sep_legs 取得（false ⟹ units=0.0 是兜底）。
    pub(super) b1_sizing_available: bool,
    /// ★L2-P5/P6：真嵌套深度（根=0，沿 parent_id 链；`element_depth` 同口径）。
    pub(super) voice_tree_depth: u8,
    /// ★L2-P6：该深度实际消费的 w_depth（[0.60,0.30,0.10] 按深度索引，越界 0.0）。
    pub(super) w_depth_at_entry: f64,
    /// 入场时刻活动腿数（prev_active.len()，含即将开仓腿的兄弟）。
    pub(super) prev_active_count: usize,
    /// ★R5-a：入场步 LexArgmin 的 top-3 J_Θ 候选键（字典序升序，`(JThetaKey, control)`）。首名 = p_star
    /// 选址（被选），余两名 = 被拒的次优。来自 `StepTrace.lex_top3`（pi_theta_step_traced 正常路径
    /// 填充）。dump 专用——不进 p_star/J_Θ/χ（R5-1 铁律）。空 vec = 该步无开仓（不应进 opsem_snap，
    /// 因 opsem_snap 仅对 opened 构造）。
    pub(super) lex_top3: Vec<(strategy::intent::JThetaKey, f64)>,
    /// 入场腿父容器 ElementId（σ_p 来源；None=真边界胚元 ∂，σ_p=0=Ambient）。
    pub(super) parent_id: Option<(u32, u64)>,
    /// 入场时刻 TW 阶段（tw.stage，入场决策点相位）。
    pub(super) t_stage: &'static str,
    /// 入场时刻 η_t bucket（tw_policy.eta_bucket(&tw)）。
    pub(super) eta_bucket: &'static str,
    /// 入场时刻 risk_mode（margin 五态）。
    pub(super) risk_mode: &'static str,
}

/// ★交易轨迹配对（平仓事件 → [`metrics::TradeRecord`]）。
///
/// ★持仓状态转移轨迹追踪（v1 方向中性，替代 long-only `track_close_to_trade`）。
///
/// `units` 有符号（正=多/负=空/0=空仓）。一次成交（`units_before → units_after`）可能：
/// - **纯开仓**（before=0, after≠0）：记新 `pos_entry_bar = Some(exit_bar)`（此 bar 为入场 bar）。
/// - **纯平仓**（before≠0, after=0）：配对 `pos_entry_bar` 产 TradeRecord（方向 = before.signum()：
///   before>0 ⟹ long=true 平多；before<0 ⟹ long=false 平空），清 entry_bar。
/// - **翻转**（before·after<0，先平后开同一笔）：先配对旧方向 trade（方向 = before.signum()，
///   qty = |before|），再记新 entry_bar（新方向持仓从此 bar 入场）。
/// - **同向减仓未到 0**（before·after>0 且 |after|<|before|）：**逐 fill 产 TradeRecord**
///   （qty = 减掉的手数，entry_bar 延续首次入场，v0 单标量近似）——bughunt F-06：否则部分
///   减仓的已实现 PnL/敞口在 trades 轨迹无对应记录，§4 随机对照 same-caliber 重算与敞口
///   归一化漏计（反例 100买10→110减5→120平5 漏 49.685）。
/// - **同向加仓**（before·after>0 且 |after|>|before|）：entry_bar 不变（延续首次入场，
///   v0 单标量近似），不产记录。
///
/// `forced` 标记窗口终点强平（不计 n_trades≥30 统计功效门槛）。qty = 平掉的绝对手数 = |before|
/// （翻转/全平时平掉全部旧仓；v0 build_exit_order 全平 ⟹ 配对唯一）。
pub(super) fn track_position_transition(
    trades: &mut Vec<metrics::TradeRecord>,
    pos_entry_bar: &mut Option<usize>,
    units_before: f64,
    units_after: f64,
    exit_bar: usize,
    forced: bool,
) {
    // ★显式三态符号（−1/0/+1）：f64::signum 对 0.0 返回 +1.0（不返回 0），不能用于判持仓有无。
    let sign = |x: f64| -> i8 {
        if x > 0.0 {
            1
        } else if x < 0.0 {
            -1
        } else {
            0
        }
    };
    // 闭合旧仓：before≠0 且符号改变（到 0 / 翻转）⟹ 配对产 trade。
    let closed = sign(units_before) != 0 && sign(units_before) != sign(units_after);
    if closed {
        if let Some(entry_bar) = pos_entry_bar.take() {
            let hold_bars = exit_bar.saturating_sub(entry_bar).max(1);
            trades.push(metrics::TradeRecord {
                entry_bar,
                exit_bar,
                hold_bars,
                qty: units_before.abs(), // 平掉的绝对手数 = |平仓前持仓|
                long: units_before > 0.0, // 方向 = 平仓前持仓方向（多/空）
                forced_close: forced,
            });
        }
    }
    // F-06：部分减仓（同向未到 0，|after|<|before|）⟹ 逐 fill 产 TradeRecord（qty=本次减掉
    // 的手数，entry_bar 延续首次入场）。与 `apply_order` 逐 fill push trade_pnls 口径对齐，
    // 消除"减仓 PnL 有账无迹"的口径错配（消费方：§4 随机对照 same-caliber/敞口归一化）。
    let same_side = sign(units_before) != 0 && sign(units_before) == sign(units_after);
    if same_side && units_after.abs() < units_before.abs() - 1e-12 {
        if let Some(entry_bar) = *pos_entry_bar {
            let hold_bars = exit_bar.saturating_sub(entry_bar).max(1);
            trades.push(metrics::TradeRecord {
                entry_bar,
                exit_bar,
                hold_bars,
                qty: units_before.abs() - units_after.abs(), // 本次减掉的绝对手数
                long: units_before > 0.0,
                forced_close: forced,
            });
        }
    }
    // 记新入场：after≠0 且 (纯开仓 before=0 / 翻转后新方向) ⟹ 此 bar 为新仓入场 bar。
    if units_after != 0.0 && (units_before == 0.0 || closed) {
        *pos_entry_bar = Some(exit_bar);
    }
}

/// ★#90（B-M3b-expand）：TW 账本线程——fill loop 内 ~10 处分散的 TwState 接线
/// （init / ②' 成本划转 shadow / ②'' 已实现入账 shadow / 腿计数守卫 / stage 推进）
/// 收敛为单一状态对象。语义与收敛前逐字等价（守卫条件内化到方法）。
pub(super) struct TwLedgerThread {
    /// TW 账本真值（#124 裁定4：生产 π 单一真值源；P2/P3/P4 谓词 + tw_final 物证消费）。
    pub(super) tw: TwState,
    /// κ 策略（A10 附则A 优先序解析结果；η_bucket/enter_ready 判据消费）。
    pub(super) policy: RiskPolicy,
    /// TW 侧已见的真实成本基（i64 shadow；方向差分派 ShortDiff 划转，入账量 cash-sound 钳制）。
    tw_seen_basis: i64,
    /// A'（裁定清单③）：费后已实现 PnL 累计（f64 真值，仅实际平仓 fill 累加；forced_pnl 不入）。
    realized_cum: f64,
    /// TW 侧已入账的量化累计 shadow（对累计值量化再派差分 ⟹ 截断误差有界不累积）。
    tw_seen_realized: i64,
}

impl TwLedgerThread {
    /// 注资口径 = funded_campaign 同款：整窗 = 一个 campaign，投入 = 初始 NAV 取整
    /// （free = notional_in = ⌊nav0⌋，notional_in 下限 1）。
    pub(super) fn new(nav0: f64, policy: RiskPolicy) -> Self {
        Self {
            tw: TwState {
                free: nav0 as i64,
                notional_in: (nav0 as i64).max(1),
                ..TwState::initial()
            },
            policy,
            tw_seen_basis: 0,
            realized_cum: 0.0,
            tw_seen_realized: 0,
        }
    }

    /// A'（清单③）：累加本 fill 的费后已实现 PnL（多 fill 同 bar 聚合——推导链第 9 条）。
    pub(super) fn add_realized(&mut self, realized: f64) {
        self.realized_cum += realized;
    }

    /// ②' TW 成本划转（#124）：真实成本基（|units|·均价，空头取绝对额=在险市值）方向差分。
    pub(super) fn sync_basis(&mut self, units: f64, entry_cost: f64) {
        self.sync_basis_raw((units.abs() * entry_cost.abs()) as i64);
    }

    /// ②' 双账本推广（#68②）：调用方自算成本基（q⁺·cost⁺ + q⁻·cost⁻）后走同一差分派发。
    pub(super) fn sync_basis_raw(&mut self, basis_now: i64) {
        let d = basis_now - self.tw_seen_basis;
        self.tw_seen_basis = basis_now;
        if d > 0 {
            let inflow = d.min(self.tw.free); // 买入 free→holding，不透支 free
            if inflow > 0 {
                self.tw = tw_step(&self.tw, TwEvent::ShortDiff(-inflow));
            }
        } else if d < 0 {
            let outflow = (-d).min(self.tw.holding); // 卖出 holding→free，成本基回流
            if outflow > 0 {
                self.tw = tw_step(&self.tw, TwEvent::ShortDiff(outflow));
            }
        }
    }

    /// ②'' TW 已实现利润入账（A' 清单③）：量化差分 ⟹ `Realize(d_pi)` 入 free（可正可负）。
    pub(super) fn sync_realized(&mut self) {
        let realized_now = self.realized_cum as i64;
        let d_pi = realized_now - self.tw_seen_realized;
        if d_pi != 0 {
            self.tw_seen_realized = realized_now;
            self.tw = tw_step(&self.tw, TwEvent::Realize(d_pi));
        }
    }

    /// TW 腿计数（#124）：legacy ReverseOpen 腿开仓（OQ-9 守卫 `is_legal_from` 内化——
    /// EarningShares 阶段开 legacy 腿 PDF 定义为非法 ⟹ 不计，合法性语义非掩盖）。
    pub(super) fn open_share_leg(&mut self) {
        if TwEvent::OpenShareLeg.is_legal_from(&self.tw) {
            self.tw = tw_step(&self.tw, TwEvent::OpenShareLeg);
        }
    }

    /// TW 腿计数关侧（#124）：`legs>=1` 守卫与开侧 OQ-9 对称跳过。CloseShareLeg(0)
    /// 口径：净额架构无腿级损益分账 ⟹ profit 口径量 0 承载，非簿记伪造。
    pub(super) fn close_share_leg(&mut self) {
        if self.tw.open_legacy_legs >= 1 {
            self.tw = tw_step(&self.tw, TwEvent::CloseShareLeg(0));
        }
    }

    /// P3/P4 TWEvent_t（#124 裁定4）：stage 推进单点（stage_progression 派生事件生产恒合法）。
    pub(super) fn on_stage_event(&mut self, ev: TwEvent) {
        debug_assert!(
            ev.is_legal_from(&self.tw),
            "stage_progression 派生事件恒合法（OQ-9 生产不变量）"
        );
        self.tw = tw_step(&self.tw, ev);
    }

    /// 终态快照（tw_final 物证；快照取强平前，强平 PnL 不入 TW）。
    pub(super) fn finish(self) -> TwState {
        self.tw
    }
}
