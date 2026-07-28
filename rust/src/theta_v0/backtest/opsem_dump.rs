//! ★opsem-dump seam（B-M5，#85）：严格区间套证书 sidecar + env-gated 操作语义 dump，
//! 自 runner.rs 纯移动（设计 chanlun/plans/runner-rs-seam-designs-20260721.md §M5）。
//!
//! 公共类型经 `runner` 门面 `pub use` 保持原路径（`runner::StrictNestSidecarSummary`）。

use super::super::config::ThetaConfig;
use super::super::{classifier, parser, strategy};
use super::runner::{LedgerOpen, TypedTrade};

/// ★#466 D0：fill 生产事务交给 OPSEM writer 的只读重基现场。
///
/// `chain` 由事件机在 adopt 覆盖前抓取；`suspended_before` 必须在
/// `CenterOscillationBook::on_chain_rebase` 核销 vanished 条目前抓取。类型本身不参与任何
/// 决策，只在 [`OpsemDump`] 已由 `OPSEM_DUMP_DIR` 构造时产生。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RebaseObservationInput {
    pub(super) level: u32,
    pub(super) chain: classifier::center_lifecycle::ChainRebaseObservation,
    pub(super) suspended_before:
        Vec<(strategy::voice::VoiceSide, classifier::center_lifecycle::CenterId)>,
    pub(super) revived: bool,
}

fn center_id_json(id: classifier::center_lifecycle::CenterId) -> serde_json::Value {
    serde_json::json!({
        "start_index": id.start_index,
        "zd": id.zd,
        "zg": id.zg,
    })
}

fn center_ids_json(
    ids: impl IntoIterator<Item = classifier::center_lifecycle::CenterId>,
) -> Vec<serde_json::Value> {
    ids.into_iter().map(center_id_json).collect()
}

fn center_id_counts(
    ids: &[classifier::center_lifecycle::CenterId],
) -> std::collections::BTreeMap<classifier::center_lifecycle::CenterId, usize> {
    let mut counts = std::collections::BTreeMap::new();
    for id in ids {
        *counts.entry(*id).or_insert(0) += 1;
    }
    counts
}

/// ★#466 D0：只把**精确 CenterId 相等**记为 continued。CenterId 缺席时，当前 seam 没有
/// 稳定谱系/构造变换边，故一律记 ambiguous；同 start/core 只落 `non_lineage_hints`，绝不据此
/// 自动判 continued/split/genuinely_removed。
fn rebase_mapping_json(
    old: classifier::center_lifecycle::CenterId,
    before_counts: &std::collections::BTreeMap<
        classifier::center_lifecycle::CenterId,
        usize,
    >,
    after_counts: &std::collections::BTreeMap<
        classifier::center_lifecycle::CenterId,
        usize,
    >,
    added: &std::collections::BTreeSet<classifier::center_lifecycle::CenterId>,
    source_occurrences_override: Option<usize>,
) -> serde_json::Value {
    let before_occurrences = source_occurrences_override
        .unwrap_or_else(|| before_counts.get(&old).copied().unwrap_or(0));
    let after_occurrences = after_counts.get(&old).copied().unwrap_or(0);
    let functional = after_occurrences == 1;
    let injective = after_occurrences > 0 && before_occurrences == 1;
    let unique = functional && injective;
    let result = if unique { "continued" } else { "ambiguous" };
    let mapped_new_ids =
        if after_occurrences > 0 { center_ids_json([old]) } else { Vec::new() };
    let same_start_added: Vec<_> =
        added.iter().copied().filter(|id| id.start_index == old.start_index).collect();
    let same_core_added: Vec<_> =
        added.iter().copied().filter(|id| id.zd == old.zd && id.zg == old.zg).collect();
    let possible_results: Vec<&str> = if unique {
        Vec::new()
    } else {
        vec!["continued", "split", "genuinely_removed"]
    };

    serde_json::json!({
        "old_center": center_id_json(old),
        "mapped_new_ids": mapped_new_ids,
        "mapping_result": result,
        "evidence": if unique { "exact_center_id" } else { "no_exact_lineage_witness" },
        "possible_results": possible_results,
        "left_degree": after_occurrences,
        "right_degree": before_occurrences,
        "functional": functional,
        "injective": injective,
        "unique": unique,
        "bijective": unique,
        "non_lineage_hints": {
            "same_start_added_count": same_start_added.len(),
            "same_start_added_ids": center_ids_json(same_start_added),
            "same_core_added_count": same_core_added.len(),
            "same_core_added_ids": center_ids_json(same_core_added),
        }
    })
}

/// 严格区间套证书记录（P3 sidecar）：目标级 `top_level` 的一条 `N^δ_{ℓ↓0}` 证书。
#[derive(Debug, Clone, PartialEq)]
pub struct StrictNestCertificateRecord {
    pub top_level: usize,
    pub certificate: classifier::nest::NestCertificate,
}

/// 严格区间套 sidecar 的末帧汇总；口径复用 P2 `nest.rs::assemble_certificates`。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StrictNestSidecarSummary {
    /// 已观察生产 replay 帧数。
    pub frames: usize,
    /// 末帧 ℓ0 `cand_delta=true` 基例数。
    pub base_count: usize,
    /// 末帧 terminal 查无次数（P1 一致性推论下应为 0）。
    pub terminal_missing: usize,
    /// 末帧每个目标级的证书数。
    pub cert_per_top: Vec<(usize, usize)>,
    /// 末帧证书总数。
    pub cert_total: usize,
    /// 末帧证书流内容（预期极稀；P2 当前 BTC 全量为 0）。
    pub certificates: Vec<StrictNestCertificateRecord>,
}

pub(super) struct StrictNestSidecarCollector {
    pub(super) enabled: bool,
    summary: StrictNestSidecarSummary,
}

impl StrictNestSidecarCollector {
    pub(super) fn new(enabled: bool) -> Self {
        Self { enabled, summary: StrictNestSidecarSummary::default() }
    }

    pub(super) fn observe_frame(
        &mut self,
        l0: &parser::ParseLayer,
        classification: &classifier::Classification,
        tower: &[std::rc::Rc<Vec<classifier::recursive_tower::LeveledMove>>],
        config: &ThetaConfig,
        cache: &classifier::TowerCache,
    ) {
        if !self.enabled {
            return;
        }

        let cand = classifier::cand_delta_tower_cached(l0, classification, tower, config, cache);
        let mut terminal_by_key = std::collections::HashMap::new();
        if let Some(l0_level) = classification.levels.first() {
            for p in l0_level.bsp.iter() {
                if p.bits.buy1 {
                    terminal_by_key.entry((p.source_index, 1i8)).or_insert(p.bits);
                }
                if p.bits.sell1 {
                    terminal_by_key.entry((p.source_index, -1i8)).or_insert(p.bits);
                }
            }
        }
        let frames = self.summary.frames + 1;
        let objects_by_level: Vec<&[classifier::recursive_tower::CpScanOwnership]> = classification
            .levels
            .iter()
            .map(|state| state.cp_ownership.as_slice())
            .collect();
        self.summary =
            summarize_strict_nest_certificates(&cand, &objects_by_level, &terminal_by_key);
        self.summary.frames = frames;
    }

    pub(super) fn finish(self) -> Option<StrictNestSidecarSummary> {
        self.enabled.then_some(self.summary)
    }
}

fn strict_nest_side_i8(side: super::super::types::Side) -> i8 {
    match side {
        super::super::types::Side::Long => 1,
        super::super::types::Side::Short => -1,
    }
}

pub(super) fn summarize_strict_nest_certificates(
    cand: &[Vec<classifier::recursive_tower::CandDeltaEvent>],
    objects_by_level: &[&[classifier::recursive_tower::CpScanOwnership]],
    terminal_by_key: &std::collections::HashMap<(usize, i8), super::super::types::BspBits>,
) -> StrictNestSidecarSummary {
    let mut summary = StrictNestSidecarSummary {
        base_count: cand.first().map(|evs| evs.iter().filter(|e| e.cand_delta).count()).unwrap_or(0),
        ..StrictNestSidecarSummary::default()
    };
    // F-06：terminal_missing 以“唯一 L0 基例”计数——此前在每个 top 的装配回调里累加，
    // 同一缺失基例会按可用 top 数重复计入。
    summary.terminal_missing = cand
        .first()
        .map(|evs| {
            evs.iter()
                .filter(|e| e.cand_delta)
                .filter(|e| {
                    !terminal_by_key.contains_key(&(e.confirm_src, strict_nest_side_i8(e.side)))
                })
                .count()
        })
        .unwrap_or(0);
    for top in 1..cand.len() {
        let certs = classifier::nest::assemble_certificates_terminal(
            cand,
            objects_by_level,
            0,
            top,
            |base| {
                terminal_by_key
                    .get(&(base.confirm_src, strict_nest_side_i8(base.side)))
                    .copied()
            },
        );
        summary.cert_per_top.push((top, certs.len()));
        summary.certificates.extend(certs.into_iter().map(|certificate| StrictNestCertificateRecord {
            top_level: top,
            certificate,
        }));
    }
    summary.cert_total = summary.certificates.len();
    summary
}

pub(super) fn strict_nest_sidecar_enabled() -> bool {
    std::env::var("THETA_STRICT_NEST_SIDECAR")
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON"))
        .unwrap_or(false)
}


// ─────────────────────────────────────────────────────────────────────────────
//  ★opsem-dump（基因 073a/274号 谱系）：只读语义快照 dump——env-gated，零生产语义改动。
//
//  触发：env `OPSEM_DUMP_DIR=<dir>`（如 /tmp/opsem）。**未设 ⟹ 全部方法 no-op**，
//  生产路径与既有测试逐字节不变（bit-exact，同 `dump_deltafree_pertrade`/`kappa_policy_resolved`
//  先例）。dump 不进 μ 桶键/J_Θ 排序/χ 门控——纯只读外化（no-patch-mentality：诊断切片
//  不冒充裁决层，铁律 exit-μ-BUCKETING-FROZEN #180 同款约束）。
//
//  产物（落 `<dir>/trades.jsonl` + `<dir>/tower_events.jsonl`）：
//  - trades.jsonl：每笔 TypedTrade 一行 JSON（entry/exit 字段 + 触发证书 + 解释器状态 +
//    声部树快照 + 背驰判定输入 + TW 阶段）。缺席字段标 null（不许编造）。
//  - tower_events.jsonl：bar 级塔事件（中枢新建/延伸/升级/破坏 + 级别 + bar 号），仅交易
//    活跃区间（首入场 bar .. 末离场 bar）。
//
//  认识论等级（formalization-validity-domain 231号）：L1（纯只读外化，零信息增量）。
//  字段缺口标注原则（no-patch-mentality + result-package 六要素）——R5（2026-07-05）后状态：
//  - 「LexArgmin 选中的与被拒的前 3 名 J_Θ 排序键」（R5-a 已实装）：`intent::lex_argmin_top_k`
//    在 `coverage::feasible_lex_candidates`（与 `pi_theta_position` 同源候选集）上稳定排序取前 3，
//    经 `StepTrace.lex_top3` → `OpsemEntrySnapshot.lex_top3` 透传至 `lex_argmin_top3` 字段
//    （`[{control,key:{5维}},...]`，首名 = p_star 选址）。不进 p_star/J_Θ/χ（R5-1 铁律）。
//  - 「区间套深度 nest_depth」（R5-c 已实装）：`econ_positive::structural_nest_depth`（与生产门
//    `build_nest_certificate` rungs 构造同款 partition_point，不依赖 hist）读 rungs.len()，
//    写入 `OpsemEntrySnapshot.nest_depth`（**不进 entry_z/MuClass/μ 桶键**——R5-1 铁律，避免
//    MuClass derive Hash 的 nest_depth 字段破坏 μ 分桶 bit-exact）。
//  - 「中枢破坏事件」：前缀因果塔单调增长（prefix classification 不删结构），**无破坏概念**
//    ⟹ tower_events.jsonl 不输出 destroy 事件（诚实缺席，不伪造）。
// ─────────────────────────────────────────────────────────────────────────────
pub(super) struct OpsemDump {
    trades_buf: std::io::BufWriter<std::fs::File>,
    tower_buf: std::io::BufWriter<std::fs::File>,
    /// ★#291（SPEC #274 T1）：中枢生命周期事件流 `center_lifecycle.jsonl`（born/broken/reset +
    /// resync 工程诊断行）。与 trades/tower_events 并列第三产物——同一 env 门（OPSEM_DUMP_DIR
    /// 未设 ⟹ 本写入器不存在，feed 不执行，生产路径 bit-exact 不变）；事件只外化不回馈决策。
    center_lifecycle_buf: std::io::BufWriter<std::fs::File>,
    /// ★#466 D0：重基事务现场 `rebase_observability.jsonl`。与其余 OPSEM 产物同一 env 门；
    /// 仅接收 fill 已完成的只读事务快照，不回馈事件机、挂起表或订单路径。
    rebase_observability_buf: Box<dyn std::io::Write>,
    /// 本次 dump 内全局单调重基序号（跨 bar/level，首条=1）。
    rebase_seq: u64,
    /// 重基观测 I/O 失败 witness。失败后关闭该旁路，避免部分行后的候选序号被重试复用；
    /// 只影响观测，不参与交易、挂起或核销决策。
    rebase_observability_failed: bool,
    rebase_observability_failure_count: u64,
    rebase_observability_failure_reported: bool,
    trade_id_counter: u64,
    /// 交易活跃区间（首入场 bar .. 末离场 bar）；None=尚未见入场。
    active_start: Option<usize>,
    active_end: Option<usize>,
    /// 上一 bar 的塔（仅交易活跃区间内 diff，O(n) per bar）。
    prev_tower: Option<Vec<std::rc::Rc<Vec<classifier::recursive_tower::LeveledMove>>>>,
    /// ★#291：每级一台中枢生命周期事件机（下标=级别）。★#336 R3：机器改为塔链消费器 ⟹
    /// 旧的 `cl_fed_units`（每级 units 投影缓存）随独立复算路径一并删除（谱系注记见
    /// `feed_center_lifecycle` 模块头「已作废」节）。
    cl_machines: Vec<classifier::center_lifecycle::CenterEventMachine>,
    /// ★#291：工程再同步累计（级消失 / ★#336 R3 链前缀分叉重基 ⟹ 该级重同步；非教义生死，
    /// 照实单列）。
    cl_resync_total: u64,
    /// ★#329：误杀拒绝累计（触发点载体身份 ≠ 在场中枢身份 ⟹ 事件机拒杀，落 `kind:"miskill"`）。
    cl_miskill_total: u64,
}

#[cfg(test)]
thread_local! {
    /// 测试注入点：Some(dir) ⟹ **本线程**的 fill loop 启用 opsem dump。进程级 env 会被并行
    /// 测试的 [`OpsemDump::from_env`] 同时读到并 truncate 同一 trades.jsonl（2026-07-13 全量
    /// 回归竞态实录：0 笔交易的合成测试把 R5 测试的 dump 清空）；线程局部对并行测试不可见。
    pub(super) static OPSEM_DUMP_DIR_OVERRIDE: std::cell::RefCell<Option<std::path::PathBuf>> =
        std::cell::RefCell::new(None);
}

impl OpsemDump {
    /// 从 env `OPSEM_DUMP_DIR` 构造；未设 ⟹ None（零开销）。测试经线程局部
    /// [`OPSEM_DUMP_DIR_OVERRIDE`] 注入（优先于 env；生产构建不含该分支）。
    pub(super) fn from_env() -> Option<Self> {
        #[cfg(test)]
        {
            if let Some(dir) = OPSEM_DUMP_DIR_OVERRIDE.with(|c| c.borrow().clone()) {
                return Self::at_dir(&dir);
            }
        }
        let dir = std::env::var("OPSEM_DUMP_DIR").ok().filter(|s| !s.is_empty())?;
        Self::at_dir(std::path::Path::new(&dir))
    }

    /// 在指定目录开 OPSEM JSONL 写入器。
    /// ponytail: 截断打开（每次回测重写；同 dump_deltafree_pertrade 落盘语义）。path 局部化——
    /// struct 只持 BufWriter（path 仅 create 时用，后续不读，YAGNI 不存字段）。
    fn at_dir(dir_path: &std::path::Path) -> Option<Self> {
        std::fs::create_dir_all(dir_path).ok()?;
        let trades_file = std::fs::File::create(dir_path.join("trades.jsonl")).ok()?;
        let tower_file = std::fs::File::create(dir_path.join("tower_events.jsonl")).ok()?;
        // ★#291：第三产物（同截断语义——每次回测重写）。
        let cl_file = std::fs::File::create(dir_path.join("center_lifecycle.jsonl")).ok()?;
        // ★#466 D0：第四产物（同截断语义）。
        let rebase_path = dir_path.join("rebase_observability.jsonl");
        let rebase_file = match std::fs::File::create(&rebase_path) {
            Ok(file) => file,
            Err(error) => {
                eprintln!(
                    "#466 D0 重基观测初始化失败：{rebase_path:?}：{error}；前三个 OPSEM 产物已按截断语义创建，本次 dump 关闭（交易决策 fail-open）"
                );
                return None;
            }
        };
        Some(Self {
            trades_buf: std::io::BufWriter::new(trades_file),
            tower_buf: std::io::BufWriter::new(tower_file),
            center_lifecycle_buf: std::io::BufWriter::new(cl_file),
            rebase_observability_buf: Box::new(std::io::BufWriter::new(rebase_file)),
            rebase_seq: 0,
            rebase_observability_failed: false,
            rebase_observability_failure_count: 0,
            rebase_observability_failure_reported: false,
            trade_id_counter: 0,
            active_start: None,
            active_end: None,
            prev_tower: None,
            cl_machines: Vec::new(),
            cl_resync_total: 0,
            cl_miskill_total: 0,
        })
    }

    /// 在入场 bar 标记交易活跃区间起点。
    pub(super) fn mark_entry(&mut self, bar: usize) {
        if self.active_start.is_none() {
            self.active_start = Some(bar);
        }
        self.active_end = Some(bar);
    }

    /// 在离场 bar 更新活跃区间末点。
    pub(super) fn mark_exit(&mut self, bar: usize) {
        self.active_end = Some(bar);
    }

    /// ★#466 D0：写一笔重基事务现场。调用点位于既有 `on_chain_rebase` 行为完成之后，但输入
    /// 已在其删除 vanished 挂起前冻结；本函数只序列化，不返回任何可供决策消费的值。
    fn write_rebase_observation(
        &mut self,
        bar: usize,
        input: &RebaseObservationInput,
    ) -> std::io::Result<()> {
        use classifier::center_lifecycle::CenterId;
        use std::collections::BTreeSet;
        use std::io::Write;

        let before_counts = center_id_counts(&input.chain.before);
        let after_counts = center_id_counts(&input.chain.after);
        let before_set: BTreeSet<CenterId> = input.chain.before.iter().copied().collect();
        let after_set: BTreeSet<CenterId> = input.chain.after.iter().copied().collect();
        let removed: BTreeSet<CenterId> = before_set.difference(&after_set).copied().collect();
        let added: BTreeSet<CenterId> = after_set.difference(&before_set).copied().collect();
        let exact: BTreeSet<CenterId> = before_set.intersection(&after_set).copied().collect();

        let identity_mappings: Vec<_> = input
            .chain
            .before
            .iter()
            .copied()
            .map(|old| rebase_mapping_json(old, &before_counts, &after_counts, &added, None))
            .collect();
        let continued_count = input
            .chain
            .before
            .iter()
            .filter(|old| {
                before_counts.get(old).copied() == Some(1)
                    && after_counts.get(old).copied() == Some(1)
            })
            .count();
        let ambiguous_count = input.chain.before.len().saturating_sub(continued_count);

        let mut suspended_before = Vec::with_capacity(input.suspended_before.len());
        let mut suspended_continued = 0usize;
        for (side, old) in input.suspended_before.iter().copied() {
            // 挂起表的 `(side, CenterId)` 键本身就是一个确定源项；它不要求旧身份仍在事件机
            // consumed 链中（#292 D 允许 superseded 历史中枢继续挂起）。
            let mut mapping =
                rebase_mapping_json(old, &before_counts, &after_counts, &added, Some(1));
            if mapping["mapping_result"] == "continued" {
                suspended_continued += 1;
            }
            mapping
                .as_object_mut()
                .expect("rebase_mapping_json 恒返回 object")
                .insert("side".into(), serde_json::json!(voice_side_str(side)));
            suspended_before.push(mapping);
        }
        let suspended_ambiguous =
            input.suspended_before.len().saturating_sub(suspended_continued);

        let before_ids_unique = before_counts.values().all(|count| *count == 1);
        let after_ids_unique = after_counts.values().all(|count| *count == 1);
        let exact_mapping_left_unique =
            exact.iter().all(|id| before_counts.get(id).copied() == Some(1));
        let exact_mapping_right_unique =
            exact.iter().all(|id| after_counts.get(id).copied() == Some(1));
        let exact_mapping_bijective =
            exact_mapping_left_unique && exact_mapping_right_unique;
        let complete_bijection = before_ids_unique
            && after_ids_unique
            && removed.is_empty()
            && added.is_empty()
            && input.chain.before.len() == input.chain.after.len();

        let rebase_seq = self.rebase_seq + 1;
        let row = serde_json::json!({
            "schema": "rebase_observability_d0_v1",
            "rebase_seq": rebase_seq,
            "bar": bar,
            "level": input.level,
            "guard_at": input.chain.at,
            "revived": input.revived,
            "before_chain_len": input.chain.before.len(),
            "after_chain_len": input.chain.after.len(),
            "before_ids": center_ids_json(input.chain.before.iter().copied()),
            "after_ids": center_ids_json(input.chain.after.iter().copied()),
            "removed_ids": center_ids_json(removed.iter().copied()),
            "added_ids": center_ids_json(added.iter().copied()),
            "suspended_at_rebase": input.suspended_before.len(),
            "suspended_before": suspended_before,
            "identity_mappings": identity_mappings,
            "mapping_result_domain": [
                "continued",
                "split",
                "genuinely_removed",
                "ambiguous"
            ],
            "mapping_result_counts": {
                "continued": continued_count,
                "split": 0,
                "genuinely_removed": 0,
                "ambiguous": ambiguous_count,
            },
            "suspended_result_counts": {
                "continued": suspended_continued,
                "split": 0,
                "genuinely_removed": 0,
                "ambiguous": suspended_ambiguous,
            },
            "checks": {
                "before_ids_unique": before_ids_unique,
                "after_ids_unique": after_ids_unique,
                "exact_mapping_count": exact.len(),
                "exact_mapping_left_unique": exact_mapping_left_unique,
                "exact_mapping_right_unique": exact_mapping_right_unique,
                "exact_mapping_bijective": exact_mapping_bijective,
                "complete_bijection": complete_bijection,
                "unmapped_old_count": removed.len(),
                "unmapped_new_count": added.len(),
            },
            "construction_witness": {
                "status": "none",
                "available": [
                    "old_chain_center_id(start_index,zd,zg)",
                    "new_chain_center_id(start_index,zd,zg)",
                    "exact_center_id_equality",
                    "non_lineage_same_start_and_same_core_hints"
                ],
                "missing": [
                    "stable_lineage_id",
                    "source_unit_ids",
                    "rebase_transform_edges",
                    "split_merge_generation_witness"
                ],
                "explanation": "当前 ChainConsumed/Classification seam 不暴露塔构造来源谱系或重基变换边；同 start/core 仅作提示，不据此实施或声明身份映射。"
            }
        });

        // 先在内存中形成含换行的完整 JSONL 行，再一次性交给 writer；只有整行写成功才提交
        // rebase_seq。若 writer 返回错误，fail-open 包装器会关闭本次 dump 的后续重基观测，
        // 因而不会用同一候选序号重试。
        let mut line = serde_json::to_vec(&row)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
        line.push(b'\n');
        self.rebase_observability_buf.write_all(&line)?;
        self.rebase_seq = rebase_seq;
        Ok(())
    }

    /// 生产调用入口：观测 I/O 失败只进入 witness 并关闭本实例的重基观测旁路，决策继续。
    pub(super) fn write_rebase_observation_fail_open(
        &mut self,
        bar: usize,
        input: &RebaseObservationInput,
    ) {
        if self.rebase_observability_failed {
            return;
        }
        let candidate_seq = self.rebase_seq.saturating_add(1);
        if let Err(error) = self.write_rebase_observation(bar, input) {
            self.rebase_observability_failed = true;
            self.report_rebase_observability_failure("write", candidate_seq, &error);
        }
    }

    fn report_rebase_observability_failure(
        &mut self,
        stage: &'static str,
        sequence: u64,
        error: &std::io::Error,
    ) {
        self.rebase_observability_failure_count += 1;
        if !self.rebase_observability_failure_reported {
            eprintln!(
                "#466 D0 重基观测 {stage} 失败：failure_count={} rebase_seq={sequence}：{error}；后续重基观测旁路关闭，交易决策继续（fail-open）",
                self.rebase_observability_failure_count
            );
            self.rebase_observability_failure_reported = true;
        }
    }

    /// 写一笔 trade JSONL 行。`t` 是 TypedTrade，`open.opsem` 是入场快照，`pnl` 是费前方向盈亏
    /// （(exit_px-entry_px)×delta，未扣费——生产 fee 已在 fill loop 扣，TypedTrade 不携 fee）。
    pub(super) fn write_trade(
        &mut self,
        t: &TypedTrade,
        open: &LedgerOpen,
        trigger_bsp_class: Option<u8>,
    ) -> std::io::Result<()> {
        use std::io::Write;
        self.trade_id_counter += 1;
        // 费前方向盈亏（delta=+1 Long / -1 Short）。
        let delta_sign: f64 = t.entry_z.delta as f64;
        let pnl_raw = delta_sign * (t.exit_px - t.entry_px);
        let o = &open.opsem;
        // ponytail: 显式 push_str 拼装 JSON——避免 format! 的 `{{`/`}}` 转义混乱（曾出引号 bug）。
        // 缺席字段写 null（JSON 标准缺席标注，不编造）。
        let mut s = String::with_capacity(1024);
        s.push('{');
        // 基本字段。
        s.push_str(&format!(
            "\"trade_id\":{},\"voice_id\":{{\"level\":{},\"ordinal\":{}}},",
            self.trade_id_counter, t.voice_id.level, t.voice_id.ordinal,
        ));
        // #262：generation 明文补打（hash64 混淆致 exit_type×级别×代际分桶缺代际轴，#153 终验B
        // 重跑实录）——纯追加字段，hash 字段原样保留，既有消费方不受影响。
        // generation 语义（interp.rs:298）：同 carrier 顺序 campaign 单调递增的高水位代次
        // （close→reopen +1，首 campaign=0），**不是**父子嵌套代数。
        s.push_str(&format!(
            "\"position_node_id\":{},\"generation_plain\":{},\"entry_bar\":{},\"exit_bar\":{},\"entry_px\":{},\"exit_px\":{},\"pnl_raw_unlevered\":{},\"exit_type\":\"{}\",\"via_structural_prune\":{},\"units\":{},",
            t.position_node_id.hash64(), t.position_node_id.generation, t.entry_bar, t.exit_bar,
            t.entry_px, t.exit_px, pnl_raw,
            exit_type_str(t.exit_type), t.via_structural_prune, t.units,
        ));
        // 触发证书。
        s.push_str("\"certificate\":{");
        s.push_str(&format!(
            "\"bsp_bits_class_index\":{},\"bsp_class_min\":{},\"level\":{},\"source_index\":{},\"dir\":\"{}\",\"delta\":{},\"nest_confirmed\":{},\"nest_depth\":{},\"parent_dir_sigma_p\":{},\"role\":\"{}\"}},",
            o.cand_bits,
            if o.cand_bsp_class == u8::MAX { -1 } else { o.cand_bsp_class as i64 },
            o.cand_level, o.cand_source_index, o.cand_dir, t.entry_z.delta,
            // R5-c：nest_depth 改读 opsem_snap（structural_nest_depth 纯结构读数），不读 entry_z
            // （π 路径 entry_z.nest_depth 恒 None——MuClass 进 μ 桶键，填 Some 破坏 bit-exact）。
            o.cand_nest_confirmed, o.nest_depth,
            t.entry_z.parent_dir, o.cand_role,
        ));
        // 入场时刻解释器状态。
        s.push_str("\"interpreter_at_entry\":{");
        s.push_str(&format!(
            "\"gamma_count_chi_filtered\":{},\"prev_active_count\":{},\"lex_argmin_top3\":{}",
            o.gamma_count, o.prev_active_count, lex_top3_json(&o.lex_top3),
        ));
        s.push_str("},");
        // 声部树快照。
        s.push_str("\"voice_tree_at_entry\":{");
        s.push_str(&format!(
            "\"parent_id\":{},\"is_boundary_root_absent\":{},\"active_count_inclusive\":{}}},",
            opt_pair_str(o.parent_id), o.parent_id.is_none(),
            o.prev_active_count.saturating_add(1),
        ));
        // 背驰判定输入。
        s.push_str("\"divergence_input\":{");
        s.push_str(&format!(
            "\"seg_a_macd_area\":{},\"seg_c_macd_area\":{},\"seg_a_dif_peak\":{},\"seg_c_dif_peak\":{},\"force_state_weak_judgment\":{}}},",
            opt_f64_str(o.seg_a_macd_area), opt_f64_str(o.seg_c_macd_area),
            opt_f64_str(o.seg_a_dif_peak), opt_f64_str(o.seg_c_dif_peak),
            opt_str_quoted(o.force_state),
        ));
        // 入场时刻 TW 阶段。
        s.push_str("\"tw_at_entry\":{");
        s.push_str(&format!(
            "\"stage\":\"{}\",\"eta_bucket\":\"{}\",\"risk_mode\":\"{}\"}},",
            o.t_stage, o.eta_bucket, o.risk_mode,
        ));
        // 出场侧 + 收尾。
        s.push_str(&format!(
            "\"entry_stop_dist\":{},\"trigger_bsp_class_at_exit\":{},\"exit_z_t_stage\":{}}}\n",
            opt_f64_str(t.entry_stop_dist),
            opt_u8_str(trigger_bsp_class),
            opt_str_quoted(t.exit_z.t_stage.map(t_stage_str)),
        ));
        self.trades_buf.write_all(s.as_bytes())?;
        Ok(())
    }

    /// 写一个塔事件 JSONL 行（仅交易活跃区间）。
    fn write_tower_event(
        &mut self,
        bar: usize,
        level: u32,
        kind: &str,
        detail: &str,
    ) -> std::io::Result<()> {
        use std::io::Write;
        let active = match (self.active_start, self.active_end) {
            (Some(s), _) if bar < s => false,
            (_, Some(_e)) => true,
            _ => false,
        };
        if !active {
            return Ok(());
        }
        let json = format!(
            "{{\"bar\":{bar},\"level\":{lvl},\"kind\":\"{kind}\",\"detail\":\"{detail}\"}}\n",
            bar = bar,
            lvl = level,
            kind = kind,
            detail = detail.replace('\\', "\\\\").replace('"', "\\\""),
        );
        self.tower_buf.write_all(json.as_bytes())?;
        Ok(())
    }

    /// 在每 bar 调用：diff tower_i vs self.prev_tower，输出新建/延伸/升级事件（仅活跃区间）。
    /// `destroy` 事件缺席——前缀因果塔单调增长（prefix classification 不删结构）。
    pub(super) fn diff_tower(
        &mut self,
        bar: usize,
        tower_i: &[std::rc::Rc<Vec<classifier::recursive_tower::LeveledMove>>],
    ) {
        use classifier::descend::RMove;
        use classifier::recursive_tower::LeveledMove;
        // 仅在交易活跃区间内 diff（避免 O(n) per-bar 全窗扫描）。
        let in_active = match (self.active_start, self.active_end) {
            (Some(s), _) if bar >= s => true,
            _ => false,
        };
        if !in_active {
            self.prev_tower = Some(tower_i.to_vec());
            return;
        }
        let prev = self.prev_tower.take();
        match prev {
            None => {
                // 首个活跃 bar：所有 Compose 都作 "new_center"。
                for (lvl, moves) in tower_i.iter().enumerate() {
                    for m in moves.iter() {
                        if let RMove::Compose { centers, .. } = &m.rmove {
                            if let Some(c) = centers.first() {
                                let _ = self.write_tower_event(
                                    bar,
                                    lvl as u32,
                                    "new_center",
                                    &format!(
                                        "L{} #{} zd={} zg={} si={} ei={}",
                                        lvl, m.id.ordinal, c.zd, c.zg, m.start_index, m.end_index
                                    ),
                                );
                            }
                        }
                    }
                }
            }
            Some(prev_vec) => {
                for (lvl, moves) in tower_i.iter().enumerate() {
                    let prev_moves: &[LeveledMove] = prev_vec
                        .get(lvl)
                        .map(|rc| rc.as_slice())
                        .unwrap_or(&[]);
                    // 升级：该级别在 prev 不存在（或为空）且现非空 ⟹ 新级别涌现。
                    if prev_moves.is_empty() && !moves.is_empty() {
                        let _ = self.write_tower_event(
                            bar,
                            lvl as u32,
                            "level_upgrade",
                            &format!("L{lvl} first compose count={}", moves.len()),
                        );
                    }
                    // 新建 Compose / 延伸末段 end_index。
                    let prev_len = prev_moves.len();
                    for (i, m) in moves.iter().enumerate() {
                        if let RMove::Compose { centers, .. } = &m.rmove {
                            if i >= prev_len {
                                // 新 Compose 涌现。
                                if let Some(c) = centers.first() {
                                    let _ = self.write_tower_event(
                                        bar,
                                        lvl as u32,
                                        "new_center",
                                        &format!(
                                            "L{} #{} zd={} zg={} si={} ei={}",
                                            lvl, m.id.ordinal, c.zd, c.zg, m.start_index, m.end_index
                                        ),
                                    );
                                }
                            } else if let Some(pm) = prev_moves.get(i) {
                                // 已存在 Compose，比较 end_index —— 延伸事件。
                                if m.end_index != pm.end_index {
                                    if let Some(c) = centers.first() {
                                        let _ = self.write_tower_event(
                                            bar,
                                            lvl as u32,
                                            "extend",
                                            &format!(
                                                "L{} #{} zd={} zg={} ei {}->{}",
                                                lvl, m.id.ordinal, c.zd, c.zg, pm.end_index, m.end_index
                                            ),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        self.prev_tower = Some(tower_i.to_vec());
    }

    /// flush 缓冲（drop 前）。
    fn flush(&mut self) {
        use std::io::Write;
        let _ = self.trades_buf.flush();
        let _ = self.tower_buf.flush();
        let _ = self.center_lifecycle_buf.flush();
        if let Err(error) = self.rebase_observability_buf.flush() {
            self.rebase_observability_failed = true;
            self.report_rebase_observability_failure("flush", self.rebase_seq, &error);
        }
    }

    // ─────────────────────────────────────────────────────────────────────
    //  ★#291（SPEC #274 T1）中枢生命周期事件机只读旁路；★**#336 R3：事件源改为消费塔链**
    //  （单一真相源）。事件落 `center_lifecycle.jsonl`，**只外化不回馈决策**（票面边界：
    //  结构地基，狭义短差动作 = #292；默认零行为变化——env 未设本方法不被调用）。
    //
    //  喂数口径（R3 后，禁第二判据源 ⟹ 禁第二复算）：
    //  - **在场中枢（出生源）**：本级塔链 `classification.levels[ℓ].centers` 的**当前全量**，
    //    经 [`classifier::center_lifecycle::CenterEventMachine::consume_chain`] 逐 bar 消费。
    //    这**恰是死亡请求载体的源表**（`mod.rs` BSP 提取喂给 level ℓ 的 centers）⟹ 在场与
    //    载体同表同层，级别对齐按定义成立。
    //  - **买卖点**：`step`（`newly_confirmed_step` append-only diff）的本 bar 新确认点，
    //    修6「定账只消费已确认的点」。
    //  - **事件域**：仅交易活跃区间（与 tower_events 同门）。首个活跃 bar 链上已有的中枢走
    //    **静默采纳**（`kind:"chain_sync" reason:"adopt"` 诊断行，不伪造出生 bar）——与
    //    `write_tower_event` 同款纪律（该处 `prev_tower` 在非活跃 bar 也持续更新 ⟹ 首个活跃
    //    bar 不重播既有 Compose）。故 **born 计数应与 `tower_events` 的 `new_center` 逐级对账**：
    //    机器 level ℓ ⟺ 塔 `new_center level=ℓ+1`（塔 L(k) ≡ `levels[k-1].centers`，见
    //    `.chanlun/review-results/center-death-identity-rootcause-20260726.md` §2.3）。
    //  - **前缀分叉 → 重基**：该级塔缓存全量重置/链回缩 ⟹ `kind:"chain_sync" reason:"rebase"`
    //    诊断行（静默采纳当前链，不伪造出生/死亡）。级消失仍走 `kind:"resync"`。
    //  - **链推进取代**：链前进时前一实例从未收到死亡事件 ⟹ `kind:"superseded"`
    //    （**不伪造** broken——塔无破坏概念，三类点是死亡的唯一教义触发）。
    //  - **陈旧死亡请求**：载体落在**在场窗之外**（Δidx≤−2）或指向已收过教义死亡的实例 ⟹
    //    `kind:"stale"` 诊断行，不计 miskill（口径见 `center_lifecycle.rs` [`StaleKillRequest`]）。
    //
    //  ★**#337 dump schema 变更（GOLDEN 先例登记，不静默）**——`center_lifecycle.jsonl` 三处：
    //  1. `kind:"superseded"` 由「每 bar 每级**一行带 `count`**」改为**逐实例事件行**：
    //     `{si,zd,zg,dd,gg,ei,chain_idx,by_chain_idx,death_form:"arena_termination"}`。
    //     理由 = 裁定②把「取代」升为**在场终结**（死亡登记形态之一），#292 要按**实例**终结
    //     挂起短差，聚合 count 不带身份，接不上。
    //  2. `broken`/`reset` 增 `death_form`（`doctrinal`）与 `slot`（`tail` / `tail_prev` =
    //     放行落在链尾主格还是容读格）。died 为空的 `reset` 二者写 null（没死人，不编造）。
    //  3. `chain_sync` 增身份字段 `tail_si/zd/zg`、`prev_si/zd/zg` 与 `revived` 布尔
    //     （评审 MAJOR-B：采纳/重基把「场」落到谁身上此前无产物级证据）。
    //  轨迹产物 `trades.jsonl`/`tower_events.jsonl` **不受影响**（本旁路只读，逐字节不变已实证）。
    //
    //  ★**已作废（谱系注记，#336 R3）**：旧口径整节（L0 喂 `tower[0][..confirmed_lens[0]]` +
    //  ℓ≥1 喂 `project_to_units_resume(tower[ℓ][..w], levels[ℓ-1].moves)` + 方向冻结水线推导 +
    //  `cl_fed_units` 投影缓存 + `watermark_shrink` resync + #331 R8 层对齐 + #331 R0′ 段游标 +
    //  #331 R2 `MISKILL_ESCAPE_N` 逃逸阀）**全部删除**：那条路径是把塔既有的中枢构造重实现了
    //  一遍（建造错误 = 重造没接生产，同型事故），产出塔链外的第二条中枢链。R3 废止之。
    //  **#291 wf8 对账基线（255/711、83/751、763 born/733 broken/59 reset 等）随之全部作废**。
    // ─────────────────────────────────────────────────────────────────────

    pub(super) fn feed_center_lifecycle(
        &mut self,
        bar: usize,
        classification: &classifier::Classification,
        step: &classifier::Classification,
    ) {
        use classifier::center_lifecycle::{CenterEventMachine, CenterId, ChainConsumed, PointOutcome};

        // 事件域 = 交易活跃区间（与 write_tower_event 同门）。
        let active = match (self.active_start, self.active_end) {
            (Some(s), _) if bar < s => false,
            (_, Some(_)) => true,
            _ => false,
        };
        if !active {
            return;
        }

        // 级数对齐：新级涌现 ⟹ 补建事件机；级消失 ⟹ 截断（工程 resync，照实计数+诊断行）。
        let n_levels = classification.levels.len();
        while self.cl_machines.len() < n_levels {
            let lvl = self.cl_machines.len() as u32;
            self.cl_machines.push(CenterEventMachine::new(lvl));
        }
        if self.cl_machines.len() > n_levels {
            for lvl in n_levels..self.cl_machines.len() {
                let _ = self.write_cl_resync(bar, lvl as u32, "level_vanished");
                self.cl_resync_total += 1;
            }
            self.cl_machines.truncate(n_levels);
        }

        for lvl in 0..n_levels {
            // ── ★R3 事件源：消费本级塔链（唯一真相源；born 由链推进直接推出）──
            let chain: &[super::super::types::Center] = &classification.levels[lvl].centers;
            match self.cl_machines[lvl].consume_chain(chain) {
                // ★#337：`superseded`（在场终结）已升为**逐实例事件**（带身份+链下标），随
                // `events` 一并落行 ⟹ 旧的「每 bar 每级一行带 count」聚合行**退役**。
                ChainConsumed::Advanced { events, superseded: _ } => {
                    for ev in events.iter() {
                        let _ = self.write_cl_event(bar, ev);
                    }
                }
                ChainConsumed::Adopted { adopted } => {
                    let _ = self
                        .write_cl_chain_sync(bar, lvl as u32, "adopt", adopted, chain, false);
                }
                ChainConsumed::Rebased { at, len: _, revived } => {
                    let _ =
                        self.write_cl_chain_sync(bar, lvl as u32, "rebase", at, chain, revived);
                    self.cl_resync_total += 1;
                }
            }
            // ── 喂本 bar 新确认买卖点（修6：只消费已确认的点）──
            //
            // ★#329 H1（R3 后口径收窄）：破坏/重置必须指名道姓——触发点自带载体
            // （`BspPoint.center`：一类 = 被破的最后中枢、三类 = 所离开回抽的中枢）⟹ 取其
            // [`CenterId`]（si,zd,zg）作 target 传入。载体不在本级链上 ⟹ 事件机拒杀并返回误杀
            // 证据（`kind:"miskill"`）；载体命中链上已退场实例 ⟹ 陈旧请求（`kind:"stale"`）。
            if let Some(step_level) = step.levels.get(lvl) {
                for p in step_level.bsp.iter() {
                    let target = match p.center {
                        Some(classifier::bsp::OwnerRef::Center(c)) => Some(CenterId::of(&c)),
                        // 二类锚（`Type1Anchor`）/ 载体缺席 ⟹ 无中枢身份可声明（None ⟹ 场非空
                        // 时必拒杀；二类点本就不产事件，实测 wf8 零命中）。
                        _ => None,
                    };
                    match self.cl_machines[lvl].push_point(p.bits, p.source_index, target) {
                        Ok(PointOutcome::Event(ev)) => {
                            let _ = self.write_cl_event(bar, &ev);
                        }
                        Ok(PointOutcome::Stale(st)) => {
                            let _ = self.write_cl_stale(bar, &st);
                        }
                        Ok(PointOutcome::Silent) => {}
                        Err(mk) => {
                            let _ = self.write_cl_miskill(bar, &mk);
                            self.cl_miskill_total += 1;
                        }
                    }
                }
            }
        }
    }

    /// ★#336 R3：链同步诊断行（`adopt` = 首次消费静默采纳既有链前缀；`rebase` = 前缀分叉后
    /// 工程重基）。二者都**不伪造**出生/死亡事件，对账时单列。
    ///
    /// ★**#337（评审 MAJOR-B）身份字段补齐**：旧行只有 `at`/`chain_len` ⟹ 采纳/重基把「场」
    /// 落到**谁**身上无从产物级核查（#336 未能判定项 4 只能靠推断）。本行补：
    /// - `tail_si/tail_zd/tail_zg` = 采纳后的在场实例（链尾）身份，链空写 null；
    /// - `prev_si/prev_zd/prev_zg` = 容读格（链尾前一格）身份，链长 < 2 写 null；
    /// - `revived` = 本次重基是否让**此前已被教义死亡杀掉的链尾实例重新在场**（复活实证）。
    fn write_cl_chain_sync(
        &mut self,
        bar: usize,
        level: u32,
        reason: &str,
        at: usize,
        chain: &[super::super::types::Center],
        revived: bool,
    ) -> std::io::Result<()> {
        use std::io::Write;
        let id_or_null = |c: Option<&super::super::types::Center>| match c {
            Some(c) => (c.start_index.to_string(), c.zd.to_string(), c.zg.to_string()),
            None => ("null".into(), "null".into(), "null".into()),
        };
        let (tsi, tzd, tzg) = id_or_null(chain.last());
        let (psi, pzd, pzg) =
            id_or_null(if chain.len() >= 2 { chain.get(chain.len() - 2) } else { None });
        let len = chain.len();
        let json = format!(
            "{{\"bar\":{bar},\"level\":{level},\"kind\":\"chain_sync\",\"reason\":\"{reason}\",\
             \"at\":{at},\"chain_len\":{len},\"tail_si\":{tsi},\"tail_zd\":{tzd},\
             \"tail_zg\":{tzg},\"prev_si\":{psi},\"prev_zd\":{pzd},\"prev_zg\":{pzg},\
             \"revived\":{revived}}}\n"
        );
        self.center_lifecycle_buf.write_all(json.as_bytes())
    }

    /// ★#336 R3：陈旧死亡请求诊断行——载体命中链上**已退场**实例（时序滞后，非错位）。
    /// 与 `miskill` 严格分列：`Δidx = target_idx - alive_idx` 是滞后深度（应为负）。
    fn write_cl_stale(
        &mut self,
        bar: usize,
        st: &classifier::center_lifecycle::StaleKillRequest,
    ) -> std::io::Result<()> {
        use classifier::center_lifecycle::KillTrigger;
        use std::io::Write;
        let trigger = match st.trigger {
            KillTrigger::ThirdClass => "third",
        };
        let side = match st.trigger_side {
            super::super::types::Side::Long => "Long",
            super::super::types::Side::Short => "Short",
        };
        let json = format!(
            "{{\"bar\":{bar},\"level\":{lvl},\"kind\":\"stale\",\"trigger\":\"{trigger}\",\
             \"src\":{src},\"side\":\"{side}\",\"alive_si\":{asi},\"alive_zd\":{azd},\
             \"alive_zg\":{azg},\"alive_idx\":{aidx},\"target_si\":{tsi},\"target_zd\":{tzd},\
             \"target_zg\":{tzg},\"target_idx\":{tidx}}}\n",
            bar = bar,
            lvl = st.level,
            trigger = trigger,
            src = st.trigger_source_index,
            side = side,
            asi = st.alive.start_index,
            azd = st.alive.zd,
            azg = st.alive.zg,
            aidx = st.alive_chain_index,
            tsi = st.target.start_index,
            tzd = st.target.zd,
            tzg = st.target.zg,
            tidx = st.target_chain_index,
        );
        self.center_lifecycle_buf.write_all(json.as_bytes())
    }

    /// #291：工程再同步诊断行（非教义生死；对账排除）。
    fn write_cl_resync(&mut self, bar: usize, level: u32, reason: &str) -> std::io::Result<()> {
        use std::io::Write;
        let json =
            format!("{{\"bar\":{bar},\"level\":{level},\"kind\":\"resync\",\"reason\":\"{reason}\"}}\n");
        self.center_lifecycle_buf.write_all(json.as_bytes())
    }

    /// ★#329：误杀拒绝诊断行（`kind:"miskill"`）——触发点载体身份 ≠ 在场中枢身份 ⟹ 事件机
    /// 拒杀，本行是那次拒绝的产物级证据（在场身份 vs 声明身份逐字段并列，缺席写 null）。
    /// 与 `resync` 同属诊断行（非教义事件），对账时单列。
    fn write_cl_miskill(
        &mut self,
        bar: usize,
        mk: &classifier::center_lifecycle::CenterMisKill,
    ) -> std::io::Result<()> {
        use classifier::center_lifecycle::KillTrigger;
        use std::io::Write;
        let trigger = match mk.trigger {
            KillTrigger::ThirdClass => "third",
        };
        let side = match mk.trigger_side {
            super::super::types::Side::Long => "Long",
            super::super::types::Side::Short => "Short",
        };
        let (tsi, tzd, tzg) = match mk.target {
            Some(t) => (t.start_index.to_string(), t.zd.to_string(), t.zg.to_string()),
            None => ("null".into(), "null".into(), "null".into()),
        };
        let json = format!(
            "{{\"bar\":{bar},\"level\":{lvl},\"kind\":\"miskill\",\"trigger\":\"{trigger}\",\
             \"src\":{src},\"side\":\"{side}\",\"alive_si\":{asi},\"alive_zd\":{azd},\
             \"alive_zg\":{azg},\"target_si\":{tsi},\"target_zd\":{tzd},\"target_zg\":{tzg}}}\n",
            bar = bar,
            lvl = mk.level,
            trigger = trigger,
            src = mk.trigger_source_index,
            side = side,
            asi = mk.alive.start_index,
            azd = mk.alive.zd,
            azg = mk.alive.zg,
            tsi = tsi,
            tzd = tzd,
            tzg = tzg,
        );
        self.center_lifecycle_buf.write_all(json.as_bytes())
    }

    /// #291：中枢生命周期事件 JSONL 行（born/broken/reset；★#337 增 `superseded` = 在场终结）。
    ///
    /// ★**#337/#489 两形态分桶登记**：每条**登记了中枢下场**的行都带
    /// `"death_form":"doctrinal"|"arena_termination"`——教义死亡（三类点破坏）vs 在场终结
    /// （被链推进取代）。`born` 与任意 `reset` 均无死亡形态；Reset 的非空 `died_*` 已改作
    /// `alive_center_leak=true` 的活中枢漏发见证。
    /// 二者同走 #292 的「终结」出口（口径见 `center_lifecycle.rs` [`DeathForm`]）。
    ///
    /// ★**#337 容读格标记**：`broken` 的 `"slot":"tail"|"tail_prev"` 表示死亡落在主格还是
    /// 容读格。Reset 的 `slot` 仅定位漏发见证，绝不表示死亡。
    fn write_cl_event(
        &mut self,
        bar: usize,
        ev: &classifier::center_lifecycle::CenterLifecycleEvent,
    ) -> std::io::Result<()> {
        use classifier::center_lifecycle::CenterLifecycleEvent as E;
        use std::io::Write;
        // 容读格标记：死亡落在链尾（主格）⟹ "tail"；落在链尾前一格 ⟹ "tail_prev"。
        // 判据 = 该实例链下标 + 1 == 本机当前链长（链尾）。机器侧的 `alive` 已在事件产出时被
        // 取走 ⟹ 这里用链长而非 alive 判定（链长在死亡事件不变）。
        let slot_str = |lvl: u32, chain_index: usize| -> &'static str {
            match self.cl_machines.get(lvl as usize) {
                Some(m) if chain_index + 1 == m.chain_len() => "tail",
                _ => "tail_prev",
            }
        };
        let side_str = |s: super::super::types::Side| match s {
            super::super::types::Side::Long => "Long",
            super::super::types::Side::Short => "Short",
        };
        let json = match ev {
            E::Born { level, center, chain_index } => format!(
                "{{\"bar\":{bar},\"level\":{level},\"kind\":\"born\",\"zd\":{zd},\"zg\":{zg},\"dd\":{dd},\"gg\":{gg},\"si\":{si},\"ei\":{ei},\"chain_idx\":{idx}}}\n",
                bar = bar, level = level, zd = center.zd, zg = center.zg, dd = center.dd,
                gg = center.gg, si = center.start_index, ei = center.end_index, idx = chain_index,
            ),
            E::Broken { level, center, chain_index, breaker_source_index, breaker_side } => format!(
                "{{\"bar\":{bar},\"level\":{level},\"kind\":\"broken\",\"death_form\":\"doctrinal\",\"slot\":\"{slot}\",\"zd\":{zd},\"zg\":{zg},\"dd\":{dd},\"gg\":{gg},\"si\":{si},\"ei\":{ei},\"chain_idx\":{idx},\"breaker_src\":{src},\"breaker_side\":\"{side}\"}}\n",
                bar = bar, level = level, slot = slot_str(*level, *chain_index),
                zd = center.zd, zg = center.zg, dd = center.dd,
                gg = center.gg, si = center.start_index, ei = center.end_index, idx = chain_index,
                src = breaker_source_index, side = side_str(*breaker_side),
            ),
            // ★#337 在场终结（被链推进取代）：与教义死亡分桶，同走「终结」出口。
            E::Superseded { level, center, chain_index, by_chain_index } => format!(
                "{{\"bar\":{bar},\"level\":{level},\"kind\":\"superseded\",\"death_form\":\"arena_termination\",\"zd\":{zd},\"zg\":{zg},\"dd\":{dd},\"gg\":{gg},\"si\":{si},\"ei\":{ei},\"chain_idx\":{idx},\"by_chain_idx\":{by}}}\n",
                bar = bar, level = level, zd = center.zd, zg = center.zg, dd = center.dd,
                gg = center.gg, si = center.start_index, ei = center.end_index, idx = chain_index,
                by = by_chain_index,
            ),
            E::Reset { level, died_center, died_chain_index, trigger_source_index, trigger_side } => {
                // ★#489：`died_*` 仅为旧线格式兼容保留，现表示 Reset 到场时仍在主格的活中枢
                // 见证；缺席写 null。它不是死亡身份，故 death_form 恒 null。
                let (dzd, dzg, ddd, dgg, dsi, dei) = match died_center {
                    Some(c) => (
                        c.zd.to_string(), c.zg.to_string(), c.dd.to_string(),
                        c.gg.to_string(), c.start_index.to_string(), c.end_index.to_string(),
                    ),
                    None => ("null".into(), "null".into(), "null".into(), "null".into(), "null".into(), "null".into()),
                };
                // ★#336 R3：`died_born_seg`（段号）→ `died_chain_idx`（链下标）；`cleared_segs`
                // 随段序列删除而**去掉**（本机无段序列，写 0 会是编造）。
                let didx = died_chain_index.map_or("null".into(), |o: usize| o.to_string());
                let (leak, slot) = match died_chain_index {
                    Some(idx) => ("true", format!("\"{}\"", slot_str(*level, *idx))),
                    None => ("false", "null".to_string()),
                };
                format!(
                    "{{\"bar\":{bar},\"level\":{level},\"kind\":\"reset\",\"death_form\":null,\"alive_center_leak\":{leak},\"slot\":{slot},\"died_zd\":{dzd},\"died_zg\":{dzg},\"died_dd\":{ddd},\"died_gg\":{dgg},\"died_si\":{dsi},\"died_ei\":{dei},\"died_chain_idx\":{didx},\"trigger_src\":{src},\"trigger_side\":\"{side}\"}}\n",
                    bar = bar, level = level, leak = leak, slot = slot,
                    dzd = dzd, dzg = dzg, ddd = ddd, dgg = dgg,
                    dsi = dsi, dei = dei, didx = didx,
                    src = trigger_source_index, side = side_str(*trigger_side),
                )
            }
        };
        self.center_lifecycle_buf.write_all(json.as_bytes())
    }
}

impl Drop for OpsemDump {
    fn drop(&mut self) {
        self.flush();
    }
}

/// 辅助：Optional<u8> → JSON 字符串（number 或 null）。
fn opt_u8_str(o: Option<u8>) -> String {
    match o {
        Some(v) => v.to_string(),
        None => "null".into(),
    }
}

/// ★R5-a 辅助：LexArgmin top-3 候选 → JSON 数组字符串。每项 `{control, key:{5 维}}`，字典序升序
/// （首名 = p_star 选址/被选，余 = 被拒次优）。空 ⟹ `[]`（无开仓步，不应进 opsem_snap）。
/// `control` 是净持仓格点（f64），`key` 是 [`JThetaKey`] 5 维 i64 字典序键。
fn lex_top3_json(items: &[(strategy::intent::JThetaKey, f64)]) -> String {
    if items.is_empty() {
        return "[]".into();
    }
    let parts: Vec<String> = items
        .iter()
        .map(|(key, control)| {
            format!(
                "{{\"control\":{},\"key\":{{\"tracking_err\":{},\"trade_cost\":{},\"risk_penalty\":{},\"turnover\":{},\"grid_index\":{}}}}}",
                control, key.tracking_err, key.trade_cost, key.risk_penalty, key.turnover, key.grid_index,
            )
        })
        .collect();
    format!("[{}]", parts.join(","))
}

/// 辅助：Optional<f64> → JSON 字符串（number 或 null）。NaN/Inf 一律 null（JSON 无 NaN）。
fn opt_f64_str(o: Option<f64>) -> String {
    match o {
        Some(v) if v.is_finite() => format!("{v}"),
        _ => "null".into(),
    }
}

/// 辅助：Optional<&str> → JSON 字符串（带引号 或 null）。
fn opt_str_quoted(o: Option<&str>) -> String {
    match o {
        Some(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
        None => "null".into(),
    }
}

/// 辅助：Optional<(u32,u64)> → JSON 对象 或 null（parent_id 序列化）。
fn opt_pair_str(o: Option<(u32, u64)>) -> String {
    match o {
        Some((lvl, ord)) => format!("{{\"level\":{lvl},\"ordinal\":{ord}}}"),
        None => "null".into(),
    }
}

/// 辅助：ExitType → 字符串。
/// #283（#281 词汇对齐）：`CloseReverseOpen` 序列化名随枚举更名——dump 文本口径变化
/// （原 "CloseShortDiff"），GOLDEN 先例如实登记；数值/计数零改动。
fn exit_type_str(e: super::super::strategy::interp::ExitType) -> &'static str {
    use super::super::strategy::interp::ExitType;
    match e {
        ExitType::CloseRoot => "CloseRoot",
        ExitType::ReduceCore => "ReduceCore",
        ExitType::CloseReverseOpen => "CloseReverseOpen",
        ExitType::RiskExit => "RiskExit",
        ExitType::Hold => "Hold",
    }
}

/// 辅助：TStage → 字符串。
pub(super) fn t_stage_str(s: super::super::strategy::ledger::TStage) -> &'static str {
    use super::super::strategy::ledger::TStage;
    match s {
        TStage::CostReduction => "CostReduction",
        TStage::CapitalRecovered => "CapitalRecovered",
        TStage::EarningShares => "EarningShares",
    }
}

/// 辅助：VoiceSide → 字符串。
pub(super) fn voice_side_str(v: super::super::strategy::voice::VoiceSide) -> &'static str {
    use super::super::strategy::voice::VoiceSide;
    match v {
        VoiceSide::Long => "Long",
        VoiceSide::Short => "Short",
        VoiceSide::Flat => "Flat",
    }
}

/// 辅助：Vertical → 字符串（声部角色垂直轴）。
/// #283（#281 词汇对齐）：`ReverseOpen` 序列化名随枚举更名（原 "ShortDiff"）——dump
/// `certificate.role` 文本口径变化，GOLDEN 先例如实登记；分类判据零改动。
fn vertical_str(v: super::super::strategy::coverage::Vertical) -> &'static str {
    use super::super::strategy::coverage::Vertical;
    match v {
        Vertical::Ambient => "Ambient",
        Vertical::FollowParent => "FollowParent",
        Vertical::ReverseOpen => "ReverseOpen",
    }
}

/// 辅助：OperationRole → 字符串（H,V,δ 三分量）。
pub(super) fn operation_role_str(r: super::super::strategy::coverage::OperationRole) -> String {
    use super::super::strategy::coverage::{Dir, Horizontal};
    let h = match r.h {
        Horizontal::First => "First",
        Horizontal::SameFollow => "SameFollow",
        Horizontal::SameReverse => "SameReverse",
    };
    let d = match r.delta {
        Dir::Plus => "Plus",
        Dir::Minus => "Minus",
    };
    format!("{h}|{}|{d}", vertical_str(r.v))
}

/// 辅助：RiskMode → 字符串。
pub(super) fn risk_mode_str(m: super::super::strategy::risk::RiskMode) -> &'static str {
    use super::super::strategy::risk::RiskMode;
    match m {
        RiskMode::Normal => "Normal",
        RiskMode::Deleverage => "Deleverage",
        RiskMode::CloseOnly => "CloseOnly",
        RiskMode::Insolvent => "Insolvent",
        RiskMode::Liquidation => "Liquidation",
    }
}

/// 辅助：EtaBucket → 字符串。
pub(super) fn eta_bucket_str(e: super::super::strategy::ledger::EtaBucket) -> &'static str {
    use super::super::strategy::ledger::EtaBucket;
    match e {
        EtaBucket::Deficit => "Deficit",
        EtaBucket::Zero => "Zero",
        EtaBucket::PositiveUnsafe => "PositiveUnsafe",
        EtaBucket::PositiveSafe => "PositiveSafe",
    }
}

/// 辅助：ForceStateA5 → 字符串。
pub(super) fn force_state_str(s: super::super::classifier::divergence::ForceStateA5) -> &'static str {
    use super::super::classifier::divergence::ForceStateA5;
    match s {
        ForceStateA5::Dominated => "Dominated(Weak=背驰)",
        ForceStateA5::Dominates => "Dominates(力度延续)",
        ForceStateA5::Tie => "Tie",
        ForceStateA5::Incomparable => "Incomparable(口径冲突)",
    }
}

#[cfg(test)]
mod rebase_observability_tests {
    use super::*;
    use classifier::center_lifecycle::{CenterId, ChainRebaseObservation};
    use std::cell::Cell;
    use std::io::Write;
    use std::rc::Rc;
    use strategy::voice::VoiceSide;

    fn cid(start_index: usize, zd: i64, zg: i64) -> CenterId {
        CenterId { start_index, zd, zg }
    }

    struct FailingWriter {
        write_calls: Rc<Cell<usize>>,
        flush_calls: Rc<Cell<usize>>,
    }

    impl Write for FailingWriter {
        fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
            self.write_calls.set(self.write_calls.get() + 1);
            Err(std::io::Error::other("注入的重基观测写入失败"))
        }

        fn flush(&mut self) -> std::io::Result<()> {
            self.flush_calls.set(self.flush_calls.get() + 1);
            Err(std::io::Error::other("注入的重基观测 flush 失败"))
        }
    }

    #[test]
    fn rebase_observation_writer_failure_is_visible_fail_open_and_not_retried() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("系统时钟应晚于 UNIX_EPOCH")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "newchan-rebase-observability-failure-{}-{nonce}",
            std::process::id()
        ));
        let input = RebaseObservationInput {
            level: 1,
            chain: ChainRebaseObservation {
                at: 0,
                before: vec![cid(10, 100, 200)],
                after: vec![cid(20, 110, 210)],
            },
            suspended_before: vec![(VoiceSide::Long, cid(10, 100, 200))],
            revived: false,
        };
        let write_calls = Rc::new(Cell::new(0));
        let flush_calls = Rc::new(Cell::new(0));

        {
            let mut dump = OpsemDump::at_dir(&dir).expect("测试 dump writer 应可创建");
            dump.rebase_observability_buf = Box::new(FailingWriter {
                write_calls: Rc::clone(&write_calls),
                flush_calls: Rc::clone(&flush_calls),
            });

            dump.write_rebase_observation_fail_open(42, &input);
            assert_eq!(write_calls.get(), 1, "首次失败必须真实到达观测 writer");
            assert_eq!(dump.rebase_observability_failure_count, 1);
            assert!(dump.rebase_observability_failure_reported);
            assert!(dump.rebase_observability_failed);
            assert_eq!(dump.rebase_seq, 0, "整行失败不得提交序号");

            dump.write_rebase_observation_fail_open(43, &input);
            assert_eq!(write_calls.get(), 1, "失败后关闭旁路，候选序号不得被重试复用");
            assert_eq!(dump.rebase_observability_failure_count, 1);
            assert_eq!(dump.rebase_seq, 0);

            dump.flush();
            assert_eq!(flush_calls.get(), 1, "显式 flush 必须覆盖重基观测 buffer");
            assert_eq!(
                dump.rebase_observability_failure_count, 2,
                "flush 失败也必须进入可见 witness"
            );
        }

        std::fs::remove_dir_all(&dir).expect("清理本测试专属临时目录");
    }

    #[test]
    fn rebase_observability_jsonl_exposes_transaction_and_honest_mapping_reads() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("系统时钟应晚于 UNIX_EPOCH")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "newchan-rebase-observability-{}-{nonce}",
            std::process::id()
        ));
        let old = cid(10, 100, 200);
        let stable = cid(30, 300, 400);
        let added = cid(10, 110, 210);
        let input = RebaseObservationInput {
            level: 1,
            chain: ChainRebaseObservation {
                at: 0,
                before: vec![old, stable],
                after: vec![added, stable],
            },
            suspended_before: vec![(VoiceSide::Long, old), (VoiceSide::Short, stable)],
            revived: false,
        };

        let mut dump = OpsemDump::at_dir(&dir).expect("测试 dump writer 应可创建");
        dump.write_rebase_observation(42, &input).expect("D0 JSONL 应可落盘");
        drop(dump);

        let raw = std::fs::read_to_string(dir.join("rebase_observability.jsonl"))
            .expect("D0 产物必须存在");
        let rows: Vec<serde_json::Value> = raw
            .lines()
            .map(|line| serde_json::from_str(line).expect("每行必须是合法 JSON"))
            .collect();
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row["schema"], "rebase_observability_d0_v1");
        assert_eq!(row["rebase_seq"], 1);
        assert_eq!(row["bar"], 42);
        assert_eq!(row["level"], 1);
        assert_eq!(row["removed_ids"].as_array().expect("removed 数组").len(), 1);
        assert_eq!(row["added_ids"].as_array().expect("added 数组").len(), 1);
        assert_eq!(
            row["mapping_result_domain"],
            serde_json::json!(["continued", "split", "genuinely_removed", "ambiguous"])
        );

        let suspended = row["suspended_before"].as_array().expect("挂起数组");
        assert_eq!(suspended.len(), 2);
        assert_eq!(suspended[0]["side"], "Long");
        assert_eq!(suspended[0]["mapping_result"], "ambiguous");
        assert_eq!(suspended[0]["unique"], false);
        assert_eq!(suspended[0]["bijective"], false);
        assert_eq!(
            suspended[0]["non_lineage_hints"]["same_start_added_count"],
            1,
            "同 start 只作非谱系提示，不得把 ambiguous 偷换成 continued"
        );
        assert_eq!(suspended[1]["side"], "Short");
        assert_eq!(suspended[1]["mapping_result"], "continued");
        assert_eq!(suspended[1]["unique"], true);
        assert_eq!(suspended[1]["bijective"], true);

        assert_eq!(row["mapping_result_counts"]["continued"], 1);
        assert_eq!(row["mapping_result_counts"]["split"], 0);
        assert_eq!(row["mapping_result_counts"]["genuinely_removed"], 0);
        assert_eq!(row["mapping_result_counts"]["ambiguous"], 1);
        assert_eq!(row["checks"]["exact_mapping_bijective"], true);
        assert_eq!(row["checks"]["complete_bijection"], false);
        assert_eq!(row["construction_witness"]["status"], "none");
        assert!(
            row["construction_witness"]["missing"]
                .as_array()
                .expect("缺口数组")
                .iter()
                .any(|v| v == "stable_lineage_id"),
            "无精确谱系时必须明示缺失 stable_lineage_id"
        );

        std::fs::remove_dir_all(&dir).expect("清理本测试专属临时目录");
    }
}
