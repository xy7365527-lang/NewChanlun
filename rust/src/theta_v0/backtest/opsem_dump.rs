//! ★opsem-dump seam（B-M5，#85）：严格区间套证书 sidecar + env-gated 操作语义 dump，
//! 自 runner.rs 纯移动（设计 chanlun/plans/runner-rs-seam-designs-20260721.md §M5）。
//!
//! 公共类型经 `runner` 门面 `pub use` 保持原路径（`runner::StrictNestSidecarSummary`）。

use super::super::config::ThetaConfig;
use super::super::{classifier, parser, strategy};
use super::runner::{LedgerOpen, TypedTrade};

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
    trade_id_counter: u64,
    /// 交易活跃区间（首入场 bar .. 末离场 bar）；None=尚未见入场。
    active_start: Option<usize>,
    active_end: Option<usize>,
    /// 上一 bar 的塔（仅交易活跃区间内 diff，O(n) per bar）。
    prev_tower: Option<Vec<std::rc::Rc<Vec<classifier::recursive_tower::LeveledMove>>>>,
    /// ★#291：每级一台中枢生命周期事件机（下标=级别）。
    cl_machines: Vec<classifier::center_lifecycle::CenterEventMachine>,
    /// ★#291：每级已喂事件机的 units 投影缓存（长度 = 该级已喂段数 fed；前缀含水线内稳定段）。
    cl_fed_units: Vec<Vec<classifier::center::UnitRange>>,
    /// ★#291：工程再同步累计（塔 cascade/水线回缩/级消失 ⟹ 该级 resync；非教义生死，照实单列）。
    cl_resync_total: u64,
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

    /// 在指定目录开两个 JSONL 写入器。
    /// ponytail: 截断打开（每次回测重写；同 dump_deltafree_pertrade 落盘语义）。path 局部化——
    /// struct 只持 BufWriter（path 仅 create 时用，后续不读，YAGNI 不存字段）。
    fn at_dir(dir_path: &std::path::Path) -> Option<Self> {
        std::fs::create_dir_all(dir_path).ok()?;
        let trades_file = std::fs::File::create(dir_path.join("trades.jsonl")).ok()?;
        let tower_file = std::fs::File::create(dir_path.join("tower_events.jsonl")).ok()?;
        // ★#291：第三产物（同截断语义——每次回测重写）。
        let cl_file = std::fs::File::create(dir_path.join("center_lifecycle.jsonl")).ok()?;
        Some(Self {
            trades_buf: std::io::BufWriter::new(trades_file),
            tower_buf: std::io::BufWriter::new(tower_file),
            center_lifecycle_buf: std::io::BufWriter::new(cl_file),
            trade_id_counter: 0,
            active_start: None,
            active_end: None,
            prev_tower: None,
            cl_machines: Vec::new(),
            cl_fed_units: Vec::new(),
            cl_resync_total: 0,
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
    }

    // ─────────────────────────────────────────────────────────────────────
    //  ★#291（SPEC #274 T1）中枢生命周期事件机只读旁路（ADR 0001 修正案一·补充二
    //  「中枢=事件」：出生=第三段重叠完成；破坏=三类买卖点；本级一类点 ⟹ 段序列与在场
    //  中枢同死）。事件落 `center_lifecycle.jsonl`，**只外化不回馈决策**（票面边界：
    //  结构地基，狭义短差动作 = #292；默认零行为变化——env 未设本方法不被调用）。
    //
    //  喂数口径（因果，禁第二判据源——构造算子/投影函数全部复用分类器单一来源）：
    //  - **段序列**：只喂**确认前缀**内的段。L0 段 = `tower[0][..confirmed_lens[0]]`
    //    （parser `segments_confirmed_len` 证书，跨 bar bit-stable，`LeveledMove::from_unit`
    //    逆恢复 `UnitRange`）；ℓ≥1 段 = `project_to_units_resume(tower[ℓ-1][..w], blocks)`
    //    增量投影（`blocks` = `levels[ℓ-1].moves`，与分类器内部同函数同输入）。ℓ≥1 水线
    //    `w = min(confirmed_lens[ℓ-1], parent_len-1)`：单元 i 方向由 ownership 关系 R(i-1)
    //    定，R(i-1) 冻结 ⟺ i ≤ m-2（decompose 模块头：R_j 冻结 ⟺ j < m-2）⟹ 喂到
    //    i < m-1 保证已喂段方向永冻、前缀一致。frontier（未确认尾段/临时尾关系）不喂——
    //    事件机比塔更保守（塔中枢可含 frontier 段），born 时点可能**晚于**塔 new_center
    //    （对账口径差异，如实列出）。
    //  - **买卖点**：`step`（`newly_confirmed_step` append-only diff）的本 bar 新确认点，
    //    修6「定账只消费已确认的点」。
    //  - **事件域**：仅交易活跃区间（与 tower_events 同门）——机器自首个活跃 bar 起从空
    //    段序列开始喂（born_seg_ordinal 是活跃窗内序号，对账口径写明）。
    //  - **resync（工程再同步，非教义生死）**：水线回缩（塔 cascade 失效传播）或级消失
    //    ⟹ 该级机器 `resync()`（清段序列+在场中枢，不产生死事件）+ 落 `kind:"resync"`
    //    诊断行 + `cl_resync_total` 计数。对账时排除该行。
    // ─────────────────────────────────────────────────────────────────────
    pub(super) fn feed_center_lifecycle(
        &mut self,
        bar: usize,
        classification: &classifier::Classification,
        tower: &[std::rc::Rc<Vec<classifier::recursive_tower::LeveledMove>>],
        confirmed_lens: &[usize],
        step: &classifier::Classification,
    ) {
        use classifier::center::UnitRange;
        use classifier::center_lifecycle::{CenterEventMachine, CenterLifecycleEvent};
        use classifier::descend::RMove;

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
        let n_levels = classification.levels.len().min(tower.len());
        while self.cl_machines.len() < n_levels {
            let lvl = self.cl_machines.len() as u32;
            self.cl_machines.push(CenterEventMachine::new(lvl));
            self.cl_fed_units.push(Vec::new());
        }
        if self.cl_machines.len() > n_levels {
            for lvl in n_levels..self.cl_machines.len() {
                let _ = self.write_cl_resync(bar, lvl as u32, "level_vanished");
                self.cl_resync_total += 1;
            }
            self.cl_machines.truncate(n_levels);
            self.cl_fed_units.truncate(n_levels);
        }

        for lvl in 0..n_levels {
            // ── 段水线（确认前缀；ℓ≥1 再按 ownership 方向冻结界收窄）──
            let w = if lvl == 0 {
                confirmed_lens.first().copied().unwrap_or(0).min(tower[0].len())
            } else {
                let parent_len = tower[lvl - 1].len();
                confirmed_lens
                    .get(lvl - 1)
                    .copied()
                    .unwrap_or(0)
                    .min(parent_len.saturating_sub(1))
                    .min(parent_len)
            };
            // ── 水线回缩（cascade 传播）⟹ 该级工程 resync，重喂新前缀 ──
            if w < self.cl_fed_units[lvl].len() {
                self.cl_machines[lvl].resync();
                self.cl_fed_units[lvl].clear();
                let _ = self.write_cl_resync(bar, lvl as u32, "watermark_shrink");
                self.cl_resync_total += 1;
            }
            // ── 喂新确认段（投影增量，摊还 O(新增)/bar）──
            let fed = self.cl_fed_units[lvl].len();
            if w > fed {
                if lvl == 0 {
                    for m in tower[0][fed..w].iter() {
                        if let RMove::Segment { direction, lo, hi } = &m.rmove {
                            self.cl_fed_units[lvl].push(UnitRange {
                                start_index: m.start_index,
                                end_index: m.end_index,
                                direction: *direction,
                                lo: *lo,
                                hi: *hi,
                            });
                        }
                    }
                } else {
                    // 单一来源增量投影（mod.rs A3 §2.5 同款调用形；blocks=当前 moves，已喂段
                    // 方向由水线收窄保证冻结 ⟹ resume 契约「前缀一致」成立）。
                    classifier::recursive_tower::project_to_units_resume(
                        &tower[lvl - 1][..w],
                        &classification.levels[lvl - 1].moves,
                        &mut self.cl_fed_units[lvl],
                    );
                }
                let new_units = self.cl_fed_units[lvl][fed..].to_vec();
                for u in new_units {
                    if let Some(ev) = self.cl_machines[lvl].push_segment(u) {
                        let _ = self.write_cl_event(bar, &ev);
                    }
                }
            }
            // ── 喂本 bar 新确认买卖点（修6：只消费已确认的点）──
            if let Some(step_level) = step.levels.get(lvl) {
                for p in step_level.bsp.iter() {
                    if let Some(ev) = self.cl_machines[lvl].push_point(p.bits, p.source_index) {
                        let _ = self.write_cl_event(bar, &ev);
                    }
                }
            }
        }
    }

    /// #291：工程再同步诊断行（非教义生死；对账排除）。
    fn write_cl_resync(&mut self, bar: usize, level: u32, reason: &str) -> std::io::Result<()> {
        use std::io::Write;
        let json =
            format!("{{\"bar\":{bar},\"level\":{level},\"kind\":\"resync\",\"reason\":\"{reason}\"}}\n");
        self.center_lifecycle_buf.write_all(json.as_bytes())
    }

    /// #291：中枢生命周期事件 JSONL 行（born/broken/reset 三类，事件含级别/ZD/ZG/出生段号）。
    fn write_cl_event(
        &mut self,
        bar: usize,
        ev: &classifier::center_lifecycle::CenterLifecycleEvent,
    ) -> std::io::Result<()> {
        use classifier::center_lifecycle::CenterLifecycleEvent as E;
        use std::io::Write;
        let side_str = |s: super::super::types::Side| match s {
            super::super::types::Side::Long => "Long",
            super::super::types::Side::Short => "Short",
        };
        let json = match ev {
            E::Born { level, center, born_seg_ordinal } => format!(
                "{{\"bar\":{bar},\"level\":{level},\"kind\":\"born\",\"zd\":{zd},\"zg\":{zg},\"dd\":{dd},\"gg\":{gg},\"si\":{si},\"ei\":{ei},\"born_seg\":{ord}}}\n",
                bar = bar, level = level, zd = center.zd, zg = center.zg, dd = center.dd,
                gg = center.gg, si = center.start_index, ei = center.end_index, ord = born_seg_ordinal,
            ),
            E::Broken { level, center, born_seg_ordinal, breaker_source_index, breaker_side } => format!(
                "{{\"bar\":{bar},\"level\":{level},\"kind\":\"broken\",\"zd\":{zd},\"zg\":{zg},\"dd\":{dd},\"gg\":{gg},\"si\":{si},\"ei\":{ei},\"born_seg\":{ord},\"breaker_src\":{src},\"breaker_side\":\"{side}\"}}\n",
                bar = bar, level = level, zd = center.zd, zg = center.zg, dd = center.dd,
                gg = center.gg, si = center.start_index, ei = center.end_index, ord = born_seg_ordinal,
                src = breaker_source_index, side = side_str(*breaker_side),
            ),
            E::Reset { level, died_center, died_born_seg_ordinal, cleared_segments, trigger_source_index, trigger_side } => {
                // died 缺席写 null（与 trades.jsonl 缺席字段同款纪律，不编造）。
                let (dzd, dzg, ddd, dgg, dsi, dei) = match died_center {
                    Some(c) => (
                        c.zd.to_string(), c.zg.to_string(), c.dd.to_string(),
                        c.gg.to_string(), c.start_index.to_string(), c.end_index.to_string(),
                    ),
                    None => ("null".into(), "null".into(), "null".into(), "null".into(), "null".into(), "null".into()),
                };
                let dord = died_born_seg_ordinal.map_or("null".into(), |o| o.to_string());
                format!(
                    "{{\"bar\":{bar},\"level\":{level},\"kind\":\"reset\",\"died_zd\":{dzd},\"died_zg\":{dzg},\"died_dd\":{ddd},\"died_gg\":{dgg},\"died_si\":{dsi},\"died_ei\":{dei},\"died_born_seg\":{dord},\"cleared_segs\":{cleared},\"trigger_src\":{src},\"trigger_side\":\"{side}\"}}\n",
                    bar = bar, level = level, dzd = dzd, dzg = dzg, ddd = ddd, dgg = dgg,
                    dsi = dsi, dei = dei, dord = dord, cleared = cleared_segments,
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
