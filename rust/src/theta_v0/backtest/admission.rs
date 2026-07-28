//! ★准入门 seam（B-M2，#89）：χ/nest/k_Θ 三门 + κ 解析
//!（设计 chanlun/plans/runner-rs-seam-designs-20260721.md §M2）。
//!
//! **溯源订正（#439，2026-07-27）**：本文件由 commit ec728bf6cf（#112 进场门消费
//! 多级投影）新增；`NestChainGate` / `admit` / `typed_lookup` 一整套业务逻辑
//! （约 900+ 行）在该提交之前的任何可达历史中都不存在，是**净新增代码**，并非
//! 自 runner.rs 移动——旧版「纯移动」表述有误。对照同一 commit 的 `fill.rs`
//! 才是真实纯移动（runner.rs diff 有对应删除行佐证）；本模块对应的 runner.rs
//! 改动仅为 import/`pub use` 调整。
//!
//! 三道门共用一个 seam 语义——「候选/组合 → 放行或收窄 𝒦_Θ」。κ 优先序
//!（env > config > baseline，A10 附则A 裁定接口冻结）藏在本模块内，fill loop
//! 不再 import env 细节。对外类型/函数经 `runner` 门面 `pub use` 保持原路径。

use super::super::classifier;
use super::super::config::ThetaConfig;
use super::super::strategy::ledger::RiskPolicy;
use super::super::types::Bar;
use super::ledger::LedgerOpen;

#[cfg(test)]
thread_local! {
    /// 测试注入点：Some(v) ⟹ **本线程**的 VOICE_EXEC gate 返回 v（优先于进程级 env）。
    /// 进程级 env 会被并行测试同时读到（OPSEM_DUMP_DIR 2026-07-13 竞态实录同型风险）；
    /// 线程局部对并行测试不可见。
    pub(super) static VOICE_EXEC_OVERRIDE: std::cell::Cell<Option<bool>> = std::cell::Cell::new(None);
}

/// ★W1 env gate：`VOICE_EXEC=1` ⟹ 声部独立执行臂（churn 修复）；未设/非"1" ⟹ 净额臂
/// （bit-exact 回归锁）。测试经线程局部 [`VOICE_EXEC_OVERRIDE`] 注入（生产构建不含该分支，
/// OPSEM_DUMP_DIR_OVERRIDE 同惯例）。
pub(super) fn voice_exec_gate() -> bool {
    #[cfg(test)]
    {
        if let Some(v) = VOICE_EXEC_OVERRIDE.with(|c| c.get()) {
            return v;
        }
    }
    std::env::var("VOICE_EXEC").ok().as_deref() == Some("1")
}

#[cfg(test)]
thread_local! {
    /// 测试注入点：Some(v) ⟹ **本线程**的 nest-gate 返回 v（优先于进程级 env）。
    /// 线程局部对并行测试不可见（VOICE_EXEC_OVERRIDE 同惯例，防 OPSEM_DUMP_DIR 式竞态）。
    pub(super) static NEST_CERT_GATE_OVERRIDE: std::cell::Cell<Option<bool>> = std::cell::Cell::new(None);
}

/// ★nest-gate（进出场区间套证书门，实装报告 nest-exit-gate-impl-20260719）env gate：
/// `THETA_NEST_CERT_GATE=1` ⟹ ①π 开仓准入门：T3 (#172) 起 **严格链判定为唯一 nest 判定源**
/// （[`NestChainGate::chain_lookup`]，恰好存在层间联动 + 逐级 N^δ 证书闭合到 L0）；L2 旧臂
/// [`super::econ_positive::build_gate_certificate`] 双读落账作对照——**typed 无证时回退旧臂
/// Xzd 通道**（Xzd 无 typed 对应物，经 `build_xzd_fallback` 单一来源 econ_positive.rs:1606：
/// 旧臂已落 Xzd ⟹ 直接复用其判定，旧臂产 L2 nest 证 ⟹ 重走单一来源补评）；②v1/双账退出循环的
/// 反向项 χ^{σ_p} 消费对象从裸 BspBits 升格为**同一套同点递归链确认**（T5b (#208) 迁链
/// 终态——出场不独立设计，ADR 20260723 裁定 3；旧 v1/dual 注入形态已由 #499 退役）。
/// 未设/非"1" ⟹ π 门整体跳过，全部路径逐字节不变（bit-exact 回归锁）。
/// 测试经线程局部 [`NEST_CERT_GATE_OVERRIDE`] 注入（VOICE_EXEC 同惯例）。
pub(super) fn nest_cert_gate_enabled() -> bool {
    #[cfg(test)]
    {
        if let Some(v) = NEST_CERT_GATE_OVERRIDE.with(|c| c.get()) {
            return v;
        }
    }
    std::env::var("THETA_NEST_CERT_GATE").ok().as_deref() == Some("1")
}

/// T3 (#172) 并门（#168 裁定 3）：**层载由链路径是否启用单一驱动**——链活（nest 证书门开）
/// ⟹ 投影层必载（含锚索引，恰好存在物化基座，索引成本即链判成本一部分）；链死 ⟹
/// 不载（零拷贝借用，零开销红线不死，#110 纪律）。层门配置面退役转派生：本函数是生产
/// 唯一派生点（π 入口消费）；classifier stamping 仍读 config 机制位（classifier 不可读
/// backtest 门 env——层次纪律）。八处 `level_projection: None` 构造点不动（链不达其境）。
pub(super) fn chain_driven_level_projection(
    config: &ThetaConfig,
) -> std::borrow::Cow<'_, ThetaConfig> {
    if nest_cert_gate_enabled() && !config.level_projection.enabled {
        let mut derived = config.clone();
        derived.level_projection =
            super::super::classifier::projection::LevelProjectionConfig::for_chain(true);
        std::borrow::Cow::Owned(derived)
    } else {
        std::borrow::Cow::Borrowed(config)
    }
}

/// ★nest-gate 观测统计（升格路径 b 实装卡 §3.4-3「拒绝率实测落账」）：门开启时逐候选落账，
/// 门关闭恒零不输出。通道标签 = [`nest_gate_admit`] 第二返回值。
/// ★#75 扩展：L2 旧臂对照差 + Xzd 回退计数。
/// ★#94：复用通道（typed 无证 ∧ 旧臂 Xzd 复用，admit≡old_admit by construction）自证成分
/// 不入对照差，单独计 `cross_reuse`（输出分工：STATS=准入判定分账，CHAIN=回退/对照）。
/// ★T4 (#173)：并集/single comparison shadow 列（typed_found/typed_none/admit_rungs/
/// rej_rungs/typed_hits_by_level/single_multi_divergence）随旧桥退役删除。
#[derive(Debug, Default)]
pub(super) struct NestGateStats {
    pub(super) total: usize,
    pub(super) admitted: usize,
    pub(super) nest_pass: usize,
    pub(super) xzd_pass: usize,
    pub(super) rej_flat_dir: usize,
    pub(super) rej_no_level: usize,
    pub(super) rej_cert_none: usize,
    pub(super) rej_nest_n_delta: usize,
    pub(super) rej_xzd_gate: usize,
    pub(super) xzd_fallback: usize,
    /// L2 旧臂对照（双读落账）：两路一致 / 旧准新拒 / 旧拒新准——仅含两路**独立**判定案例。
    pub(super) cross_agree: usize,
    pub(super) cross_old_pass_new_rej: usize,
    pub(super) cross_old_rej_new_pass: usize,
    /// 复用通道单独记账（#94）：typed 无证 ∧ 旧臂已落 Xzd ⟹ admit≡old_admit by construction，
    /// 「一致」是自证成分——**不计入** agree/对照差三列，单列呈现。
    pub(super) cross_reuse: usize,
    // ── T3 (#172) 链判定读数（NEST_GATE_T3 行消费；判定唯一源 = 链）──
    /// 链全闭合通过（= nest_pass 通道计数，方向守卫读数：链确认总数 vs 并集 17 基线）。
    pub(super) chain_pass: usize,
    /// 链缺/断拒（= nest_n_delta_false 通道中链拒部分）。
    pub(super) chain_reject: usize,
    /// 缺环拒（首位归因 = 缺）。
    pub(super) chain_reject_missing: usize,
    /// 断环拒（首位归因 = 断）。
    pub(super) chain_reject_broken: usize,
    /// 链 NoChain（→ Xzd 回退通道）。
    pub(super) chain_none: usize,
    /// 链顶级别分布（有链顶的候选：Pass/Reject/NoChain 全计——链即身份读数）。
    pub(super) chain_top_dist: std::collections::BTreeMap<u32, usize>,
    /// π 单门双覆盖的出场归因子总体：χ 过滤后的门前候选中，方向性候选与 `prev_active`
    /// 存在同级反向活动腿的总数。`prev_active` 按定义只含未平仓腿。
    ///（严格超集口径见 [`exit_candidate_would_close`] 文档——不含 fold 证书门与腿占用状态。）
    pub(super) exit_cand_total: usize,
    /// 上述子总体中被门放行的候选数；拒绝数由 `total - admitted` 导出。
    pub(super) exit_cand_admitted: usize,
    // T5a (#207)：方向见证统计字段（chain_dir_witness_divergence）随方向退役删除——
    // 「方向分歧」在 ADR 20260723 裁定 1 下是非概念（同点跨型 = 各级自为真，非分歧）。
}

impl NestGateStats {
    pub(super) fn observe(&mut self, admit: bool, channel: &'static str, obs: NestGateObs) {
        self.total += 1;
        if admit {
            self.admitted += 1;
        }
        match channel {
            "nest_pass" => self.nest_pass += 1,
            "xzd_pass" => self.xzd_pass += 1,
            "flat_dir" => self.rej_flat_dir += 1,
            "no_level" => self.rej_no_level += 1,
            "cert_none" => self.rej_cert_none += 1,
            "nest_n_delta_false" => self.rej_nest_n_delta += 1,
            "xzd_gate_fail" => self.rej_xzd_gate += 1,
            _ => unreachable!("nest_gate_admit 通道标签闭集"),
        }
        if obs.xzd_fallback {
            self.xzd_fallback += 1;
        }
        // #94：复用通道（admit≡old_admit by construction）单独记账，自证成分不入对照差。
        if obs.reused_old_xzd {
            self.cross_reuse += 1;
        } else {
            match (obs.old_admit, admit) {
                (true, true) | (false, false) => self.cross_agree += 1,
                (true, false) => self.cross_old_pass_new_rej += 1,
                (false, true) => self.cross_old_rej_new_pass += 1,
            }
        }
        // ── T3 (#172) 链读数（判定唯一源落账；NEST_GATE_T3 行消费）──
        // flat_dir/no_level 早退候选未进入链求值（obs 为 default）——不计链三态，防污染
        // NoChain 语义（NoChain = 实际经链解析而查无；基线窗 flat/no_level=0，红线无差）。
        if !matches!(channel, "flat_dir" | "no_level") {
            match obs.chain_verdict {
                ChainVerdict::Pass => self.chain_pass += 1,
                ChainVerdict::Reject => {
                    self.chain_reject += 1;
                    match obs.chain_first_gap.map(|(_, kind)| kind) {
                        Some(ChainGapKind::Missing) => self.chain_reject_missing += 1,
                        Some(ChainGapKind::Broken) => self.chain_reject_broken += 1,
                        None => {}
                    }
                }
                ChainVerdict::NoChain => self.chain_none += 1,
            }
            if let Some(top) = obs.chain_top {
                *self.chain_top_dist.entry(top).or_default() += 1;
            }
        }
    }

    /// π 出场归因只读落账：只消费调用点已经读出的方向/活动腿匹配结果，不参与准入判定。
    pub(super) fn observe_exit_candidate(&mut self, admit: bool, would_close: bool) {
        if !would_close {
            return;
        }
        self.exit_cand_total += 1;
        if admit {
            self.exit_cand_admitted += 1;
        }
    }

    pub(super) fn exit_cand_rejected(&self) -> usize {
        self.exit_cand_total - self.exit_cand_admitted
    }

    /// π 可达臂的出场归因诊断行。独立命名，避免与 deprecated v1/dual
    /// 决策注入门的 `NEST_GATE_EXIT` 行混为同一统计域。
    pub(super) fn exit_cand_report_line(&self) -> String {
        format!(
            "NEST_GATE_EXIT_CAND total={} admitted={} rejected={}",
            self.exit_cand_total,
            self.exit_cand_admitted,
            self.exit_cand_rejected(),
        )
    }
}

/// π 候选过滤门的出场归因读出：方向性候选若命中同级反向活动腿，则它满足 fold 规则2
/// 平仓消费的级别/方向前置条件。本读出**不含** fold 的 `nest_confirmed` 证书门与 fold 内
/// 腿占用状态，故为真实消费域的**超集**：已知偏宽源 = (a) `struct_break_dir` 零 bit 回退
/// 候选（`nest_confirmed=false`，fold 归 record）、(b) 同 bar 多候选竞争同一腿的重复计数
///（#510 影子评审实测 60000 bar 约 4.4%）。方向性候选经 `root_sel` 蕴含
/// `nest_confirmed=true ∧ bsp_class≠MAX`，故证书门本身不构成偏差源。
/// 这里只读 `(level, dir)` 与 `prev_active`，不复制 fold 平仓谓词、不消费候选，
/// 也不参与 [`NestChainGate::admit`] 的判定。读出口径 = χ 过滤后的门前候选。
pub(super) fn exit_candidate_would_close(
    candidate_level: u32,
    candidate_dir: super::super::strategy::voice::VoiceSide,
    prev_active: &[super::super::strategy::interp::ActiveLeg],
) -> bool {
    use super::super::strategy::voice::VoiceSide;

    match candidate_dir {
        VoiceSide::Long | VoiceSide::Short => prev_active.iter().any(|leg| {
            leg.level == candidate_level
                && matches!(
                    (candidate_dir, leg.dir),
                    (VoiceSide::Long, VoiceSide::Short) | (VoiceSide::Short, VoiceSide::Long)
                )
        }),
        VoiceSide::Flat => false,
    }
}

/// ★nest-gate 单候选 L2 读出（升格路径 b，实装卡 §3.2）：开仓候选的 N^δ/Xzd 凭据读出。
///
/// 门源 = [`super::econ_positive::build_gate_certificate`]（econ_positive.rs:1572）——与 econ
/// 二通道准入门（econ_positive.rs:364-374）**同函数同判据**，不产生第二裁决源（090/no-patch）；
/// 通过读出（Nest→`n_delta` / Xzd→`gate_pass` / None→拒）逐字镜像 econ_positive.rs:367-371。
/// `confirm_index` = 当前 bar `i`（与 econ 侧 `i` 同口径，因果）；`bsp_of_level`/次级视图取自
/// **前缀因果分类**（classification_i，≤i 数据），与 econ 侧 `cls_i.levels` 同口径。
///
/// ★#75（N3-T2 真链切换）：typed 真链为唯一 nest 判定源（[`NestChainGate::admit`]）。
/// 本函数双读落账供对照差（cross 计数）；typed 无证且本读出已落 Xzd 通道时，其 Xzd 判定
/// 被**复用为门判定**（Xzd 无 typed 对应物；本函数 Xzd 通道与 `build_xzd_fallback` 同一
/// 单一来源，econ_positive.rs:1606，复用即逐值一致——该复用案例 admit≡old_admit
/// by construction，cross 对照差剔除之，见 `NestGateStats::cross_reuse`）。
/// 返回 `(放行, 通道标签)`；标签供 [`NestGateStats`] 落账（拒绝率实测分通道归因）。
pub(super) fn nest_gate_admit(
    tower: &[std::rc::Rc<Vec<classifier::recursive_tower::LeveledMove>>],
    c: &super::super::strategy::interp::Candidate,
    hist: &[f64],
    confirm_index: usize,
    classification: &classifier::Classification,
) -> (bool, &'static str) {
    use super::super::strategy::voice::VoiceSide;
    use super::super::types::Side;
    let delta = match c.dir {
        VoiceSide::Long => Side::Long,
        VoiceSide::Short => Side::Short,
        VoiceSide::Flat => return (false, "flat_dir"), // 与 nest_confirm（interp.rs:382）同语义
    };
    let lvl = c.level as usize;
    let Some(ls) = classification.levels.get(lvl) else {
        return (false, "no_level");
    };
    let (sub_centers, sub_bsp) = if lvl > 0 {
        match classification.levels.get(lvl - 1) {
            Some(sub) => (sub.centers.as_slice(), sub.bsp.as_slice()),
            None => (&[][..], &[][..]),
        }
    } else {
        (&[][..], &[][..])
    };
    match super::econ_positive::build_gate_certificate(
        tower,
        lvl,
        c.source_index,
        delta,
        &c.bits,
        hist,
        confirm_index,
        &ls.bsp,
        sub_centers,
        sub_bsp,
    ) {
        Some(super::econ_positive::GateCertificate::Nest(cert)) => {
            let pass = cert.n_delta();
            (pass, if pass { "nest_pass" } else { "nest_n_delta_false" })
        }
        Some(super::econ_positive::GateCertificate::Xzd(ev)) => {
            let pass = ev.gate_pass();
            (pass, if pass { "xzd_pass" } else { "xzd_gate_fail" })
        }
        None => (false, "cert_none"),
    }
}

// ═════════════════ #75（SPEC #73 A 线第二票，N3-T2 进场门真链切换）═════════════════
//
// 门开路径（THETA_NEST_CERT_GATE=1）改消费 #74 的 typed 真链证书索引
//（`classifier::nest_index::build_nest_certificate_index`）：N^δ 跨级递归链判定
//（`cert.certificate().n_delta()`，nest.rs 递归核单一来源）替代 L2 点包含读出
//（econ_positive::build_nest_certificate）。L2 旧臂保留为对照读出双读落账——typed 命中时
// 不参与 nest 判定（唯一源 = typed 真链）；typed 无证时回退旧臂 Xzd 通道（Xzd 无 typed
// 对应物，经 build_xzd_fallback 单一来源 econ_positive.rs:1606）。
//
// ## 身份桥（候选 → 证书基例事件，先确证后实装）
//
// - **坐标系同一性（代码证据）**：候选 `BspPoint.source_index` = L0 原始 K 序位置
//   （bsp.rs:114-115；一类点 = C 破中枢段端点 signal.rs:399 `make_first_point(end.source_index)`）；
//   证书事件坐标同域——trend `turn_source` = t*（段端点扫描，level_view.rs:773-776）、
//   pan `turn_source` = `structure.source_index` = C 段端点（level_view.rs:863 +
//   signal.rs:701 `end.source_index`）、`interval_b` 同为段坐标。生产交叉证据：
//   `terminal_bits_in_book`（nest.rs:607-628）在同一窗口表达式内比较
//   `BspPoint.source_index` 与事件 `interval_b`/`turn_source`（p117 生产查法，task-101
//   实测 91/91 绑定、median |dt|=0）；L2 旧臂 `find_move_by_end_index(exec_moves, source_index)`
//   （econ_positive.rs:899-900）同域匹配塔段端点。⟹ 同一 L0 原始 K 序坐标系，确证成立。
// - **级别桥**：T4 (#173) 起进场侧 fixed ℓ=lvl+1 comparison 与 multi 扫描桥均退役
//   （判定唯一源 = T3 严格链）。T1 移位 `event_bsp_book_level(ℓ)=ℓ-1`
//   仍定义级别-ℓ 事件的终端背书账本。
// - **身份判据 = 同点递归（T5a #207 / ADR 20260723 裁定 1）**：候选 → 证书的桥接
//   不再是固定 ℓ+1 键查，而是两元锚（极值价, 组锚）在 [L0, 链顶] 逐级键域查询
//   （`by_triple_anchor`）+ 恰好存在层间联动（`chain_lookup` 文档）。
// - **多证书合并（T7 裁定：布尔 + 最强档）**：同级多证书任一 `n_delta` 通过即准，
//   链深取最深一张（仅归因，不线性放大）。
// - **T5b (#208) 旧出场桥删除账**：出场侧固定 ℓ+1 无因果守卫桥四件
//   （`typed_lookup`/`by_end`/`has_bridge_key`/`NestCandidateEventExt.seg_c_full`）
//   随 #499 退役删除（#206 Q3 判删——拍脑袋临时物）。
//   旧值桥证否史（turn_source 等值桥必漏、seg_c_full 为收束前全段权威坐标）见 #75。

/// 真链门观测（`NestGateStats::observe` 第三入参）：T3 (#172) 起 **链结果为唯一 nest
/// 判定源**（`chain_*` 字段）；L2 旧臂读出为对照格（只落账不消费）。
/// T4 (#173)：single/multi comparison 格（typed_rungs/typed_hit_levels/single_admit/
/// multi_admit）与并集 shadow 读出已随旧桥单/multi 查询删除（进场侧）。
/// T5b (#208)：出场侧 `typed_lookup` 与旧 v1/dual 注入形态均已删除。
#[derive(Debug, Default, Clone)]
pub(super) struct NestGateObs {
    /// L2 旧臂判定（对照读出；链 NoChain ∧ 旧臂已落 Xzd 通道时被复用为门判定——
    /// 该复用案例的「两路一致」是自证，单独计 cross_reuse 不入对照差，见 reused_old_xzd）。
    pub(super) old_admit: bool,
    /// L2 旧臂通道标签（`nest_gate_admit` 第二返回值；早退候选 = ""）。
    pub(super) old_channel: &'static str,
    /// 链 NoChain ∧ L2 有证 ⟹ 经 `build_xzd_fallback` 走 Xzd 回退（语义变更集，对照可见）。
    pub(super) xzd_fallback: bool,
    /// 链 NoChain ∧ 旧臂已落 Xzd 通道 ⟹ 复用旧臂 Xzd 读出为门判定（Xzd 无 typed 对应物，
    /// 旧臂该通道与 `build_xzd_fallback` 同一单一来源）；admit≡old_admit by construction。
    pub(super) reused_old_xzd: bool,
    // ── T3 (#172) 链字段（唯一 nest 判定源；谱系全落账）──
    /// 链裁决三态（Pass → nest_pass；Reject → nest_n_delta_false；NoChain → Xzd 回退）。
    pub(super) chain_verdict: ChainVerdict,
    /// 链顶账本级（恰好存在经 T2 层间联动给出）。
    pub(super) chain_top: Option<u32>,
    /// 自链顶向下连续闭合到哪级（Pass ⟹ Some(0) = 闭合到 L0）。
    pub(super) chain_closed_down_to: Option<u32>,
    /// 首位归因：最高非闭合级 + 缺/断位置极性。
    pub(super) chain_first_gap: Option<(u32, ChainGapKind)>,
    /// 完整链谱系（逐级闭合情况，[L0, 链顶] 升序）。
    pub(super) chain_genealogy: Vec<ChainLevelGenealogy>,
    /// 极值价（T1 供给线；None = x 处无 confirmed 分型）。
    pub(super) chain_price: Option<super::super::types::Tick>,
    /// 组锚 a*（T2 本级层解析；None = 本级层未载 / 该脚未登记）。
    pub(super) chain_anchor: Option<usize>,
}

// ═════════════════ T3（#172 严格链判定，#163 裁定 1/3 + #168 裁定 1/3/4 执行）
// ═════════════════ + T5a（#207 方向退役，ADR 20260723 裁定 1 执行）═════════════════
//
// 语义 pin（依据 #163 裁定 1 字面 + 六术语词汇 + ADR 20260723 裁定 1，设计决定与理由随类型注释落账）：
//
// - **身份判据 = 同点递归；方向整体退役出身份层**（T5a，ADR 20260723 裁定 1）：跨级确认是
//   递归区间套——分型在各级递归落到同一个点即为同一事件，落不上 = 递归失败。方向（买/卖
//   标签）在身份判定中**完全多余**：同一点可在不同级别分别为顶/底分型（如反向收束：L2 底、
//   L1 顶），各级分型类型是各级自己的结构事实，都可为真；买卖标签属交易层（`event.side`/
//   VoiceSide/入场裁决/Xzd 回退保留不动），与身份层解耦。身份锚自三元组（方向, 极值价,
//   合并组锚）简化为**两元（极值价, 合并组锚）**；同价双脚由合并组区分；跨型共点不产生
//   价格二义（T1 探针已证：价格与组锚同锚于 x 处 L0 分型——方向位在键中本就零工作）。
// - **级别坐标**：链级 = BSP 账本级（book level，`Classification.levels` 索引）；证书键级 =
//   账本级 + 1（`event_bsp_book_level` T1 移位——nest level-ℓ 事件的拐点账本 = levels[ℓ-1]，
//   与旧固定桥 ℓ=lvl+1 同移位）。
// - **链区间 = [L0, 链顶]**，对全候选一致（「闭合到 L0」字面）：候选本级是**普通链级**
//   （链即身份——级别归属由链谱系给出，不靠候选自报 lvl 猜方向）；砍掉本级会丢失旧单桥
//   覆盖总体（typed_found 基线），且本级恰是旧固定桥唯一查询级，必须被链内含。
// - **链顶** = x 为拐点（**不问分型类型**——顶/底皆可，T5a 起方向/类型不进存在性判据）的
//   最高账本级，由**恰好存在**经 T2 层索引层间联动给出（非塔顶移动窗口、非固定 ℓ+1）：
//   逐级查层 `cross_level_query(极值价)`，脚身份 = 组锚 a* 匹配。极值价经 T1 供给线
//   `fractal_at_source` 解析（gate 持有同一分型账本 Rc，单一来源）；a* 自**本级层**
//   `source_index == c.source_index` 条目解析（禁第二查法：gate 不自行调
//   `merged_group_anchor` 另起锚解析）。
// - **闭合** = 该级键域 `(事件级, 极值价, a*)` 有身份集，≥1 张证书过因果守卫
//   （链上全部 `judge_at ≤ anchor`，越界整证剔除——现守卫语义逐字），且级内 T7 合并
//   `n_delta` 过（判定谓词唯一来源 `cert.certificate().n_delta()`，nest.rs 递归核，禁第二查法）。
// - **缺/断极性**（词汇表：缺环即拒 = lvl+1 单级过证不算；断环即拒 = 高级有证而中间断不算）：
//   非闭合级 g 的上方区间 (g, 链顶] 内有闭合级 ⟹ **断**（高级有证而中间断），否则 **缺**。
//   底质三态（缺）：T2 层无该脚存在 / 键域查无身份或索引无证 / 有证但全被因果守卫剔除。
//   （断底质「有因果干净证书但合并 n_delta 假」在现装配下结构性不可达——
//   `assemble_typed_certificate` 只产过证（nest.rs:683 debug_assert）；保留语义位，
//   守卫/n_delta 消费逐字，伪证一旦出现即落 [`ChainLevelStatus::Broken`]。）
//   T5a 复核：缺/断定义按新语义**逐字保留**——极性判据（上方有闭合 ⟹ 断）与底质三态
//   均不涉及方向分量；唯一变化 = 存在性与键域查询不再滤方向（同点跨型回归为合法递归）。
// - **三态裁决**：全链闭合 = Pass（`nest_pass`）；≥1 闭合级但有缺/断 = Reject
//   （`nest_n_delta_false`，对标旧 `Some(pass=false)` 语义位）；零闭合级 / 存在性全无 /
//   锚不可解 = **NoChain**（Xzd 回退通道逐字不动——含「唯一证书全被因果守卫剔除」的
//   #112-T2 现语义：零闭合 ⟹ NoChain ⟹ 回退）。

/// T3 链级状态（链谱系逐级落账的底质）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ChainLevelStatus {
    /// 闭合：≥1 因果干净证书，级内 T7 合并 `n_delta` 过。
    Closed,
    /// 断（底质）：≥1 因果干净证书但合并 `n_delta` 假（现装配结构性不可达，见上注）。
    Broken,
    /// 缺（底质）：T2 层无该脚存在（恰好存在在该级断裂）。
    MissingExistence,
    /// 缺（底质）：有存在但键域查无身份 / 索引无证。
    MissingCert,
    /// 缺（底质）：键域有证但全被因果守卫剔除（整证剔除 ⟹ 不算「有证」，归缺不归断）。
    MissingCausal,
}

/// T3 缺/断位置极性（词汇表判据：上方有闭合 ⟹ 断，否则缺）。
/// T5a (#207) 复核：极性判据不涉及方向分量，新语义下逐字保留。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ChainGapKind {
    /// 缺环：上方区间 (g, 链顶] 无闭合级（lvl+1 单级过证不算同型）。
    Missing,
    /// 断环：上方区间 (g, 链顶] 内有闭合级（高级有证而中间断）。
    Broken,
}

/// T3 链谱系单级条目（逐级闭合情况全落账，供归因与级别身份判定——链即身份）。
/// T5a (#207)：方向见证字段（dir_witness/DirWitness）随方向退役删除——「方向分歧」
/// 在 ADR 20260723 裁定 1 下是非概念（同点跨型 = 各级自为真，469 例 A/B/C 分类账
/// 中 A 类 235 例为语义真相、B/C 类为同价双脚，均非分歧）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ChainLevelGenealogy {
    /// BSP 账本级（book level）。
    pub(super) level: u32,
    /// 证书键级 = 账本级 + 1（T1 移位）。
    pub(super) event_level: u32,
    /// 底质状态。
    pub(super) status: ChainLevelStatus,
    /// 缺/断位置极性（非闭合级时填）。
    pub(super) gap: Option<ChainGapKind>,
    /// 键域身份数（因果守卫剔除前）。
    pub(super) n_certs: usize,
    /// 过因果守卫证书数。
    pub(super) n_causal_clean: usize,
    /// 级内最深链深（仅归因，不线性放大）。
    pub(super) rungs: usize,
}

/// T3 链裁决三态（admit 消费位）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum ChainVerdict {
    /// 全链闭合通过。
    Pass,
    /// ≥1 闭合级但有缺/断（含谱系）。
    Reject,
    /// 零闭合级 / 存在性全无 / 锚不可解（x 在任何键域都查无）——Xzd 回退通道。
    #[default]
    NoChain,
}

/// T3 严格链查询结果：三态 + 完整链谱系（链顶级别、逐级闭合情况、连续闭合到哪级）。
#[derive(Debug, Clone, Default)]
pub(super) struct ChainProbe {
    pub(super) verdict: ChainVerdict,
    /// 极值价（T1 供给线解析；None = x 处无 confirmed 分型）。
    pub(super) price: Option<super::super::types::Tick>,
    /// 组锚 a*（T2 本级层解析；None = 本级层未载 / 该脚未登记）。
    pub(super) anchor: Option<usize>,
    /// 链顶账本级（恰好存在给出；None = 任何级都无该脚存在）。
    pub(super) chain_top: Option<u32>,
    /// 自链顶向下连续闭合到哪级（Pass ⟹ Some(0) = 闭合到 L0）。
    pub(super) closed_down_to: Option<u32>,
    /// 首位归因：最高非闭合级 + 缺/断极性（全闭合 / NoChain-无区间时 None）。
    pub(super) first_gap: Option<(u32, ChainGapKind)>,
    /// 逐级谱系（按账本级升序，[L0, 链顶]）。
    pub(super) levels: Vec<ChainLevelGenealogy>,
}

/// #93 步骤 0：值指纹（替代 Rc ptr_eq）。存水线 + 尾段副本，不持有 Rc——
pub(super) struct LevelFingerprint {
    w_self: usize,
    w_lower: usize,
    /// tower[ℓ][w_self..] 的副本（O(未确认尾段) = O(1) 摊还）。
    tail_self: Vec<classifier::recursive_tower::LeveledMove>,
    /// tower[ℓ-1][w_lower..] 的副本。
    tail_lower: Vec<classifier::recursive_tower::LeveledMove>,
}

impl LevelFingerprint {
    /// 内容不变判据：水线不退 + 尾段逐值相等。
    /// 论证（bit-exact 命门，§3.1 步骤 0 构造性论证）：
    /// - [..w] 证书保跨 bar bit-stable ⟹ 前缀免比对；
    /// - new_w >= old_w ⟹ [old_w..new_w] 在上 bar 属尾段（已存于 old_tail[..]），
    ///   本 bar 确认 ⟹ 证书保其不变 ⟹ 只需比 new_tower[old_w..] == old_tail。
    pub(super) fn content_unchanged(
        &self,
        new_w_self: usize,
        new_w_lower: usize,
        new_tower_self: &[classifier::recursive_tower::LeveledMove],
        new_tower_lower: &[classifier::recursive_tower::LeveledMove],
    ) -> bool {
        // 水线回退 ⟹ 保守全量（恒正确退化）。
        if new_w_self < self.w_self || new_w_lower < self.w_lower {
            return false;
        }
        // 从旧水线到末尾逐值比对（旧尾段 = old_tower[old_w..]；新塔[new_w..] ⊆ 新塔[old_w..]）。
        // 证书保 [..new_w] 稳定 ⟹ [old_w..new_w] 不变 ⟹ 只需比 [old_w..] 整段。
        &new_tower_self[self.w_self..] == self.tail_self.as_slice()
            && &new_tower_lower[self.w_lower..] == self.tail_lower.as_slice()
    }
}

pub(super) struct NestChainGate {
    pub(super) hist: Vec<f64>,
    pub(super) dif: Vec<f64>,
    pub(super) close_src: Vec<usize>,
    /// 下标 = nest 级别 ℓ 的 append-only 事件账本（`events_by_level[0]` 恒空，nest 不听 L0）。
    pub(super) events_by_level: Vec<Vec<classifier::level_view::NestCandidateEvent>>,
    pub(super) seen: std::collections::HashSet<classifier::nest::NestEventIdentity>,
    /// T1 (#170 键域重锚，expand 无门纯增写）→ T5a (#207 去方向位）：新单键域
    /// **(ℓ, 极值价, 组锚@ℓ) → 身份集**——键 = 不变量两元锚 + 级别（ADR 20260723 裁定 1：
    /// 身份判据 = 同点递归，方向整体退役出身份层；#163 裁定 2：合并组锚是该级包含层
    /// 事实，键天然分级，级别居键内不居身份内）。两元锚由 provider 在事件构造点携带
    /// （`NestCandidateEventExt.extreme_price`/`group_anchor`，禁第二查法）；极值价 =
    /// 整数 tick **精确等值无容差**（v3 硬禁令）。方向不进键：同点跨型共点不产生价格
    /// 二义（T1 探针已证，方向位在键中本就零工作）；`event.side` 仍存事件本体供交易层
    /// （入场裁决/Xzd 回退），不进身份键。读出 = T3 链判定（唯一消费者）。
    /// T5b (#208)：旧出场侧 `by_end` 值桥键域（固定 ℓ+1, seg_c_full.1, side）已删
    ///（#206 Q3 判删——出场迁链后无读者）。
    pub(super) by_triple_anchor: std::collections::HashMap<
        (u32, super::super::types::Tick, usize),
        Vec<classifier::nest::NestEventIdentity>,
    >,
    /// #218 面 B：事件身份 → 两元锚正查账本（`absorb_exts` 落账时与 `by_triple_anchor`
    /// 同源写入——`NestCandidateEventExt` 已带 `extreme_price`/`group_anchor`，零新增
    /// 解析、禁第二查法）；`build_nest_certificate_index` 二类判同的事件侧锚经此透传
    /// （spec owner-attribution-fix-20260724 ID-2 进核路径）。未载 = 事件侧锚供给缺失
    /// （锚不可解，诚实判负——与 `by_triple_anchor` 跳过同一前件）。
    pub(super) anchor_by_id: std::collections::HashMap<
        classifier::nest::NestEventIdentity,
        (super::super::types::Tick, usize),
    >,
    /// T1 (#170) 两元锚供给：分型管（L0 confirmed 分型账本，`parse_layer` Rc 共享 O(1)）。
    /// 极值价/方向的单一来源（provider 构造点查 `fractal_at_source`）。
    pub(super) fractals: std::rc::Rc<Vec<super::super::types::Fractal>>,
    /// T1 (#170) 两元锚供给：该级包含层（L0 merged_bars，`parse_layer` Rc 共享 O(1)）。
    /// 组锚的单一来源（provider 构造点查 `merged_group_anchor`）。
    pub(super) merged_bars: std::rc::Rc<Vec<super::super::types::Bar>>,
    /// T1 (#170) 锚供给未命中而跳过新键域登记的事件计数（诚实缺锚照实落账；
    /// 只进内存计数，不进任何打印行——NEST_GATE_STATS 决策面逐字节不变）。
    pub(super) n_anchor_misses: usize,
    /// 派生跳过指纹：每级 (w_self, w_lower, tail_self, tail_lower)——**不持有 Rc**
    ///（#93 步骤 0：消除指纹自溃——旧 ptr_eq 因 gate 持 Rc ⟹ classifier make_mut
    /// strong_count>1 ⟹ 每 bar COW 新分配 ⟹ ptr_eq 恒失效。改水线 + 尾段逐值比对：
    /// tower_confirmed_len 证书保 [..w] 跨 bar bit-stable ⟹ 只需比 [w..] 尾段）。
    pub(super) derived: Vec<Option<LevelFingerprint>>,
    /// 已结算冻结的 (level, run_start, run_end) 集合（#93 步骤 1：R1 confirmed 全 pair
    /// 已入账 ⟹ 重派生产出必被 dedup，零贡献；R3 pan 出生即定型）。
    pub(super) frozen_runs: std::collections::HashSet<(usize, usize, usize)>,
    pub(super) index: classifier::nest_index::NestCertificateIndex,
    /// 索引重建指纹：重建时的事件总数 + 各级 bsp 账本 Rc。
    pub(super) index_event_count: usize,
    pub(super) book_fps: Vec<std::rc::Rc<Vec<classifier::bsp::BspPoint>>>,
    pub(super) n_derivations: usize,
    pub(super) n_index_builds: usize,
    pub(super) n_provider_errors: usize,
    /// 派生产出的全部事件数（含未确认——诊断用，区分「零产出」与「全未确认」）。
    pub(super) n_events_seen: usize,
}

impl NestChainGate {
    /// 门开一次性构建（O(n)）：provider 域序列 = classify_impl 同构造（merged closes MACD +
    /// close_src 映射；因果前缀安全，见结构体文档）。
    pub(super) fn new(bars: &[super::super::types::Bar], config: &ThetaConfig) -> Self {
        let l0 = super::super::parser::parse_layer(bars, config);
        let closes: Vec<f64> = l0.merged_bars.iter().map(|b| b.close as f64).collect();
        let close_src: Vec<usize> = l0.merged_bars.iter().map(|b| b.source_index).collect();
        let series = classifier::divergence::compute_macd(&closes, &config.macd);
        NestChainGate {
            hist: series.hist,
            dif: series.dif,
            close_src,
            events_by_level: Vec::new(),
            seen: std::collections::HashSet::new(),
            by_triple_anchor: std::collections::HashMap::new(),
            anchor_by_id: std::collections::HashMap::new(),
            fractals: std::rc::Rc::clone(&l0.fractals),
            merged_bars: std::rc::Rc::clone(&l0.merged_bars),
            n_anchor_misses: 0,
            derived: Vec::new(),
            frozen_runs: std::collections::HashSet::new(),
            index: classifier::nest_index::NestCertificateIndex::default(),
            index_event_count: 0,
            book_fps: Vec::new(),
            n_derivations: 0,
            n_index_builds: 0,
            n_provider_errors: 0,
            n_events_seen: 0,
        }
    }

    #[cfg(test)]
    pub(super) fn for_test(hist: Vec<f64>, dif: Vec<f64>, close_src: Vec<usize>) -> Self {
        NestChainGate {
            hist,
            dif,
            close_src,
            events_by_level: Vec::new(),
            seen: std::collections::HashSet::new(),
            by_triple_anchor: std::collections::HashMap::new(),
            anchor_by_id: std::collections::HashMap::new(),
            fractals: std::rc::Rc::new(Vec::new()),
            merged_bars: std::rc::Rc::new(Vec::new()),
            n_anchor_misses: 0,
            derived: Vec::new(),
            frozen_runs: std::collections::HashSet::new(),
            index: classifier::nest_index::NestCertificateIndex::default(),
            index_event_count: 0,
            book_fps: Vec::new(),
            n_derivations: 0,
            n_index_builds: 0,
            n_provider_errors: 0,
            n_events_seen: 0,
        }
    }

    /// T3 (#172) 测试夹具：携两元锚供给的 `for_test`（链查询的极值价/组锚单一来源 =
    /// T1 供给线同一账本族，禁第二查法）。
    #[cfg(test)]
    pub(super) fn for_test_with_supplies(
        hist: Vec<f64>,
        dif: Vec<f64>,
        close_src: Vec<usize>,
        fractals: std::rc::Rc<Vec<super::super::types::Fractal>>,
        merged_bars: std::rc::Rc<Vec<super::super::types::Bar>>,
    ) -> Self {
        NestChainGate {
            fractals,
            merged_bars,
            ..Self::for_test(hist, dif, close_src)
        }
    }

    /// 每 bar 增量喂法：级别事件按需重派生（#93 值指纹跳过未变级），新确认事件 first-wins 追加。
    /// `confirmed_lens[ℓ]` = `classification.tower_confirmed_len(ℓ)`（parser/A3 证书链，禁第二查法）。
    pub(super) fn sync_events(
        &mut self,
        tower: &[std::rc::Rc<Vec<classifier::recursive_tower::LeveledMove>>],
        confirmed_lens: &[usize],
        as_of: usize,
    ) {
        let n_levels = tower.len();
        if n_levels > self.events_by_level.len() {
            self.events_by_level.resize_with(n_levels, Vec::new);
            self.derived.resize_with(n_levels, || None);
        }
        for level in 1..n_levels {
            let w_self = confirmed_lens.get(level).copied().unwrap_or(0);
            let w_lower = confirmed_lens.get(level - 1).copied().unwrap_or(0);
            let tower_self = tower[level].as_slice();
            let tower_lower = tower[level - 1].as_slice();
            // #93 步骤 0：值指纹判变（替代 Rc ptr_eq——消除自溃 + COW）。
            if let Some(fp) = &self.derived[level] {
                if fp.content_unchanged(w_self, w_lower, tower_self, tower_lower) {
                    continue; // 两级塔内容逐字节未变 ⟹ 事件集不变（构造性论证 §3.1 步骤 0）
                }
            }
            self.n_derivations += 1;
            let exts = self.derive_level_events(tower, level, as_of);
            self.absorb_exts(exts);
            // 存值指纹（不持有 Rc ⟹ classifier make_mut strong_count==1 原地 O(tail)）。
            self.derived[level] = Some(LevelFingerprint {
                w_self,
                w_lower,
                tail_self: tower_self[w_self..].to_vec(),
                tail_lower: tower_lower[w_lower..].to_vec(),
            });
        }
    }

    /// 级别-ℓ 事件派生（p92 `collect_snapshot_candidates` 同级**骨架同构**——合法 run 扫描 →
    /// 投影 → decompose → assemble_level_view → provider ext；**差异已披露**：错误不 panic——
    /// 计数落账并跳过该级（NEST_GATE_INDEX 行可见；门是 opt-in 诊断臂，不得击穿生产回测），
    /// 事件吸收走 `absorb_exts` first-wins dedup + 只收确认事件，见 NestChainGate 结构体文档）。
    pub(super) fn derive_level_events(
        &mut self,
        tower: &[std::rc::Rc<Vec<classifier::recursive_tower::LeveledMove>>],
        level: usize,
        as_of: usize,
    ) -> Vec<classifier::level_view::NestCandidateEventExt> {
        use classifier::level_view::{
            assemble_level_view, lower_legs_from, project_extended_windows_carried_only,
            provide_nest_candidate_events_ext, C2LevelViewConfig, C2VersionTuple,
            CoordinateWindow, LevelViewMaterial, LevelViewQuery, ProjectionMaterial,
        };
        let lower = match lower_legs_from(&tower[level - 1]) {
            Ok(legs) => legs,
            Err(_) => {
                self.n_provider_errors += 1;
                return Vec::new();
            }
        };
        let windows = &tower[level];
        let mut out = Vec::new();
        let mut run_start = None;
        for index in 0..=windows.len() {
            let valid = index < windows.len()
                && project_extended_windows_carried_only(std::slice::from_ref(&windows[index]))
                    .is_ok();
            match (run_start, valid) {
                (None, true) => run_start = Some(index),
                (Some(start), false) => {
                    match project_extended_windows_carried_only(&windows[start..index]) {
                        Ok(projection) => {
                            let centers: Vec<_> =
                                projection.seeds.iter().map(|seed| seed.center).collect();
                            let blocks = classifier::decompose::decompose(&centers);
                            let start_source =
                                projection.seeds.first().expect("nonempty run").start_index;
                            let end_source =
                                projection.seeds.last().expect("nonempty run").end_index;
                            let query = LevelViewQuery {
                                level: level as u32,
                                coordinate_window: CoordinateWindow {
                                    start: start_source,
                                    end: end_source,
                                },
                                as_of,
                                version: C2VersionTuple::auto_pairing(),
                            };
                            match assemble_level_view(
                                C2LevelViewConfig { enabled: true },
                                query,
                                LevelViewMaterial {
                                    projection: ProjectionMaterial::ExactThree(&projection),
                                    move_blocks: &blocks,
                                    lower_legs: &lower,
                                    hist: &self.hist,
                                    dif: &self.dif,
                                    close_src: &self.close_src,
                                },
                            ) {
                                Ok(view) => out.extend(provide_nest_candidate_events_ext(
                                    level as u32,
                                    &projection,
                                    &blocks,
                                    &lower,
                                    &view,
                                    &self.hist,
                                    &self.dif,
                                    &self.close_src,
                                    // T1 (#170)：两元锚供给（分型管 + 包含层，单一来源）。
                                    &self.fractals,
                                    &self.merged_bars,
                                )),
                                Err(_) => self.n_provider_errors += 1,
                            }
                        }
                        Err(_) => self.n_provider_errors += 1,
                    }
                    run_start = None;
                }
                _ => {}
            }
        }
        out
    }

    /// first-wins 追加（p92 YieldBook 同构）：只收确认事件（未确认过不了基例门/N2 门，
    /// 对索引零贡献）；身份去重保首次观察（`judge_at` 不后移）。
    pub(super) fn absorb_exts(&mut self, exts: Vec<classifier::level_view::NestCandidateEventExt>) {
        for ext in exts {
            let event = ext.event;
            self.n_events_seen += 1;
            if !event.divergence_confirmed {
                continue;
            }
            let id = classifier::nest::NestEventIdentity::of(&event);
            if !self.seen.insert(id) {
                continue;
            }
            // T1 (#170 键域重锚) → T5a (#207 去方向位)：单键域 (ℓ, 极值价, 组锚@ℓ) →
            // 身份集。两元锚由 provider 在事件构造点携带（禁第二查法）；缺锚（供给未命中）
            // = 诚实跳过 + 计数（下方事件账本登记不受影响）。方向位自身份注册退役
            //（ADR 20260723 裁定 1——`event.side` 仅供交易层，不进身份键）。
            // T5b (#208)：旧 `by_end` 登记（固定 ℓ+1 出场桥键）已删（#206 Q3 判删）。
            if let (Some(price), Some(anchor)) = (ext.extreme_price, ext.group_anchor) {
                self.by_triple_anchor
                    .entry((event.level, price, anchor))
                    .or_default()
                    .push(id);
                // #218 面 B：事件侧两元锚正查账本（同源写入，零新增解析）——二类判同
                // 的事件锚经 build_nest_certificate_index 透传进核（spec ID-2）。
                self.anchor_by_id.insert(id, (price, anchor));
            } else {
                self.n_anchor_misses += 1;
            }
            let slot = event.level as usize;
            if slot >= self.events_by_level.len() {
                self.events_by_level.resize_with(slot + 1, Vec::new);
            }
            self.events_by_level[slot].push(event);
        }
    }

    /// 索引按需重建（事件增长或终端背书账本 Rc 变化）；生产口径固定 B（裁定生产口径，
    /// nest.rs:386-387），终端口径 CWindow 由构建器内部固定（p117 T1 裁定2）。
    pub(super) fn sync_index(&mut self, classification: &classifier::Classification) {
        let event_count: usize = self.events_by_level.iter().map(Vec::len).sum();
        let books_changed = self.book_fps.len() != classification.levels.len()
            || self
                .book_fps
                .iter()
                .zip(classification.levels.iter())
                .any(|(held, level)| !std::rc::Rc::ptr_eq(held, &level.bsp));
        if event_count == self.index_event_count && !books_changed {
            return;
        }
        // #218 面 B：判同参照包接线——oracle = gate 已持 T1 供给线（fractals/merged_bars，
        // projection::anchor_resolver 单一来源，禁第二查法）；事件侧两元锚 = absorb_exts
        // 落账的 anchor_by_id 正查（NestCandidateEventExt 已带，零新增解析）。
        let oracle = classifier::projection::anchor_resolver(&self.fractals, &self.merged_bars);
        let anchor_by_id = &self.anchor_by_id;
        let event_anchor_of = |e: &classifier::level_view::NestCandidateEvent| {
            anchor_by_id
                .get(&classifier::nest::NestEventIdentity::of(e))
                .map(|&(price, anchor)| (Some(price), Some(anchor)))
                .unwrap_or((None, None))
        };
        self.index = classifier::nest_index::build_nest_certificate_index(
            classification,
            &self.events_by_level,
            classifier::nest::NestIntervalCaliber::B,
            &oracle,
            &event_anchor_of,
        );
        self.index_event_count = event_count;
        self.book_fps = classification
            .levels
            .iter()
            .map(|level| std::rc::Rc::clone(&level.bsp))
            .collect();
        self.n_index_builds += 1;
    }

    /// T3 (#172) → T5a (#207) 脚身份解析（恰好存在的锚）：返回 `(极值价, 组锚 a*)`，
    /// 双 `None` 安全。
    ///
    /// - **极值价**：T1 供给线单一来源 `fractal_at_source`（gate 持有的同一分型账本 Rc；
    ///   x 处无 confirmed 分型 ⟹ None——诚实缺锚，禁降级）。
    /// - **组锚 a***：**T2 本级层**条目解析（`cross_level_query(极值价)` 回执中
    ///   `source_index == c.source_index` 者）——层间联动的物化锚（禁第二查法：gate 不自行
    ///   调 `merged_group_anchor` 另起锚解析）。本级层未载（None）/ 该脚未登记 ⟹ None。
    /// - **T5a 方向退役**（ADR 20260723 裁定 1）：解析不携带方向——同一 x 跨型（顶/底）
    ///   共点不产生价格二义（T1 探针已证），层索引按极值价单键命中全部类型登记。
    fn resolve_foot(
        &self,
        c: &super::super::strategy::interp::Candidate,
        classification: &classifier::Classification,
    ) -> (Option<super::super::types::Tick>, Option<usize>) {
        let price = super::super::parser::fractal::fractal_at_source(&self.fractals, c.source_index)
            .map(|f| f.price);
        let Some(price) = price else { return (None, None) };
        let anchor = classification
            .levels
            .get(c.level as usize)
            .and_then(|ls| ls.level_projection.as_ref())
            .and_then(|layer| {
                layer
                    .cross_level_query(price)
                    .matches
                    .iter()
                    .find(|e| e.source_index == c.source_index)
                    .map(|e| e.group_anchor)
            });
        (Some(price), anchor)
    }

    /// T3 (#172) → T5a (#207) 惰性索引重建前置（fill loop 消费）：候选是否可能经链键域
    /// 读到证书——脚可解析 ∧ 任一链事件级键 `(ℓ, 极值价, a*)` 在 `by_triple_anchor`
    /// 有身份集。保守超集（不扫存在性）：true ⟹ `chain_lookup` 可能读索引 ⟹ 须先
    /// `sync_index`；false ⟹ 链必 NoChain/键域查无（不读索引，可跳过重建——决策逐字节不变）。
    /// T4 (#173) 起为进场侧重建**唯一**前置；索引唯一读者 = `chain_lookup`（仅经
    /// `by_triple_anchor` 键取身份后触索引）。
    ///
    /// **T5a 去方向位的等价性论证（保守超集，逐点成立）**：记旧 hint（方向时代）为
    /// H_old(c, delta)。① 锚解析：新 `resolve_foot` 命中条件 = 本级层按极值价有该脚
    /// 条目（任一类型登记）；旧条件 = 按 (delta, 极值价) 有——旧 ⟹ 新（同脚同价同坐标
    /// 条目必在合流后的键下），故锚可解集旧 ⊆ 新。② 键域存在：新键 (ℓ, 价, 锚) 有
    /// 身份集 ⟺ 旧两键 (ℓ, true, 价, 锚) ∪ (ℓ, false, 价, 锚) 任一有（同集合流）；
    /// 旧 hint 的键 (ℓ, delta, 价, 锚) 有 ⟹ 新键有。两分量皆旧 ⟹ 新，故 H_old=true
    /// ⟹ H_new=true（逐点）：**旧触发重建的候选流逐点保持触发**，查询点索引内容 =
    /// (classification, events_by_level) 的确定函数 ⟹ 旧 hint=true 案例的链读出逐字节
    /// 可比；H_new 多出的触发（异型脚/异侧证可解析案例）= 方向退役的解放人口，其
    /// `index_builds` 计数与末次重建时点差异按足迹列账（不进任何判定）。
    pub(super) fn chain_key_hint(
        &self,
        c: &super::super::strategy::interp::Candidate,
        classification: &classifier::Classification,
    ) -> bool {
        let (Some(price), Some(anchor)) = self.resolve_foot(c, classification) else {
            return false;
        };
        (1..=classification.levels.len() as u32)
            .any(|el| self.by_triple_anchor.contains_key(&(el, price, anchor)))
    }

    /// T3 (#172) **严格链查询** → T5a (#207) **去方向形态**（#206 Q1 / ADR 20260723
    /// 裁定 1 执行：身份判据 = 同点递归，方向完全多余）。
    ///
    /// 链构造（语义 pin 见 [`ChainProbe`] 上方模块注）：
    /// 1. 脚身份 = (极值价, 组锚 a*)，经 [`Self::resolve_foot`] 单一来源解析
    ///    （**不携带方向**——同点跨型共点不产生价格二义）；
    /// 2. **恰好存在扫描**：逐级查 T2 层 `cross_level_query(极值价)`，脚存在 =
    ///    回执含组锚 a* 条目（**不问分型类型**——同一 x 可在不同级别分别为顶/底，
    ///    各级自为真）；**链顶** = x 为拐点的最高账本级（层间联动给出，非塔顶窗口、
    ///    非固定 ℓ+1）；
    /// 3. 链区间 **[L0, 链顶]** 逐级查 `by_triple_anchor`(事件级 = 账本级+1, 极值价, a*)
    ///    → 证书身份集（**不滤方向**——异侧登记的证书对链闭合同样有效）→ `n_delta` +
    ///    因果守卫（`judge_at ≤ anchor`，越界整证剔除——现语义逐字）；
    /// 4. 三态裁决 + 缺/断位置极性（判据不涉及方向，逐字保留）。方向见证装置
    ///    （DirWitness/opposite 扫描）已退役——「方向分歧」是非概念。
    ///
    /// 返回 [`ChainProbe`]（三态 + 完整谱系；NoChain 也带已解析的锚/存在性信息供 dump 归因）。
    pub(super) fn chain_lookup(
        &self,
        c: &super::super::strategy::interp::Candidate,
        anchor_index: usize,
        classification: &classifier::Classification,
    ) -> ChainProbe {
        let (price, anchor) = self.resolve_foot(c, classification);
        let (Some(price), Some(anchor)) = (price, anchor) else {
            // 锚不可解（x 处无分型 / 本级层未载 / 该脚未登记）⟹ NoChain（Xzd 回退）。
            return ChainProbe { price, anchor, ..Default::default() };
        };
        // 恰好存在扫描（逐级 T2 层；不问分型类型——同点跨型各级自为真）。
        let mut existence: Vec<bool> = Vec::with_capacity(classification.levels.len());
        for ls in &classification.levels {
            let ex = ls.level_projection.as_ref().is_some_and(|layer| {
                layer
                    .cross_level_query(price)
                    .matches
                    .iter()
                    .any(|e| e.group_anchor == anchor)
            });
            existence.push(ex);
        }
        let Some(chain_top) = existence.iter().rposition(|&e| e) else {
            // 结构不可达（本级已解析出 a* ⟹ 本级必有存在），保守 NoChain。
            return ChainProbe { price: Some(price), anchor: Some(anchor), ..Default::default() };
        };
        // 链区间 [L0, 链顶] 逐级证书查询（事件键级 = 账本级 + 1；键域不滤方向）。
        let mut levels: Vec<ChainLevelGenealogy> = Vec::with_capacity(chain_top + 1);
        for book in 0..=chain_top {
            let event_level = book as u32 + 1;
            if !existence[book] {
                levels.push(ChainLevelGenealogy {
                    level: book as u32,
                    event_level,
                    status: ChainLevelStatus::MissingExistence,
                    gap: None,
                    n_certs: 0,
                    n_causal_clean: 0,
                    rungs: 0,
                });
                continue;
            }
            let mut n_certs = 0usize;
            let mut n_clean = 0usize;
            let mut pass = false;
            let mut rungs = 0usize;
            if let Some(ids) = self.by_triple_anchor.get(&(event_level, price, anchor)) {
                n_certs = ids.len();
                for id in ids {
                    let Some(cert) = self.index.get(id) else { continue };
                    // 因果守卫：链上任一确认钟越过锚定 bar ⟹ 整证剔除（现语义逐字）。
                    if cert.judge_at().iter().any(|&t| t > anchor_index) {
                        continue;
                    }
                    n_clean += 1;
                    // 判定谓词唯一来源（nest.rs 递归核，禁第二查法）；级内 T7 合并同款。
                    pass |= cert.certificate().n_delta();
                    rungs = rungs.max(cert.judge_at().len() - 1);
                }
            }
            let status = if n_clean == 0 {
                if n_certs > 0 {
                    ChainLevelStatus::MissingCausal
                } else {
                    ChainLevelStatus::MissingCert
                }
            } else if pass {
                ChainLevelStatus::Closed
            } else {
                ChainLevelStatus::Broken
            };
            levels.push(ChainLevelGenealogy {
                level: book as u32,
                event_level,
                status,
                gap: None,
                n_certs,
                n_causal_clean: n_clean,
                rungs,
            });
        }
        // 缺/断位置极性（自链顶向下扫：上方有闭合 ⟹ 断，否则缺）+ 连续闭合前缀 + 首位归因。
        let mut seen_closed_above = false;
        let mut closed_down_to: Option<u32> = None;
        let mut prefix_broken = false;
        let mut first_gap: Option<(u32, ChainGapKind)> = None;
        for g in levels.iter_mut().rev() {
            if g.status == ChainLevelStatus::Closed {
                seen_closed_above = true;
                if !prefix_broken {
                    closed_down_to = Some(g.level);
                }
            } else {
                prefix_broken = true;
                let kind = if seen_closed_above {
                    ChainGapKind::Broken
                } else {
                    ChainGapKind::Missing
                };
                g.gap = Some(kind);
                if first_gap.is_none() {
                    first_gap = Some((g.level, kind));
                }
            }
        }
        let n_closed = levels
            .iter()
            .filter(|g| g.status == ChainLevelStatus::Closed)
            .count();
        let verdict = if n_closed == 0 {
            ChainVerdict::NoChain // 零闭合级 ⟹ Xzd 回退（含因果守卫全剔，现语义保留）
        } else if n_closed == levels.len() {
            ChainVerdict::Pass
        } else {
            ChainVerdict::Reject
        };
        ChainProbe {
            verdict,
            price: Some(price),
            anchor: Some(anchor),
            chain_top: Some(chain_top as u32),
            closed_down_to,
            first_gap,
            levels,
        }
    }

    /// 门开单候选裁决：**T3 (#172) 严格链为唯一 nest 判定源**（[`Self::chain_lookup`] 三态：
    /// Pass → `nest_pass`；Reject（缺/断环）→ `nest_n_delta_false`——对标旧 `Some(pass=false)`
    /// 语义位；NoChain → 既有 Xzd 回退逐字不动）。旧并集 `typed_lookup_multi` 与 fixed
    /// `typed_lookup` comparison 格及 shadow dump 装置已随 T4 (#173) 旧桥退役删除；L2 旧臂
    /// （`nest_gate_admit` → `build_gate_certificate`）恒双读落账供既有 cross 对照。
    ///
    /// 链 NoChain 时的回退与旧臂「Nest None」分支同语义，两种消费方式**不等价、不可统一**
    /// （#94 择 (b)：保留重走，理由如下）：旧臂已落 Xzd ⟹ 其读出本就产自
    /// `build_xzd_fallback` 同一单一来源（`build_gate_certificate` Nest-None 分支），
    /// 直接复用语义逐值一致且省一次重算；旧臂产了 L2 nest 证（点包含读出，已退役为对照）
    /// ⟹ 该分支**从未执行 Xzd 评估，不存在可复用的旧 Xzd 读出**，必须重走
    /// [`super::econ_positive::build_xzd_fallback`] 补评 Xzd（Type2/3）
    /// 或拒（Type1/StructBreak/无执行段）；旧臂 None ⟹ 拒。
    pub(super) fn admit(
        &self,
        tower: &[std::rc::Rc<Vec<classifier::recursive_tower::LeveledMove>>],
        c: &super::super::strategy::interp::Candidate,
        hist: &[f64],
        confirm_index: usize,
        classification: &classifier::Classification,
    ) -> (bool, &'static str, NestGateObs) {
        self.admit_inner(tower, c, hist, confirm_index, classification)
    }

    fn admit_inner(
        &self,
        tower: &[std::rc::Rc<Vec<classifier::recursive_tower::LeveledMove>>],
        c: &super::super::strategy::interp::Candidate,
        hist: &[f64],
        confirm_index: usize,
        classification: &classifier::Classification,
    ) -> (bool, &'static str, NestGateObs) {
        use super::super::strategy::voice::VoiceSide;
        use super::super::types::Side;
        let delta = match c.dir {
            VoiceSide::Long => Side::Long,
            VoiceSide::Short => Side::Short,
            VoiceSide::Flat => {
                return (false, "flat_dir", NestGateObs::default()); // 与旧臂同语义
            }
        };
        let lvl = c.level as usize;
        if classification.levels.get(lvl).is_none() {
            return (false, "no_level", NestGateObs::default());
        }
        // T3 (#172) 唯一 nest 判定源：严格链（恰好存在层间联动 + 逐级自治证书，闭合到 L0）。
        // T5a (#207)：链查询不携带方向（身份 = 同点递归）；delta 仅供交易层（旧臂对照与
        // Xzd 回退——同点递归链回答「这个点是不是确认的拐点」，买卖标签回答「做哪边」）。
        let chain = self.chain_lookup(c, confirm_index, classification);
        // L2 旧臂对照读出（链判定不消费；链 NoChain 时其 Xzd 通道被复用）。
        let (old_admit, old_channel) = nest_gate_admit(tower, c, hist, confirm_index, classification);
        let (admit, channel, xzd_fallback, reused_old_xzd) = match chain.verdict {
            ChainVerdict::Pass => (true, "nest_pass", false, false),
            ChainVerdict::Reject => (false, "nest_n_delta_false", false, false),
            ChainVerdict::NoChain => match old_channel {
                // 旧臂 Xzd 读出 = build_xzd_fallback 同一单一来源的结果 ⟹ 复用即逐值一致；
                // admit≡old_admit by construction ⟹ obs 打 reused_old_xzd，cross 对照差剔除该自证。
                "xzd_pass" | "xzd_gate_fail" => (old_admit, old_channel, false, true),
                "nest_pass" | "nest_n_delta_false" => {
                    // 旧臂此分支只评了 L2 nest n_delta，从未执行 Xzd 评估 ⟹ 无可复用读出，
                    // 必须重走单一来源 build_xzd_fallback（#94 择 (b)，见函数文档）。
                    let ls = &classification.levels[lvl];
                    let (sub_centers, sub_bsp) = if lvl > 0 {
                        match classification.levels.get(lvl - 1) {
                            Some(sub) => (sub.centers.as_slice(), sub.bsp.as_slice()),
                            None => (&[][..], &[][..]),
                        }
                    } else {
                        (&[][..], &[][..])
                    };
                    match super::econ_positive::build_xzd_fallback(
                        tower,
                        lvl,
                        c.source_index,
                        delta,
                        &c.bits,
                        confirm_index,
                        &ls.bsp,
                        sub_centers,
                        sub_bsp,
                    ) {
                        Some(ev) => {
                            let pass = ev.gate_pass();
                            (pass, if pass { "xzd_pass" } else { "xzd_gate_fail" }, true, false)
                        }
                        None => (false, "cert_none", true, false),
                    }
                }
                _ => (false, "cert_none", false, false),
            },
        };
        let obs = NestGateObs {
            old_admit,
            old_channel,
            xzd_fallback,
            reused_old_xzd,
            chain_verdict: chain.verdict,
            chain_top: chain.chain_top,
            chain_closed_down_to: chain.closed_down_to,
            chain_first_gap: chain.first_gap,
            chain_genealogy: chain.levels,
            chain_price: chain.price,
            chain_anchor: chain.anchor,
        };
        (admit, channel, obs)
    }
}


/// [close_pred 折 𝒦_Θ] 计算 [`KThetaRiskGate`]（Q2：风控 stop/risk 作可行集约束门，非第二出口）。
///
/// 复用 `exec::close_pred` 契约锚（no-patch-keep-primitive）：
/// - **risk（GlobalRiskClose）**：`risk_mode(equity)` ∈ {Insolvent,Liquidation} ⟹ `force_flat`
///   （v0 可计算 Insolvent `E_t≤0`；账户层 MM/buffer/liq 未建模，诚实有效域 L0）。
/// - **stop（结构止损触及）**：per 活动腿从**入场冻结的** `structural_stop`（[`LedgerOpen::entry_stop`]，
///   开仓 bar 一次性算）判 `stop_hit(bar,…)` 触及——多腿止损 ⟹ `stop_long`、空腿止损 ⟹ `stop_short`。
///   族A 修复：旧路径逐 bar 用 `leg.source_index`（carrier 走势 ρ，随父延伸漂移）回查 `classification`
///   ⟹ drifted ρ 不命中 ⟹ 静默跳过 ⟹ 跨趋势持仓 stop 永不触发；改入场冻结对齐 nautilus
///   `record_held_voice`（exitfix-research §族A）。
/// - **reverse_signal 不入本门**：反向信号关活动腿走 `interpret` 𝒟_x（腿级单出口）；
///   **parent_invalid** v0 root 恒 false（无父）。
pub(super) fn k_theta_risk_gate(
    prev_active: &[super::super::strategy::interp::ActiveLeg],
    open_trades: &std::collections::HashMap<classifier::recursive_tower::ElementId, LedgerOpen>,
    bar: &Bar,
    equity: f64,
    p_t: f64,
    px: f64,
    margin: Option<&super::super::strategy::risk::MarginModel>,
) -> (super::super::strategy::coverage::KThetaRiskGate, super::super::strategy::risk::RiskMode) {
    use super::super::strategy::coverage::KThetaRiskGate;
    use super::super::strategy::exec::{close_pred, stop_hit, CloseTriggers, FillSide};
    use super::super::strategy::risk::{
        global_risk_close, margin_inputs, risk_mode, RiskMode, RiskModeInput,
    };
    use super::super::strategy::voice::VoiceSide;

    // risk mode：有 margin 注入且 bar 时间落某快照段 ⟹ 真实 MM/liq/buffer（as_of 零前视，
    // margin-design §2.7）；否则退化 MM=0（bit-exact 现状，M1/M2/M3 不可达）。
    let mode = match margin.and_then(|m| m.book.as_of(bar.timestamp).map(|s| (m, s))) {
        Some((m, sched)) => {
            // p_t 净 lot × mark = 净名义（美元，margin-design §2.2：net_notional 已折算勿再乘价）。
            let net_notional_usd = p_t.abs() * px;
            risk_mode(&margin_inputs(net_notional_usd, equity, sched, &m.cushions))
        }
        None => risk_mode(&RiskModeInput {
            equity,
            maint_margin: 0.0,
            buffer1: 0.0,
            buffer2: 0.0,
            liq_flag: false,
        }),
    };
    let risk_close = global_risk_close(mode);
    // M2/M3（Deleverage/CloseOnly）：净幅上限=当前 |p_t|（margin-design §2.8，禁增仓 → 真改订单流）。
    let no_increase_cap = match mode {
        RiskMode::Deleverage | RiskMode::CloseOnly => Some(p_t.abs()),
        _ => None,
    };

    // ★族A 修复（formal-chain §9 closePred Stop 覆盖度）：stop 从**入场冻结的 structural_stop**
    // （[`LedgerOpen::entry_stop`]）读出，非逐 bar 用 `leg.source_index` 回查 classification。
    // 旧路径 `leg.source_index` 是 carrier 走势的 ρ（右端点），随父延伸漂移（coverage.rs 父延伸
    // 不变量 ρ≥旧 source_index）⟹ 按 drifted ρ 查 bsp_index 不命中 ⟹ `None => continue` 静默
    // 跳过该腿 stop 判定 ⟹ 持仓跨大级别趋势时 stop 永不触发（族 A 根因，exitfix-research §族A）。
    // 入场路径（[`candidate_stop_dist`]）用候选 `c.source_index`（bsp 确认点）查得对——两路径同腿
    // 不同坐标是 bug。本修复对齐两路径 + nautilus `record_held_voice`（入场一次性算 stop 冻结到
    // HeldVoice.stop，exitfix-research line 49）：stop 值固定在开仓结构 = formal-chain §9 语义
    // （结构失效价触及，非 trailing）。L0 静态根因；L2 dump（3765 等笔 stop 读出实际值）待 OOS。
    let mut long_stop = false;
    let mut short_stop = false;
    for leg in prev_active {
        let exit_side = match leg.dir {
            VoiceSide::Long => FillSide::Sell,
            VoiceSide::Short => FillSide::Buy,
            VoiceSide::Flat => continue, // Flat 不入活动集（防御性）
        };
        let stop = match open_trades.get(&leg.id).and_then(|o| o.entry_stop) {
            Some(s) => s,
            None => {
                // 腿不在 open_trades = **结构走势载体**（非 campaign 持仓）。`next_active` 含走势元素
                // （AncOK 祖先闭包 + [`coverage::restore_ancestor_chain_from_registry`] 注入的父 carrier
                // —— empirically：活动集里 level-1 走势载体与 open_trades 里 level-0 campaign 持仓并存，
                // open_trades 仅记 campaign）。这类腿的 `dir`=走势 eps（非持仓方向）、`source_index`=
                // 走势 ρ（漂移）—— 非开仓结构，无 stop 可读。旧路径用 drifted ρ 查 bsp_index 也返
                // None ⟹ 同样跳过，但旧路径把载体**误当持仓**算 stop（无意义计算）。本修复按
                // open_trades 成员区分 campaign 持仓 vs 结构载体，仅前者判 stop（族A 正域）。
                continue;
            }
        };
        if !bar.untradable && stop_hit(bar, stop, exit_side) {
            match leg.dir {
                VoiceSide::Long => long_stop = true,
                VoiceSide::Short => short_stop = true,
                VoiceSide::Flat => {}
            }
        }
    }

    // close_pred 折 𝒦_Θ（契约锚保留）：风控项（stop ∨ risk）→ 方向约束门。
    // G3（#138）：mode 一并透出——z 第 13 维 risk_mode 的账本态真值源（每 bar 已算，零重算）。
    (
        KThetaRiskGate {
            force_flat: risk_close, // GlobalRiskClose ⟹ 𝒦_Θ={0}
            stop_long: close_pred(&CloseTriggers {
                parent_invalid: false,
                reverse_signal: false,
                stop: long_stop,
                risk_close,
            }),
            stop_short: close_pred(&CloseTriggers {
                parent_invalid: false,
                reverse_signal: false,
                stop: short_stop,
                risk_close,
            }),
            no_increase_cap, // M2/M3 净幅上限（margin-design §2.8）
        },
        mode,
    )
}

/// χ_t 阈值过滤上下文（task #41 chi-theta-filter）——`pi_theta_fill_loop` 的可选候选集过滤器。
///
/// `None`（不传）⟹ χ≡1 全覆盖（解释器处理全部 Γ_t，frozen Θ v0 bit-exact 不变）。`Some(ctx)` ⟹
/// 每 bar 把 `step_gamma`（确认-bar 部署候选）过滤为 `Γ_t^trade={γ:μ(z_γ)>θ}`（[`selector::filter_gamma`]）。
///
/// **μ 表因果性由调用方负责**（selector.rs 模块头诚实声明）：`est` 若来自全窗 in-sample 交易 = 泄漏
/// （L1 选择器逻辑生效证明，非 L2 alpha）；walk-forward 增量 μ 是下游 delta-r-alpha 工位（L2/L3）。
/// θ 为**常数**（不从样本 μ 分布选，codex Q1 审查确认无前视）。
pub struct ChiFilterCtx<'a> {
    /// μ(z) 表（z 六维全互斥分类的样本边际收益）。
    pub est: &'a super::mu_estimator::MuEstimator,
    /// θ 阈值（成本/风险门槛，Θ_risk 参数，常数）。
    pub theta: f64,
    /// z_alpha 单边置信分位（如 1.645=95%）：准入用 LCB(μ)=mean−z_alpha·std/√n（严格alpha.pdf p25
    /// §12 防高维 z 过拟合）。z_alpha=0 ⟹ LCB=mean ⟹ 退化回裸 μ 门（向后兼容）。
    pub z_alpha: f64,
    /// 无 LCB 证据（空类 μ=None 或 n<2 单样本）χ 取值：false=不交易（最诚实，codex Q3）；
    /// true=全覆盖默认交易。
    pub treat_empty_as_pass: bool,
    /// 准入量切换（acc-three-way-l2 #83）：`None` ⟹ 准入量 = LCB(μ)（默认，z_alpha 驱动，frozen
    /// bit-exact）；`Some(τ²)` ⟹ 准入量 = mu_shrink(z,τ²) 层级收缩（z_alpha 忽略）。
    pub shrink_tau_sq: Option<f64>,
}

/// barrier 缓冲系数 κ 政策注入（A10 附则A 裁定——**优先序写死**：env `KAPPA_BARRIER_*`
/// （L2 敏感性诊断覆写，codex `.kappa-ruling-20260704`）> `config.risk_policy` > `baseline()` κ=0）。
///
/// env `KAPPA_BARRIER_NUM` / `KAPPA_BARRIER_DEN`（默认 den=1）⟹ `RiskPolicy::try_new_ratio(num, den)`；
/// **两者都未设（或 num 不可解析，现状语义）⟹ 落 `config.risk_policy`；config=None ⟹ `baseline()`
/// κ=0**——生产/测试逐字节不变（bit-exact）。非法 env 值（负分子/非正分母）⟹ panic（诊断 knob
/// 快失败，不静默降级掩盖网格错配——语义不变）。单源纪律防双源静默漂移（χ G2 先例）。
pub(super) fn kappa_policy_resolved(config_policy: Option<RiskPolicy>) -> RiskPolicy {
    let env = std::env::var("KAPPA_BARRIER_NUM")
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .map(|num| {
            let den = std::env::var("KAPPA_BARRIER_DEN")
                .ok()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(1);
            (num, den)
        });
    kappa_priority_resolve(env, config_policy)
}

/// κ 优先序纯函数（A10 附则A 写死：env>config>baseline；可测——不碰进程 env）。
/// env 非法值 panic（与 env knob 语义一致）；env=None ⟹ config 或 baseline。
pub(super) fn kappa_priority_resolve(env: Option<(i64, i64)>, config_policy: Option<RiskPolicy>) -> RiskPolicy {
    match env {
        Some((num, den)) => RiskPolicy::try_new_ratio(num, den)
            .unwrap_or_else(|| panic!("非法 κ barrier grid 值 num={num} den={den}（要求 num≥0 ∧ den>0）")),
        None => config_policy.unwrap_or_else(RiskPolicy::baseline),
    }
}

// ═════════════════ T5a（#207）shadow dump 装置（#[cfg(test)] 限定，env 驱动）═════════════════

/// T5a (#207) 链判定 shadow dump（T3_SHADOW_DUMP 同型单语义版）：逐候选 JSONL 落
/// **新链结果**（三态 + 谱系 + 缺/断）+ admit/channel——**不含旧链列**（旧链基线 =
/// T3 dump `typed-none-strict-chain-dump-candidates-20260723.jsonl` 4875 行，阶段 B
/// 对照面；本装置不做新旧双读）。`#[cfg(test)]` 限定：生产构建不含本模块，门开形态
/// 未设 env 时全方法 no-op（零开销红线不死）。
///
/// 触发（两路，均测试专用）：
/// - env `T5A_CHAIN_DUMP_PATH=<file>`：惰性单文件（首条记录时打开；`window` 字段取
///   `M8_WIN_FILTER` env，未设 = null）——单窗重放/小型测试用。
/// - env `T5A_CHAIN_DUMP_DIR=<dir>` + m8 分窗接线：wverify m8 测试逐窗调
///   [`open_for_window`]`(tag)`（开 `<dir>/t5a_chain_dump_<tag>.jsonl`），窗末 [`close`]。
///
/// 线程局部持有写入器（进程级 env 会被并行测试同时读到——OPSEM_DUMP_DIR 2026-07-13
/// 竞态实录同型风险；m8 三窗同线程串行，open/close 逐窗切换）。写失败只 eprintln 一次
/// 不 panic（诊断臂不得击穿回测，`n_provider_errors` 同纪律）。
#[cfg(test)]
pub(super) mod t5a_chain_dump {
    use super::{ChainGapKind, ChainLevelStatus, ChainVerdict, NestGateObs};
    use std::io::Write;

    thread_local! {
        static WRITER: std::cell::RefCell<Option<std::io::BufWriter<std::fs::File>>> =
            std::cell::RefCell::new(None);
        static WINDOW: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
        /// PATH 模式惰性打开只试一次（打开失败不每候选重试）。
        static LAZY_TRIED: std::cell::Cell<bool> = std::cell::Cell::new(false);
    }

    /// m8 分窗接线：env `T5A_CHAIN_DUMP_DIR` 设置时开 `<dir>/t5a_chain_dump_<tag>.jsonl`；
    /// 未设 ⟹ no-op（返回 false）。每窗调用一次（wverify m8 测试窗首），窗末 [`close`]。
    pub(crate) fn open_for_window(tag: &str) -> bool {
        let Ok(dir) = std::env::var("T5A_CHAIN_DUMP_DIR") else { return false };
        if dir.is_empty() {
            return false;
        }
        let path = std::path::Path::new(&dir).join(format!("t5a_chain_dump_{tag}.jsonl"));
        match std::fs::File::create(&path) {
            Ok(f) => {
                WRITER.with(|w| *w.borrow_mut() = Some(std::io::BufWriter::new(f)));
                WINDOW.with(|w| *w.borrow_mut() = Some(tag.to_string()));
                eprintln!("[t5a_dump] window={tag} → {}", path.display());
                true
            }
            Err(e) => {
                eprintln!("[t5a_dump] 开窗 dump 创建失败 {}：{e}（no-op 继续，不击穿回测）", path.display());
                false
            }
        }
    }

    /// 窗末关闭（flush 由 Drop 保证）；同时复位 PATH 惰性标记（下一窗/下一测试可重开）。
    pub(crate) fn close() {
        WRITER.with(|w| *w.borrow_mut() = None);
        WINDOW.with(|w| *w.borrow_mut() = None);
        LAZY_TRIED.with(|t| t.set(false));
    }

    /// PATH 模式惰性打开（每线程一次）：env `T5A_CHAIN_DUMP_PATH` 未设 ⟹ 保持无写入器。
    fn lazy_open_from_env() {
        LAZY_TRIED.with(|t| t.set(true));
        let Ok(path) = std::env::var("T5A_CHAIN_DUMP_PATH") else { return };
        if path.is_empty() {
            return;
        }
        match std::fs::File::create(&path) {
            Ok(f) => {
                WRITER.with(|w| *w.borrow_mut() = Some(std::io::BufWriter::new(f)));
                WINDOW.with(|w| *w.borrow_mut() = std::env::var("M8_WIN_FILTER").ok());
                eprintln!("[t5a_dump] PATH 模式 → {path}");
            }
            Err(e) => eprintln!("[t5a_dump] dump 创建失败 {path}：{e}（no-op 继续）"),
        }
    }

    fn gap_kind_str(k: ChainGapKind) -> &'static str {
        match k {
            ChainGapKind::Missing => "missing",
            ChainGapKind::Broken => "broken",
        }
    }

    /// 逐候选记录（fill loop 门开分支内调用；无写入器 ⟹ no-op）。
    /// `obs` = 本候选 `NestChainGate::admit` 的读出（链三态 + 谱系 + 缺/断 + 两元锚）。
    pub(crate) fn record(
        bar: usize,
        c: &super::super::super::strategy::interp::Candidate,
        obs: &NestGateObs,
        admit: bool,
        channel: &str) {
        use super::super::super::strategy::voice::VoiceSide;
        let tried = LAZY_TRIED.with(|t| t.get());
        if !tried {
            lazy_open_from_env();
        }
        let window = WINDOW.with(|w| w.borrow().clone());
        WRITER.with(|w| {
            let mut slot = w.borrow_mut();
            let Some(writer) = slot.as_mut() else { return };
            let verdict = match obs.chain_verdict {
                ChainVerdict::Pass => "pass",
                ChainVerdict::Reject => "reject",
                ChainVerdict::NoChain => "no_chain",
            };
            let dir = match c.dir {
                VoiceSide::Long => "Long",
                VoiceSide::Short => "Short",
                VoiceSide::Flat => "Flat",
            };
            let levels: Vec<serde_json::Value> = obs
                .chain_genealogy
                .iter()
                .map(|g| {
                    let status = match g.status {
                        ChainLevelStatus::Closed => "closed",
                        ChainLevelStatus::Broken => "broken",
                        ChainLevelStatus::MissingExistence => "missing_existence",
                        ChainLevelStatus::MissingCert => "missing_cert",
                        ChainLevelStatus::MissingCausal => "missing_causal",
                    };
                    serde_json::json!({
                        "level": g.level,
                        "event_level": g.event_level,
                        "status": status,
                        "gap": g.gap.map(gap_kind_str),
                        "certs": g.n_certs,
                        "clean": g.n_causal_clean,
                        "rungs": g.rungs,
                    })
                })
                .collect();
            let line = serde_json::json!({
                "window": window,
                "bar": bar,
                "level": c.level,
                "source_index": c.source_index,
                // dir = 交易层方向标签（候选自报，描述列）——身份层不消费（T5a 方向退役）。
                "dir": dir,
                "price": obs.chain_price,
                "anchor": obs.chain_anchor,
                "chain": {
                    "verdict": verdict,
                    "top": obs.chain_top,
                    "closed_down_to": obs.chain_closed_down_to,
                    "first_gap": obs
                        .chain_first_gap
                        .map(|(l, k)| serde_json::json!({"level": l, "kind": gap_kind_str(k)})),
                    "levels": levels,
                },
                "admit": admit,
                "channel": channel,
            });
            // 写失败不击穿回测（诊断臂纪律，n_provider_errors 同款）；io 错误随
            // BufWriter 后续写入自然显现，不逐候选报警。
            let _ = writeln!(writer, "{line}");
        });
    }
}


#[cfg(test)]
mod issue467_exit_candidate_tests {
    use super::super::super::classifier::recursive_tower::ElementId;
    use super::super::super::strategy::interp::ActiveLeg;
    use super::super::super::strategy::voice::VoiceSide;
    use super::{exit_candidate_would_close, NestGateStats};

    fn active_leg(level: u32, dir: VoiceSide, ordinal: u64) -> ActiveLeg {
        ActiveLeg {
            level,
            dir,
            source_index: ordinal as usize,
            lambda: ordinal as usize,
            id: ElementId { level, ordinal },
            parent_id: None,
            is_boundary_root: true,
            op_parent: None,
        }
    }

    #[test]
    fn nest_gate_exit_candidate_counts_complete_matching_subpopulation() {
        let prev_active = [
            active_leg(2, VoiceSide::Long, 0),
            active_leg(1, VoiceSide::Short, 1),
        ];
        let cases = [
            // admit, level, candidate direction, would drive same-level opposite-leg close
            (false, 2, VoiceSide::Short, true),
            (true, 1, VoiceSide::Long, true),
            (false, 2, VoiceSide::Long, false),
            (false, 3, VoiceSide::Short, false),
            (false, 2, VoiceSide::Flat, false),
        ];
        let mut stats = NestGateStats::default();

        for (admit, level, dir, expected) in cases {
            let would_close = exit_candidate_would_close(level, dir, &prev_active);
            assert_eq!(would_close, expected, "level={level} dir={dir:?}");
            stats.observe_exit_candidate(admit, would_close);
        }

        assert_eq!(stats.exit_cand_total, 2, "完整子总体只含同级反向持仓匹配");
        assert_eq!(stats.exit_cand_admitted, 1, "admitted 侧同法计数");
        assert_eq!(stats.exit_cand_rejected(), 1, "rejected 侧归因计数");
        assert_eq!(
            stats.exit_cand_report_line(),
            "NEST_GATE_EXIT_CAND total=2 admitted=1 rejected=1",
            "π 可达臂使用独立行名，避免与 deprecated v1/dual NEST_GATE_EXIT 混淆"
        );
    }
}
