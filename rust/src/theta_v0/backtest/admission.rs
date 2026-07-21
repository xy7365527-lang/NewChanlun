//! ★准入门 seam（B-M2，#89）：χ/nest/k_Θ 三门 + κ 解析，自 runner.rs 纯移动
//!（设计 chanlun/plans/runner-rs-seam-designs-20260721.md §M2）。
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
/// `THETA_NEST_CERT_GATE=1` ⟹ ①π 开仓准入门：#75（N3-T2）起 **typed 真链为唯一 nest 判定源**
/// （[`NestChainGate`] 证书索引，N^δ 跨级递归链判定）；L2 旧臂
/// [`super::econ_positive::build_gate_certificate`] 双读落账作对照——**typed 无证时回退旧臂
/// Xzd 通道**（Xzd 无 typed 对应物，经 `build_xzd_fallback` 单一来源 econ_positive.rs:1606：
/// 旧臂已落 Xzd ⟹ 直接复用其判定，旧臂产 L2 nest 证 ⟹ 重走单一来源补评）；②v1/双账退出循环的
/// 反向项 χ^{σ_p} 消费对象从裸 BspBits 升格为 typed nest 证书基例
///（[`strategy::exit::exit_decision_for_nested_cert`]，实装卡 §2.4）。未设/非"1" ⟹
/// 两臂整体跳过，全部路径逐字节不变（bit-exact 回归锁）。
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

/// ★nest-gate 观测统计（升格路径 b 实装卡 §3.4-3「拒绝率实测落账」）：门开启时逐候选落账，
/// 门关闭恒零不输出。通道标签 = [`nest_gate_admit`] 第二返回值。
/// ★#75 扩展：真链命中/链深构成（准拒分账）+ L2 旧臂对照差 + Xzd 回退计数。
/// ★#94：复用通道（typed 无证 ∧ 旧臂 Xzd 复用，admit≡old_admit by construction）自证成分
/// 不入对照差，单独计 `cross_reuse`（输出分工：STATS=准入判定分账，CHAIN=真链命中/链深/对照）。
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
    // ── #75 真链读数 ──
    pub(super) typed_found: usize,
    pub(super) typed_none: usize,
    /// 真链命中证书的链深构成（准入侧）：rungs 0 / 1 / ≥2。
    pub(super) admit_rungs: [usize; 3],
    /// 真链命中证书的链深构成（拒绝侧）。
    pub(super) rej_rungs: [usize; 3],
    pub(super) xzd_fallback: usize,
    /// L2 旧臂对照（双读落账）：两路一致 / 旧准新拒 / 旧拒新准——仅含两路**独立**判定案例。
    pub(super) cross_agree: usize,
    pub(super) cross_old_pass_new_rej: usize,
    pub(super) cross_old_rej_new_pass: usize,
    /// 复用通道单独记账（#94）：typed 无证 ∧ 旧臂已落 Xzd ⟹ admit≡old_admit by construction，
    /// 「一致」是自证成分——**不计入** agree/对照差三列，单列呈现。
    pub(super) cross_reuse: usize,
    /// #112 multi 命中的级别谱系计数：level ℓ → 命中该级的候选数（一候选同级至多一次）。
    pub(super) typed_hits_by_level: std::collections::BTreeMap<u32, usize>,
    /// #112 旧 fixed `typed_lookup` 与 multi `typed_lookup_multi` 的 `Option<bool>` 分歧数。
    pub(super) single_multi_divergence: usize,
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
        match obs.typed_rungs {
            Some(rungs) => {
                self.typed_found += 1;
                let bucket = rungs.min(2);
                if admit {
                    self.admit_rungs[bucket] += 1;
                } else {
                    self.rej_rungs[bucket] += 1;
                }
            }
            None => self.typed_none += 1,
        }
        if obs.xzd_fallback {
            self.xzd_fallback += 1;
        }
        for &level in &obs.typed_hit_levels {
            *self.typed_hits_by_level.entry(level).or_default() += 1;
        }
        if obs.single_admit != obs.multi_admit {
            self.single_multi_divergence += 1;
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
//   signal.rs:701 `end.source_index`）、`interval_b`/`seg_c_full` 同为段坐标。生产交叉证据：
//   `terminal_bits_in_book`（nest.rs:607-628）在同一窗口表达式内比较
//   `BspPoint.source_index` 与事件 `interval_b`/`turn_source`（p117 生产查法，task-101
//   实测 91/91 绑定、median |dt|=0）；L2 旧臂 `find_move_by_end_index(exec_moves, source_index)`
//   （econ_positive.rs:899-900）同域匹配塔段端点。⟹ 同一 L0 原始 K 序坐标系，确证成立。
// - **级别桥**：fixed comparison 仍取候选 lvl ↔ 基例事件 ℓ = lvl + 1；#111/#112
//   multi 判定从 `level_origin+1` 起扫描所有更深级别（ADR 裁定 2）。T1 移位
//   `event_bsp_book_level(ℓ)=ℓ-1` 仍定义级别-ℓ 事件的终端背书账本。
// - **值桥**：候选 `source_index` == 事件 `seg_c_full.1`（离开段终点 = 背驰制造的买卖点位置）。
//   trend 域 `turn_source` = t* ≤ `seg_c_full.1`（R1 收束），**不能**用 `turn_source` 等值桥——
//   `seg_c_full`（裁定 #64 §2(d) additive 记录）正是收束前全段坐标的权威载体；pan 域
//   `seg_c_full.1` == `turn_source` 恒成立（level_view.rs:874-875）。
// - **多证书合并（T7 裁定：布尔 + 最强档）**：同桥多证书任一 `n_delta` 通过即准，
//   链深取最深一张（仅归因，不线性放大）。

/// 真链门观测（`NestGateStats::observe` 第三入参）：multi typed 命中谱系/链深 + fixed typed
/// 与 L2 旧臂两组对照读出。
#[derive(Debug, Default, Clone)]
pub(super) struct NestGateObs {
    /// multi 真链命中证书的链深（rungs = judge_at.len()-1，跨级合并取最深）；None = multi 无证。
    pub(super) typed_rungs: Option<usize>,
    /// multi 命中的 nest 级别集合（升序、去重；来自 `MultiLevelTypedLookup::hits`）。
    pub(super) typed_hit_levels: Vec<u32>,
    /// 旧 fixed ℓ=lvl+1 typed 路径的判定，只作 single/multi 对照，不参与准入。
    pub(super) single_admit: Option<bool>,
    /// multi typed 路径的合并判定；None 时才进入既有 Xzd fallback 三分支。
    pub(super) multi_admit: Option<bool>,
    /// L2 旧臂判定（对照读出；typed 无证 ∧ 旧臂已落 Xzd 通道时被复用为门判定——
    /// 该复用案例的「两路一致」是自证，单独计 cross_reuse 不入对照差，见 reused_old_xzd）。
    pub(super) old_admit: bool,
    /// typed 无证 ∧ L2 有证 ⟹ 经 `build_xzd_fallback` 走 Xzd 回退（语义变更集，对照可见）。
    pub(super) xzd_fallback: bool,
    /// typed 无证 ∧ 旧臂已落 Xzd 通道 ⟹ 复用旧臂 Xzd 读出为门判定（Xzd 无 typed 对应物，
    /// 旧臂该通道与 `build_xzd_fallback` 同一单一来源）；admit≡old_admit by construction。
    pub(super) reused_old_xzd: bool,
}

/// #111 多级查询单级命中（谱系记录：命中在哪个级别，供 #112 消费与 miss 归因）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MultiLevelTypedHit {
    /// 命中基例事件的 nest 级别 ℓ（≥ level_origin+1）。
    pub(super) level: u32,
    /// 该级 T7 合并判定（任一证书 `n_delta` 通过即准）。
    pub(super) pass: bool,
    /// 该级最深链深（rungs = judge_at.len()-1，仅归因）。
    pub(super) rungs: usize,
}

/// #111 多级查询结果：级间保留谱系（`hits` 按级别升序），`merged` = 跨级 T7 同款
/// 布尔合并 + 最深链深（仅归因，不线性放大）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MultiLevelTypedLookup {
    pub(super) merged: (bool, usize),
    pub(super) hits: Vec<MultiLevelTypedHit>,
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
    /// 身份 → 收束前全段坐标（值桥载体）。
    pub(super) seg_c_full: std::collections::HashMap<classifier::nest::NestEventIdentity, (usize, usize)>,
    pub(super) seen: std::collections::HashSet<classifier::nest::NestEventIdentity>,
    /// 值桥反查：(级别 ℓ, seg_c_full.1, is_long) → 基例身份集（T7 合并候选）。
    pub(super) by_end: std::collections::HashMap<(u32, usize, bool), Vec<classifier::nest::NestEventIdentity>>,
    /// #111 多级键域（expand）：(seg_c_full.1, is_long) → **全级别**基例身份集。级别居于
    /// 身份内（`NestEventIdentity.level`），使同一 source_index 在多个级别的证书都可被查到
    ///（[`Self::typed_lookup_multi`] 消费）。与 `by_end` 在 [`Self::absorb_exts`] 单写点
    /// 同步登记；旧固定键域 `by_end` 保留并存，既有查询路径零改动。
    pub(super) by_end_multi: std::collections::HashMap<(usize, bool), Vec<classifier::nest::NestEventIdentity>>,
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
            seg_c_full: std::collections::HashMap::new(),
            seen: std::collections::HashSet::new(),
            by_end: std::collections::HashMap::new(),
            by_end_multi: std::collections::HashMap::new(),
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
            seg_c_full: std::collections::HashMap::new(),
            seen: std::collections::HashSet::new(),
            by_end: std::collections::HashMap::new(),
            by_end_multi: std::collections::HashMap::new(),
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
            self.seg_c_full.insert(id, ext.seg_c_full);
            let is_long = matches!(event.side, super::super::types::Side::Long);
            self.by_end
                .entry((event.level, ext.seg_c_full.1, is_long))
                .or_default()
                .push(id);
            // #111 多级键域：与 by_end 同步登记（级别在身份内），旧键域零改动。
            self.by_end_multi
                .entry((ext.seg_c_full.1, is_long))
                .or_default()
                .push(id);
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
        self.index = classifier::nest_index::build_nest_certificate_index(
            classification,
            &self.events_by_level,
            classifier::nest::NestIntervalCaliber::B,
        );
        self.index_event_count = event_count;
        self.book_fps = classification
            .levels
            .iter()
            .map(|level| std::rc::Rc::clone(&level.bsp))
            .collect();
        self.n_index_builds += 1;
    }

    /// 身份桥键存在性（`by_end` 增量维护，O(1)）：无键 ⟹ `typed_lookup` 必 None
    /// （索引无需重建——惰性重建的精确前置判据，决策逐字节不变）。
    pub(super) fn has_bridge_key(
        &self,
        lvl: usize,
        source_index: usize,
        delta: super::super::types::Side,
    ) -> bool {
        self.by_end.contains_key(&(
            lvl as u32 + 1,
            source_index,
            matches!(delta, super::super::types::Side::Long),
        ))
    }

    /// #112 multi 身份桥键存在性（`by_end_multi` 与 `by_end` 在 `absorb_exts` 同点增量维护，
    /// O(1)）：无键 ⟹ `typed_lookup_multi` 必 None。只作 entry 侧惰性索引重建前置；true
    /// 仍可能因级别下界、终端背书或因果守卫而查无证书。
    pub(super) fn has_bridge_key_multi(
        &self,
        source_index: usize,
        delta: super::super::types::Side,
    ) -> bool {
        self.by_end_multi.contains_key(&(
            source_index,
            matches!(delta, super::super::types::Side::Long),
        ))
    }

    /// 身份桥反查（模块级桥注：级别 ℓ=lvl+1，值桥 source_index==seg_c_full.1，side 同向）。
    /// T7 布尔合并：任一命中证书 `n_delta` 通过即准；链深取最深一张（仅归因）。
    /// 命中证书经 `cert.certificate().n_delta()` 读出（nest.rs 递归核单一来源，禁第二查法）。
    pub(super) fn typed_lookup(
        &self,
        lvl: usize,
        source_index: usize,
        delta: super::super::types::Side,
    ) -> Option<(bool, usize)> {
        let ids = self.by_end.get(&(
            lvl as u32 + 1,
            source_index,
            matches!(delta, super::super::types::Side::Long),
        ))?;
        let mut merged: Option<(bool, usize)> = None;
        for id in ids {
            if let Some(cert) = self.index.get(id) {
                let pass = cert.certificate().n_delta();
                let rungs = cert.judge_at().len() - 1;
                merged = Some(match merged {
                    Some((p, r)) => (p || pass, r.max(rungs)),
                    None => (pass, rungs),
                });
            }
        }
        merged
    }

    /// #111 多级递归查询路径（#112 起由 entry 门消费；旧固定 ℓ=lvl+1 路径
    /// [`Self::typed_lookup`] 保留为 comparison-only 读出）。
    ///
    /// 从候选实际所在级别 `level_origin` 出发，向**所有更深级别**（ℓ ≥ level_origin+1）
    /// 扫描身份桥（多级键域 `by_end_multi`；值桥 source_index==seg_c_full.1、side 同向
    /// 语义与旧路径逐字一致）。级别下限 = level_origin+1：跨级链从买卖点次级别往下
    /// 递归合法（ADR adr-level-identity-multiview-20260721 裁定 2），不含本级与更浅级。
    ///
    /// **因果守卫**：只用锚定 bar 因果前缀内的确认事件——证书链上全部首次可证钟
    /// `judge_at`（高→低含基例）都 ≤ `anchor_index`，任一越界即整证剔除（不引入未来函数）。
    ///
    /// 判定谓词唯一来源不变（`cert.certificate().n_delta()`，nest.rs 递归核，禁第二查法）；
    /// 本路径只改查询方向。返回谱系（命中在哪个级别）供 #112 消费与 miss 归因；
    /// 无任何级别命中 ⟹ None。
    pub(super) fn typed_lookup_multi(
        &self,
        level_origin: usize,
        source_index: usize,
        delta: super::super::types::Side,
        anchor_index: usize,
    ) -> Option<MultiLevelTypedLookup> {
        let ids = self.by_end_multi.get(&(
            source_index,
            matches!(delta, super::super::types::Side::Long),
        ))?;
        let mut per_level: std::collections::BTreeMap<u32, (bool, usize)> =
            std::collections::BTreeMap::new();
        for id in ids {
            // 级别桥：只向更深级别扫描（ℓ ≥ level_origin+1）。
            if (id.level as usize) <= level_origin {
                continue;
            }
            let Some(cert) = self.index.get(id) else { continue };
            // 因果守卫：链上任一确认钟越过锚定 bar ⟹ 整证不可用。
            if cert.judge_at().iter().any(|&t| t > anchor_index) {
                continue;
            }
            let pass = cert.certificate().n_delta();
            let rungs = cert.judge_at().len() - 1;
            // 级内 T7 合并（布尔 + 最深，旧路径同款）；级间保留谱系。
            per_level
                .entry(id.level)
                .and_modify(|(p, r)| {
                    *p = *p || pass;
                    *r = (*r).max(rungs);
                })
                .or_insert((pass, rungs));
        }
        if per_level.is_empty() {
            return None;
        }
        let hits: Vec<MultiLevelTypedHit> = per_level
            .into_iter()
            .map(|(level, (pass, rungs))| MultiLevelTypedHit { level, pass, rungs })
            .collect();
        let merged = hits.iter().fold((false, 0usize), |(p, r), h| {
            (p || h.pass, r.max(h.rungs))
        });
        Some(MultiLevelTypedLookup { merged, hits })
    }

    /// 门开单候选裁决：**multi typed 真链为唯一 nest 判定源**（`typed_lookup_multi` →
    /// `n_delta`）；multi 无证时回退旧臂 Xzd 通道（Xzd 无 typed 对应物，单一来源 =
    /// [`super::econ_positive::build_xzd_fallback`]，econ_positive.rs:1606）。
    /// 旧 fixed `typed_lookup` 仅落 single/multi comparison；L2 旧臂（`nest_gate_admit` →
    /// `build_gate_certificate`）恒双读落账供既有 cross 对照。
    ///
    /// typed 无证时的回退与旧臂「Nest None」分支同语义，两种消费方式**不等价、不可统一**
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
        // 旧 fixed ℓ=lvl+1 路径只作 comparison-only 读出，不参与判定。
        let single = self.typed_lookup(lvl, c.source_index, delta);
        // #112 entry 唯一 nest 判定源：multi 投影查询；anchor=当前确认 bar（因果守卫）。
        let typed = self.typed_lookup_multi(lvl, c.source_index, delta, confirm_index);
        // L2 旧臂对照读出（multi 命中时不参与 nest 判定；multi 无证时其 Xzd 通道被复用）。
        let (old_admit, old_channel) = nest_gate_admit(tower, c, hist, confirm_index, classification);
        let (admit, channel, xzd_fallback, reused_old_xzd) = match typed.as_ref() {
            Some(result) => {
                let pass = result.merged.0;
                (pass, if pass { "nest_pass" } else { "nest_n_delta_false" }, false, false)
            }
            None => match old_channel {
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
            typed_rungs: typed.as_ref().map(|result| result.merged.1),
            typed_hit_levels: typed
                .as_ref()
                .map(|result| result.hits.iter().map(|hit| hit.level).collect())
                .unwrap_or_default(),
            single_admit: single.map(|(pass, _)| pass),
            multi_admit: typed.as_ref().map(|result| result.merged.0),
            old_admit,
            xzd_fallback,
            reused_old_xzd,
        };
        (admit, channel, obs)
    }
}

// ═════════════════ #76（SPEC #73 A 线第三票，出场门真链切换）═════════════════
//
// 出场反向项从 v0 基例升格为 typed 真链反查：门开时 v1/dual 退出循环持有
// [`ExitNestGateCtx`]——逐 bar 前缀因果喂法（与 π 进场门同一装配源、同一身份桥、
// 同一喂法纪律，禁第二查法），反向候选经 `exit_decision_for_nested_cert` 注入的
// 查询闭包反查 typed 证书并以 `n_delta()` 判定；反查 miss ⟹ 诚实不准出（出场侧
// 无 Xzd 对应物，Xzd 回退只服务进场）；v0 基例 `reverse_nest_cert_base` 退役为
// 对照读出（双读落账 NEST_GATE_EXIT，判定不消费）。

/// #76 出场侧真链门统计（NEST_GATE_EXIT 行）：反向准出率 + 链深构成 + v0 对照差。
///
/// 计数域 = **depth-0 反向信号候选**（反向项有效域；depth>0 ShortDiff 腿的反向确认
/// 由 F1 段终点锚替代，其查询读出既不进判定也不进统计）。cross 口径同 #94：两路
/// **独立**判定才入对照差——出场侧无复用通道（Xzd 不服务出场），无自证成分可剔除。
#[derive(Default)]
pub(super) struct ExitNestGateStats {
    /// depth-0 反向信号候选查询总数。
    pub(super) total: usize,
    /// 真链 n_delta 通过（准出）。
    pub(super) admitted: usize,
    /// 身份桥命中（有 typed 证书）。
    pub(super) typed_found: usize,
    /// 身份桥反查 miss（⟹ 诚实不准出）。
    pub(super) typed_none: usize,
    /// Flat 方向候选（无方向 ⟹ 无证书，拒——与 v0 基例同语义）。
    pub(super) flat_dir: usize,
    /// 真链命中证书的链深构成（准入侧 / 拒绝侧）：rungs 0 / 1 / ≥2。
    pub(super) admit_rungs: [usize; 3],
    pub(super) rej_rungs: [usize; 3],
    /// v0 基例对照读出（双读落账，判定不消费）：两路一致 / v0 准真链拒 / v0 拒真链准。
    pub(super) cross_agree: usize,
    pub(super) cross_v0_pass_typed_rej: usize,
    pub(super) cross_v0_rej_typed_pass: usize,
}

impl ExitNestGateStats {
    pub(super) fn observe(&mut self, typed: Option<(bool, usize)>, v0: bool, pass: bool, flat: bool) {
        self.total += 1;
        if flat {
            self.flat_dir += 1;
        } else {
            match typed {
                Some((p, rungs)) => {
                    self.typed_found += 1;
                    let bucket = rungs.min(2);
                    if p {
                        self.admit_rungs[bucket] += 1;
                    } else {
                        self.rej_rungs[bucket] += 1;
                    }
                }
                None => self.typed_none += 1,
            }
        }
        if pass {
            self.admitted += 1;
        }
        match (v0, pass) {
            (true, true) | (false, false) => self.cross_agree += 1,
            (true, false) => self.cross_v0_pass_typed_rej += 1,
            (false, true) => self.cross_v0_rej_typed_pass += 1,
        }
    }

    /// NEST_GATE_EXIT 行（门开时 v1/dual 退出循环末尾各打一行；门关恒不输出——
    /// 诊断读出与 NEST_GATE_STATS 同 env 惯例，不进任何判定）。
    pub(super) fn report(&self, path: &str) {
        eprintln!(
            "NEST_GATE_EXIT path={} total={} admitted={} rejected={} | typed_found={} typed_none={} flat_dir={} | admit_rungs r0={} r1={} r2+={} | rej_rungs r0={} r1={} r2+={} | cross agree={} v0_pass_typed_rej={} v0_rej_typed_pass={}",
            path,
            self.total,
            self.admitted,
            self.total - self.admitted,
            self.typed_found,
            self.typed_none,
            self.flat_dir,
            self.admit_rungs[0],
            self.admit_rungs[1],
            self.admit_rungs[2],
            self.rej_rungs[0],
            self.rej_rungs[1],
            self.rej_rungs[2],
            self.cross_agree,
            self.cross_v0_pass_typed_rej,
            self.cross_v0_rej_typed_pass,
        );
    }
}

/// #76 出场侧真链门 ctx：门开时 v1/dual 退出循环持有——**方案 (b) 预注入**（实装卡
/// §关键障碍）：把 #75 `NestChainGate` 的 typed_lookup 查询能力注入退出循环，
/// `exit_decision_impl` 只消费注入的查询结果，不穿参整个塔。
///
/// **同一装配源（禁第二查法）**：事件派生/账本/索引全部复用 [`NestChainGate`] 机器；
/// 反查 = [`NestChainGate::typed_lookup`]（级别桥 ℓ=lvl+1、值桥 `source_index == seg_c_full.1`、
/// side 同向——#75 确证身份桥，turn_source 等值桥已证否）；判定 =
/// `cert.certificate().n_delta()`（nest.rs 递归核单一来源）。
///
/// **因果纪律**：`sync_bar` 每 bar 经 `IncrementalClassifier::classify_at(i)` 取前缀因果
/// （分类, 塔）喂 `sync_events`（judge_at 首次观察不后移）——与 π 层进场门
/// （runner.rs:2215 一带）同款喂法；索引惰性重建（本 bar 候选无一命中身份桥键 ⟹
/// `typed_lookup` 必 None ⟹ 不重建，决策逐字节不变）。v1 路径注：decisions 来自上游
/// 全窗 recognize（deprecated F-01 口径不动），但**门喂法**是逐 bar 因果前缀——
/// 出场准入判定不新增前视分量。
///
/// **出场侧口径**：反查 miss ⟹ 诚实不准出（**不得** fallback 到 v0 基例判定——v0 只进
/// 对照读出）；Flat 候选 ⟹ 无方向无证书，拒（与 v0 基例 `nest_confirm` 同语义）。
pub(super) struct ExitNestGateCtx<'a> {
    pub(super) classifier: super::incremental::IncrementalClassifier<'a>,
    pub(super) gate: NestChainGate,
    pub(super) stats: ExitNestGateStats,
}

impl<'a> ExitNestGateCtx<'a> {
    /// 门开构建：IncrementalClassifier（逐 bar 前缀因果源）+ NestChainGate（hist/dif/
    /// close_src 全序列一次性构建，EMA/合并因果 ⟹ 前缀安全，见 NestChainGate 文档）。
    pub(super) fn new(bars: &'a [Bar], config: &'a ThetaConfig) -> Self {
        ExitNestGateCtx {
            classifier: super::incremental::IncrementalClassifier::new(bars, config),
            gate: NestChainGate::new(bars, config),
            stats: ExitNestGateStats::default(),
        }
    }

    /// 每 bar 前缀喂法（退出决策段开头调用）：classify_at(i) → sync_events → 本 bar
    /// 候选命中身份桥键时 sync_index（惰性重建判据与 π 层 :2219-2231 同款）。
    pub(super) fn sync_bar(&mut self, i: usize, candidates: &[&super::super::strategy::VoiceDecision]) {
        use super::super::strategy::voice::{voice_side, VoiceSide};
        let (classification_i, tower_i) = self.classifier.classify_at(i);
        let confirmed_lens = self.classifier.tower_confirmed_lens(tower_i.len());
        self.gate.sync_events(&tower_i, &confirmed_lens, i);
        let needs_index = candidates.iter().any(|d| {
            let delta = match voice_side(d.root_side, d.depth) {
                VoiceSide::Long => super::super::types::Side::Long,
                VoiceSide::Short => super::super::types::Side::Short,
                VoiceSide::Flat => return false,
            };
            self.gate.has_bridge_key(d.level as usize, d.signal_index, delta)
        });
        if needs_index {
            self.gate.sync_index(&classification_i);
        }
    }

    /// 反向候选准入查询（注入 `exit_decision_for_nested_cert` 的闭包本体）：
    /// 身份桥反查命中 ⟹ `n_delta` 判定；miss ⟹ false（诚实不准出，禁 v0 fallback）。
    /// v0 基例双读落账为对照读出（仅 depth=0 有效域——depth>0 的反向项已被 F1 锚
    /// 替代，其读出弃用，不进判定也不进统计）。
    pub(super) fn reverse_admit(&mut self, d: &super::super::strategy::VoiceDecision, depth: usize) -> bool {
        use super::super::strategy::voice::{voice_side, VoiceSide};
        use super::super::types::Side;
        let delta = match voice_side(d.root_side, d.depth) {
            VoiceSide::Long => Side::Long,
            VoiceSide::Short => Side::Short,
            VoiceSide::Flat => {
                if depth == 0 {
                    let v0 = super::super::strategy::exit::reverse_nest_cert_base(d);
                    self.stats.observe(None, v0, false, true);
                }
                return false; // Flat：无方向 ⟹ 无证书（与 v0 基例同语义）
            }
        };
        let typed = self.gate.typed_lookup(d.level as usize, d.signal_index, delta);
        let pass = typed.map(|(p, _)| p).unwrap_or(false); // nest-None = 不准出
        if depth == 0 {
            let v0 = super::super::strategy::exit::reverse_nest_cert_base(d); // 对照读出
            self.stats.observe(typed, v0, pass, false);
        }
        pass
    }

    /// NEST_GATE_EXIT 行（门开时 v1/dual 退出循环末尾各打一行）。
    pub(super) fn report(&self, path: &str) {
        self.stats.report(path);
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
