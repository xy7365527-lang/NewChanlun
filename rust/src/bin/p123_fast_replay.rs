//! task #123：快速端到端重放（sparse dirty-driven）。
//!
//! 设计：`chanlun/review-results/p123-fast-replay-design-20260718.md` 候选 C 主线
//! （per-run dirty 驱动稀疏重算）；Phase 0 实测：`/tmp/p122_phase0_report.md`。
//!
//! ★口径版本（同 p116，2026-07-18 起）：p117 T1 终端背书裁定
//!（`chanlun/escalate/bsp-terminal-endorsement-ruling-20260718.md`）——TERM/CERT 终端查法
//! = `levels[ℓ-1].bsp` + C-b 窗口（`[interval_b.0, turn_source]` 最早 confirm_side 点；lib 单一
//! 来源 `nest::terminal_bits_at_event`/`terminal_bits_in_book`，与 p92/p116 生产查法逐字同口径）。
//!
//! 管线 = p116_turnpoint_anchor_existence 原样：terminal pass（增量塔）/ TURN 逐 bar 观察 /
//! 终态快照 targets / 终态兜底 observe_snapshot / 证书装配 / CKPT 侧信道 / dump 行格式 /
//! stdout 门行字段（前缀 P116_→P123_）全部逐字保留，账本语义（candidates/divergences 首插
//! or_insert、pending 出清条件、term_seen 去重）逐行一致。**唯一**差异在 prefix pass：
//! 慢版「每次 trigger 对全部 arrived (level,run) 全量重估」换为 per-(level,run) 评估缓存 +
//! 四类 dirty 判据稀疏重估（设计 §3）。判据代码零新增——dirty run 用与慢版逐字相同的 lib
//! 调用链同一输入重估（project run → decompose → assemble_level_view →
//! provide_nest_candidate_events），无新判据路径。
//!
//! ── 稀疏化架构（实装）──
//! trigger 语义冻结（设计 §4.4）：trigger=(forest_epoch, signal_signature) 检测与慢版逐字一致；
//! 评估与账本更新（candidates/divergences 首插、pending 出清、DIV/TERM 落盘）只发生在 trigger
//! bar，钟位 = trigger bar（= 慢版 `or_insert(index)` 归集 bar，逐位对齐）。suppressed 情形
//!（trigger 变但 pending 已空）不更新 last_trigger，与慢版同行同语义。
//! 每个 trigger bar，对 arrived run（turn_source<=as_of 的 pending 目标所属
//! (level, run_source_start)，run_source_start = 目标 provider_window.0）：
//!   dirty ⟹ 慢版 per-run 评估块逐字重估，产出按当前 pending 过滤后入缓存；
//!   clean ⟹ 复用缓存事件集，再按当前 pending 过滤（保序）做 pending 匹配。
//! 事件拼接顺序 = 慢版 collect_target_candidates 同一嵌套序（level 升序 → run_source 升序 →
//! provide 内部序），DIV/TERM 落盘行序逐位对齐。
//! 四类 dirty 判据：
//!   (i)   tower[level] 内容变（frontier 重折/追加；run 投影与 run 分区源）。
//!   (ii)  tower[level-1] 内容变（lower legs 源）。
//!   (iii) as_of 越过 lower legs 端点水位。level_view 全部 as_of 依赖已逐处核验为
//!         `segment.end_index <= as_of` 形：pan 支（level_view.rs:722）/ provide_divergence_pairs
//!         （:829,:853）/ trend_confirm_time 扫描界（:562）与三买（signal.rs:505）。legs 端点恒 ≤
//!         其所在 bar（塔只含 ≤ 当前 bar 的走势）⟹ 端点集合在 (ii) 干净期间不变且全部
//!         ≤ last_as_of ⟹ (iii) 独立触发构造性不可能——(iii) ⊂ (ii)。判据保留作廉价断言，
//!         计数器 P123_SPARSE wm_cross_without_lower 恒 0 自证（非 0 即实装 bug，停线）。
//!   (iv)  levels[ℓ-1].bsp 账本内容变 ⟹ 仅对该级未命中 pending 事件做 TERM 反查
//!         （O(账本)，不重投影；signal_signature 对账本属过度触发，本路径仅 TERM 消费）。
//!         账本只在 trigger bar 可变：bsp 提取 memo key=(centers.len, upper_moves.len,
//!         struct_len)（mod.rs:1962-1963）+ frontier 失效清 key（mod.rs:1698-1700/:1750-1752）
//!         的全部前提变化都是塔写入站点 ⟹ forest_epoch bump（mod.rs:2118-2124 E1-E4 折叠，
//!         逐值判据 over-invalidate）⟹ trigger；sig_only=0（Phase 0 §2）为旁证。
//! 判据完备性（C5 构造性签字）：事件产出变化 ⟹ 评估输入（tower[ℓ] / tower[ℓ-1] 内容）变化
//! ⟹ 塔写入站点命中 ⟹ forest_epoch bump ⟹ 同 bar trigger。lib 全链（projection/decompose/
//! assemble/provide）为输入纯函数：投影=run 窗切片纯函数（(i)）、lower_legs_from=tower[ℓ-1]
//! 纯函数（(ii)）、hist/dif/close_src 因果前缀（早段冻结；map_src_to_close_idx 对固定端点的
//! 结果只在端点越过时变——(iii)/(ii) 覆盖）。故 sparse 在 trigger bar 的评估/入账覆盖全部
//! 可证事件，首证钟与慢版逐位对齐，不依赖「事件可证 ⟹ trigger」经验假设。
//!
//! ── 哨兵实装：内容快照值比对（对设计 §4.1 Rc holding trick 的实装层替代，090 声明）──
//! 设计 §4.1 指定「跨迭代持有上一 bar 的 tower Rc 克隆 ⟹ cache 内 Rc::make_mut 写时复制
//! ⟹ ptr 变 ⟺ 该级写过」。本实装用**内容快照值比对**替代 ptr 哨兵，理由三条：
//! (a) soundness——dirty 判据的语义要求是「输入内容逐位相同 ⟹ 产出逐位相同」，值比对直接
//!     回答该问题；ptr 只是内容的代理，且 frontier 同字节重写（mod.rs:1896-1901 的
//!     tail_upper==popped_upper 每 bar 重扫复现情形）在 holding 下 ptr 必翻 = 系统性
//!     over-dirty（安全方向但实测会抬高重估率）。
//! (b) 免除放大——cache 条目永久持有 Rc 会使 strong_count>1 ⟹ lib 每个写 bar 对该级
//!     Vec<LeveledMove> 全量写时复制（mod.rs:1809/1815/1904-1905 站点，O(级长)×写 bar 数
//!     的纯新增成本）。快照方案与慢版一样逐 bar drop 上轮 classification/tower 引用，
//!     strong_count==1 原地写保留，对 lib 零成本扰动。
//! (c) 无 ptr 语义 ⟹ 无地址复用隐患，完备性不依赖 lib 写入站点审计。
//! 代价：每 trigger 每 arrived 级 O(级长) 值比对 + 变更时代次快照克隆（LeveledMove 为浅
//! Clone——subs/centers 均 Rc 共享）。计数与计时见 stderr P123_SPARSE（不进验收面）。
//!
//! ── §6.4 强制前置签字（实装前逐函数核验，2026-07-18）──
//! 1) `nearest_confirmed_center_idx`（signal.rs:211-218）：纯函数——partition_point 读
//!    `centers.end_index <= seg_start`，无 as_of/全局态。输入 centers=run 投影种子（(i)）、
//!    seg_start=lower leg（(ii)）。隐式时间依赖：无。签字通过。
//! 2) `locate_pan_div_structure` / `_front_anchor`（signal.rs:637-694 / :706-733）：纯函数——
//!    窗口由 c.end_index/seg.start_index 二分定界，`departure_move_c_start`
//!    （divergence.rs:841-851）同为纯函数（窗口 ≤ seg.start_index，无 as_of 入参）；as_of 仅经
//!    调用点过滤 `segment.end_index <= as_of`（level_view.rs:722）进入 ⟹ (iii) 覆盖。签字通过。
//! 3) `center_block_kind` / `center_block_kind_at`（decompose.rs:213-226 / :197-205）：纯函数——
//!    只读 blocks（projection centers 的 decompose 输出 ⟸ (i)），无任何时间读。签字通过。
//!
//! ── trend_confirm 游标驻留（090 能力声明）──
//! 设计 §4.3 的**函数内** per-pair 游标驻留（env 包络/acc_hi/area/dif/hist 极值跨评估驻留）
//! 需改 theta_v0 生产源码（trend_confirm_time 是 level_view.rs 私有函数，bin 不可达）或
//! fork 判据路径——两者本轮皆禁（生产源码零改动 / 禁新判据路径）。**未实装**，如实声明。
//! 实装的驻留 = **view 粒度**：评估缓存条目跨 clean bar 驻留整批产出（投影/pairs/事件），
//! (i)-(iv) 保证输入不变 ⟹ 重扫零次；§4.3 的单调性论据（T2 假→真、T5 真→假终假、t3 未决
//! 保持未决）在本架构中是判据 (ii)/(iii) 的完备性论据（端点越过必须重估，两个方向的状态
//! 翻转都被覆盖），不是独立机制。§4.5 assemble/provide 双算单源化同理**未实装**（lib 内
//! fusion 属生产改动）；其成本在保留评估内部，与慢版逐位相同。残量界：稀疏化收益全部来自
//! 评估次数下降，保留评估单价不变——实测计数/计时见 stderr P123_SPARSE，预估见任务回报。
//!
//! ── 验收面（设计 §5）与白名单 ──
//! 必须逐位：stdout 门行全套（INPUT/RULE/PROBE/YIELD/CERT/D3/BASELINE/PROVIDER/SNAPSHOT/
//! BIT_EXACT/R7/MISSED(+EVENT)，与慢版对拍时先做 `sed 's/^P116_/P123_/'` 正规化）+ dump 的
//! CERT（judge_at 向量逐字=首证钟唯一主键）/DIV/TERM/TURN/TURN_CLASS/FALLBACK（含行序）。
//! 白名单剔除：**仅** PROVIDER 行的 `views=` 字段（物理装配次数，稀疏化必变；同行
//! snapshots/too_short/invalid_seed/missing_carried_center/other/unresolved_targets/complete
//! 余部逐位，unresolved_targets 保留逐位——候选 A 语义与 C 无关）。CKPT 侧信道验收关闭
//!（物理时机耦合，只写不判；对拍双方均不设 P116_CKPT）。stderr 进度/稀疏统计行不在验收面。
//!
//! ── M2 截段自检实测锚（2026-07-18，P123_SHADOW=1 全程对拍；后台有 1 核 v3 重放竞争，墙钟仅
//! 相对对照）──
//! - 100k（P116_MAX_BARS=100001, P116_CKPT=100000）：triggers=1205 reevals=1423 reuses=561
//!   wm_cross_without_lower=0（(iii)⊂(ii) 构造性证明的实证）；shadow_checks=1984（=慢版 views
//!   全数）mismatches=0 / term mismatches=0；CKPT@100000 与 250k 档逐位一致。
//! - 250k（P116_MAX_BARS=250001, P116_CKPT=50000）：triggers=2485（与 Phase 0 §2 fired 逐位
//!   一致）reevals=3719（L1 1984/1984=100%、L2 1217/1824=66.7%、L3 518/1470=35.2%；
//!   slow_would 各级 = Phase 0 §3 REEVAL 逐位一致）reuses=1559 wm_cross_without_lower=0
//!   shadow_checks=5278 mismatches=0 / term mismatches=0；term_skips=0（L1 恒 fresh，(iv)
//!   跳过路径两档均未触发——构造性安全但未实证，如实声明）。
//! - 跨码对照：CKPT@250000 集合 == /tmp/p92_ckpt_dump_v3.txt 同 as_of 集合（268 行，diff=0，
//!   现行口径唯一既有参照）；DIV@250k 集合+行序 == /tmp/p116_smoke_dump.txt（465 行 diff=0，
//!   背驰判据口径跨二进制一致）⟹ prefix 稀疏路径与旧慢版产物逐位兼容；TURN/FALLBACK 同
//!   diff=0；TERM/CERT/CKPT vs smoke/v2 差异 = p117 T1 级别移位（smoke/v2 为 T1 前旧编译
//!   产物，按 p116 模块头 T4 裁决3 注记，不作验收基准）。
//! - 跨档残余（同码，非 sparse bug，已逐条归因）：DIV 1 行序位移（DIV 行无 key 身份/兜底
//!   时机差）；TERM 100k-only 13 行（终态快照 targets 非单调——frontier 相邻盘整结构在
//!   snapshot@100k 在场、@250k 离场/改址，及 targeted↔snapshot run 分区差——两者皆为
//!   p116 原有语义，shadow 已证同 trigger 内 sparse≡强制全量）。关键方向（250k 首见 ≤100k
//!   而 100k 漏）计数=0。
//! - 保留率实测：views 3719/5278=70.5%@250k——trigger 率 ≈ L0 段产出率（0.929% vs
//!   0.905%/bar，Phase 0 §2/§6）⟹ L1（view 税 82%）~全 dirty，稀疏化在输入侧判据下的
//!   实测节省远小于 Phase 0 oracle（productive-only 2-4%）。预估加速比见任务回报（修正口径）。
//!
//! ── M3 全量双跑协议（本里程碑只备不跑；由编排者在 v3 全量完成后执行）──
//! A) 慢版对照（同 worktree 同码）：
//!    `cargo run --release --bin p116_turnpoint_anchor_existence -- analysis/data_cache/btc_1m_full.json`
//!    （P116_MAX_BARS 不设=全量；P116_DUMP=/tmp/p116_full_dump_v3.txt；P116_CKPT 不设）。
//!    快版：本 bin 全量（P116_DUMP=/tmp/p123_full_dump.txt；P116_CKPT 不设）。
//!    stdout 正规化后 diff=0（白名单仅 views=）；dump 的 CERT/DIV/TERM/TURN/TURN_CLASS/FALLBACK
//!    含行序 diff=0；首证钟主键 = CERT judge_at 向量逐字；P123_MISSED missed=0。
//! B) CKPT 对账（既有的在跑产物）：v3 全量完成后，本 bin 另跑一次 P116_CKPT=250000，
//!    逐 as_of 与 /tmp/p92_ckpt_dump_v3.txt 的 CKPT 集合比对——同码同口径 ⟹ 每 as_of 集合
//!    相等（注意 p116_smoke_dump / p92_ckpt_dump_v2 是 p117 T1 **前**旧编译产物，其
//!    TERM/CERT/CKPT 集合按 p116 模块头 T4 裁决3 注记「级别移位前口径」，不作验收基准）。
//! C) 判负即停线：任一 diff 现场归档（bar+EventKey+level+判据计数现场），先查 dirty 漏判
//!    （P123_SPARSE 计数），再查 trigger 语义；禁调判据凑通过（090）。
//!
//! 用法：`cargo run --release --bin p123_fast_replay -- <btc_1m_full.json>`
//! env（协议逐字复用 p116，设计 §7）：`P116_DUMP`（dump 路径）、`P116_MAX_BARS`（截断重放
//! 长度）、`P116_CKPT`（每 K bar 检查点全量证书快照，只写不判）。p123 自有开关：
//! `P123_SHADOW=1`（设计 §6.3 shadow 强制对拍：每 trigger 对 arrived run 无视 dirty 判定
//! 强制重估并与缓存路径逐字比对 + TERM 门控旁证；mismatch 打 stderr，计数入
//! P123_SPARSE_SUMMARY；长期回归开关，不进验收面）。

use newchan_rust::theta_v0::classifier;
use newchan_rust::theta_v0::classifier::bsp::BspPoint;
use newchan_rust::theta_v0::classifier::decompose;
use newchan_rust::theta_v0::classifier::level_view::{
    assemble_level_view, lower_legs_from, project_extended_windows_carried_only,
    provide_nest_candidate_events, C2LevelViewConfig, C2VersionTuple, CoordinateWindow,
    LevelViewMaterial, LevelViewQuery, LowerLeg, NestCandidateEvent, NestDivergenceKind,
    ProjectionError, ProjectionMaterial,
};
use newchan_rust::theta_v0::classifier::nest::{
    assemble_certificates_snapshot, assemble_typed_certificates, event_bsp_book_level,
    terminal_bits_at_event, terminal_bits_in_book, NestIntervalCaliber, TerminalMatch,
    TypedNestCertificate,
};
use newchan_rust::theta_v0::classifier::recursive_tower::LeveledMove;
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::{ParseLayer, ParseLayerIncr};
use newchan_rust::theta_v0::types::{quantize, Bar, BspBits, MoveKind, Side, Timestamp};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::rc::Rc;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

/// #98/#99 调研侧信道：`P116_DUMP=<path>` 时逐条落盘明细行（协议逐字复用 p116，见模块头）。
/// 只写不判——不参与任何账本、真值或裁定；未设 env 时零行为差异。
static DUMP: OnceLock<Option<Mutex<BufWriter<File>>>> = OnceLock::new();

fn dump_line(args: std::fmt::Arguments<'_>) {
    let sink = DUMP.get_or_init(|| {
        std::env::var("P116_DUMP")
            .ok()
            .and_then(|path| File::create(path).ok())
            .map(|file| Mutex::new(BufWriter::new(file)))
    });
    if let Some(sink) = sink {
        if let Ok(mut writer) = sink.lock() {
            let _ = writer.write_fmt(args).and_then(|()| writer.write_all(b"\n"));
        }
    }
}

fn dump_flush() {
    if let Some(Some(sink)) = DUMP.get() {
        if let Ok(mut writer) = sink.lock() {
            let _ = writer.flush();
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct EventKey {
    level: u32,
    short: bool,
    kind: NestDivergenceKind,
    seg_a: (usize, usize),
    interval_b: (usize, usize),
    interval_a: (usize, usize),
    turn_source: usize,
}

impl From<&NestCandidateEvent> for EventKey {
    fn from(event: &NestCandidateEvent) -> Self {
        Self {
            level: event.level,
            short: event.side == Side::Short,
            kind: event.kind,
            seg_a: event.seg_a,
            interval_b: event.interval_b,
            interval_a: event.interval_a,
            turn_source: event.turn_source,
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct ProviderAudit {
    snapshots: usize,
    views: usize,
    projection_too_short: usize,
    projection_invalid_seed: usize,
    projection_missing_carried_center: usize,
    projection_other: usize,
    snapshot_future_violations: usize,
}

impl ProviderAudit {
    fn observe_projection(&mut self, error: ProjectionError) {
        match error {
            ProjectionError::TooShort { .. } => self.projection_too_short += 1,
            ProjectionError::InvalidSeed { .. } => self.projection_invalid_seed += 1,
            ProjectionError::MissingCarriedCenter { .. } => {
                self.projection_missing_carried_center += 1
            }
            ProjectionError::InvalidLowerLeg { .. } => self.projection_other += 1,
        }
    }

    fn provider_complete(&self) -> bool {
        self.projection_too_short == 0
            && self.projection_invalid_seed == 0
            && self.projection_missing_carried_center == 0
            && self.projection_other == 0
    }
}

/// #97 身份键：与 [`NestEventIdentity`] 字段一一对应，用于对账链覆盖。
type IdentityKey = (u32, usize, (usize, usize));

#[derive(Debug, Default)]
struct YieldBook {
    candidates: BTreeMap<EventKey, usize>,
    divergences: BTreeMap<EventKey, usize>,
    terminal_confirmed: BTreeSet<EventKey>,
    cert_a: BTreeSet<String>,
    cert_b: BTreeSet<String>,
    cert_a_kind: BTreeMap<&'static str, usize>,
    cert_b_kind: BTreeMap<&'static str, usize>,
    d3_edges: usize,
    d3_violations: usize,
    /// #97: B 口径链实际吸收过的事件身份（任意 rung 位置）。
    covered_b: BTreeSet<IdentityKey>,
    /// #97: 走盘整块回退进料口的候选。
    intake_fallbacks: BTreeSet<EventKey>,
    /// #116: TERM 行去重集（dump 侧信道专用；与 terminal_confirmed 账本解耦——
    /// terminal_confirmed 仍仅终态快照入账，保持 p92 stdout 口径逐字不变）。
    term_seen: BTreeSet<EventKey>,
}

/// #116: TURN 游标（每级已落盘稳定完成块数 + 身份去重集）。与 p116 逐字一致。
#[derive(Debug, Default)]
struct TurnBook {
    done: BTreeMap<usize, usize>,
    seen: BTreeMap<usize, BTreeSet<(usize, usize, &'static str)>>,
}

impl TurnBook {
    /// 每 bar 调用：把每级「稳定完成」（≥2 后继块，见 p116 模块头稳定规则）的 typed 结构
    /// 终点落盘为 TURN 行。摊还 O(新稳定块数)；moves 无新增时 O(级别数)。
    fn observe(&mut self, classification: &classifier::Classification) {
        for (level, state) in classification.levels.iter().enumerate() {
            let m = state.moves.len();
            // 链尾两块（Active 尾 + 最近 Completed 块）仍可能触 frontier 重折，不落盘。
            let ready = m.saturating_sub(2);
            let done = self.done.entry(level).or_insert(0);
            if ready < *done {
                // 段账本回缩 / 全量重折护栏：游标归零重扫（seen 挡掉同身份重复行）。
                *done = 0;
            }
            let from = *done;
            *done = ready;
            if from >= ready {
                continue;
            }
            let seen = self.seen.entry(level).or_default();
            for block in &state.moves[from..ready] {
                let key = (block.start_center, block.end_center, move_kind_tag(block.kind));
                if !seen.insert(key) {
                    continue;
                }
                let start = state
                    .centers
                    .get(block.start_center)
                    .expect("MoveBlock.start_center 越界（decompose 不变量破裂）");
                let end = state
                    .centers
                    .get(block.end_center)
                    .expect("MoveBlock.end_center 越界（decompose 不变量破裂）");
                dump_line(format_args!(
                    "TURN level={} end={} start={} kind={}",
                    level,
                    end.end_index,
                    start.start_index,
                    move_kind_tag(block.kind),
                ));
            }
        }
    }
}

fn move_kind_tag(kind: MoveKind) -> &'static str {
    match kind {
        MoveKind::Trend => "Trend",
        MoveKind::Consolidation => "Consolidation",
    }
}

/// #116: DIV 行 kind 标签（沿用 p92 bucket 命名：趋势背驰=trend，盘整背驰=pan）。
fn nest_kind_tag(kind: NestDivergenceKind) -> &'static str {
    match kind {
        NestDivergenceKind::Trend => "trend",
        NestDivergenceKind::Consolidation => "pan",
    }
}

struct TerminalState {
    l0: ParseLayer,
    classification: classifier::Classification,
    tower: Vec<Rc<Vec<LeveledMove>>>,
    cache: classifier::TowerCache,
}

// ═══════════════════════════════════════════════════════════════════════════════
//  p123 稀疏化实装（唯一差异面，见模块头「稀疏化架构」）
// ═══════════════════════════════════════════════════════════════════════════════

/// per-level 派生缓存：run 分区 / lower legs / 端点水位线，全部用**内容快照值比对**做哨兵
///（设计 §4.1 Rc holding trick 的实装层替代，soundness 见模块头 090 声明）。
/// `self_gen`/`lower_gen` 在内容值变时 +1——run 条目只记代次，避免「partition 已同步但
/// 条目未重估」的错位（同步与重估解耦时代次差 ⟹ dirty，构造性无漏）。
#[derive(Debug, Default)]
struct LevelDerived {
    self_gen: u64,
    self_snap: Vec<LeveledMove>,
    lower_gen: u64,
    lower_snap: Vec<LeveledMove>,
    /// run_source_start（= run 首窗投影首 seed start_index = 目标 provider_window.0）→
    /// (start, end) 窗下标区间。与慢版 collect_target_candidates 分区扫描逐字同算法。
    run_ranges: BTreeMap<usize, (usize, usize)>,
    lower_legs: Vec<LowerLeg>,
    /// lower_legs 的 end_index（塔不变量：LeveledMove 序列按 end_index 升序，递归塔
    /// recursive_tower.rs:120；partition_point 直接用，不再排序）。
    lower_ends: Vec<usize>,
}

/// per-(level, run_source_start) 评估缓存条目（设计 §3 的 per-(level,run) 评估缓存）。
#[derive(Debug)]
struct RunEntry {
    /// 评估时 LevelDerived.self_gen / lower_gen（dirty 判据 (i)/(ii) 的代次锚）。
    self_gen: u64,
    lower_gen: u64,
    /// 评估时 as_of（dirty 判据 (iii) 的水位基线；构造上 (iii)⊂(ii)，见模块头证明）。
    last_as_of: usize,
    /// 上次评估产出 ∩ 评估时 pending（provide 输出原序）。judge_at 字段为评估时 as_of，
    /// 应用侧不消费（钟位取 trigger bar，与慢版 or_insert(index) 同口径）。
    events: Vec<NestCandidateEvent>,
}

/// 稀疏化计数（stderr 专用，不进验收面）：判据触发/重估/复用/TERM 反查现场。
#[derive(Debug, Default)]
struct SparseStats {
    triggers: usize,
    reevals: usize,
    reuses: usize,
    syncs_self: usize,
    syncs_lower: usize,
    /// (iii) 在 (ii) 干净下独立触发次数——构造性应为 0（模块头证明），非 0 即 bug。
    wm_cross_without_lower: usize,
    wm_cross_with_lower: usize,
    term_rechecks: usize,
    term_skips: usize,
    /// §6.3 shadow 对拍（P123_SHADOW=1）：强制重估次数 / dirty 漏判数 / TERM 门控漏判数。
    shadow_checks: usize,
    shadow_mismatches: usize,
    shadow_term_mismatches: usize,
    per_level_reevals: BTreeMap<usize, usize>,
    per_level_views_slow_would: BTreeMap<usize, usize>,
}

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("用法: p123_fast_replay <btc_1m_full.json>")?;
    if args.next().is_some() {
        return Err("参数过多".to_string());
    }
    let config = ThetaConfig::default();
    let loaded = load_bars(Path::new(&path), config.tick.tick_size)?;
    if loaded.bars.is_empty() {
        return Err("输入 bars 为空".to_string());
    }
    let max_bars = std::env::var("P116_MAX_BARS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .map_or(loaded.bars.len(), |value| value.min(loaded.bars.len()));

    println!(
        "P123_INPUT bars={} replay_bars={} first_date={} last_date={}",
        loaded.bars.len(),
        max_bars,
        loaded.first_date,
        loaded.last_date
    );
    println!(
        "P123_RULE cand=dir_and_comparable_and_extreme weak=divergence_stage interval_production=B typed=trend_or_consolidation d3=sidecar terminal=BspBits_confirm_side clock=first_prefix created_at=forbidden"
    );
    println!(
        "P123_PROBE turn=completed_typed_structure_ge2_successors turn_end=turning_center_end_index div=first_prefix_divergence_confirmed div_end=turn_source term=event_terminal_bits_first_seen term_bar=turn_source"
    );
    println!(
        "P123_SPARSE_MODE cache=per_level_run dirty=self_content,lower_content,leg_end_watermark,bsp_content term_recheck=fresh_or_book_changed trigger=forest_epoch+signal_signature_frozen"
    );

    let mut audit = ProviderAudit::default();
    let terminal = run_terminal_pass(&loaded.bars[..max_bars], &config)?;
    let (hist, close_src) = terminal.cache.causal_series();
    let dif = terminal.cache.macd_dif();
    let terminal_events =
        collect_snapshot_candidates(&terminal.tower, max_bars - 1, hist, dif, close_src, &mut audit)?;
    let mut targets = BTreeMap::new();
    for event in terminal_events.iter().flatten() {
        targets.insert(EventKey::from(event), *event);
    }
    let prefix_started = Instant::now();
    let (mut book, prefix_views, unresolved_targets, stats) =
        run_targeted_prefix_pass(&loaded.bars[..max_bars], &config, &targets)?;
    let prefix_elapsed = prefix_started.elapsed();
    audit.views += prefix_views;
    audit.snapshots += book.candidates.len();
    eprintln!(
        "P123_SPARSE_SUMMARY triggers={} reevals={} reuses={} syncs_self={} syncs_lower={} wm_cross_with_lower={} wm_cross_without_lower={} term_rechecks={} term_skips={} shadow_checks={} shadow_mismatches={} shadow_term_mismatches={} prefix_s={:.3}",
        stats.triggers,
        stats.reevals,
        stats.reuses,
        stats.syncs_self,
        stats.syncs_lower,
        stats.wm_cross_with_lower,
        stats.wm_cross_without_lower,
        stats.term_rechecks,
        stats.term_skips,
        stats.shadow_checks,
        stats.shadow_mismatches,
        stats.shadow_term_mismatches,
        prefix_elapsed.as_secs_f64(),
    );
    for (level, reevals) in &stats.per_level_reevals {
        let slow_would = stats.per_level_views_slow_would.get(level).copied().unwrap_or(0);
        eprintln!(
            "P123_SPARSE_LEVEL level={level} reevals={reevals} slow_would_views={slow_would}"
        );
    }

    let l0 = terminal.l0;
    let classification = terminal.classification;
    let tower = terminal.tower;
    let cache = terminal.cache;
    let mut final_events = terminal_events;
    for events in &mut final_events {
        for event in events {
            let key = EventKey::from(&*event);
            event.judge_at = book
                .divergences
                .get(&key)
                .or_else(|| book.candidates.get(&key))
                .copied()
                .unwrap_or(max_bars - 1);
        }
    }
    observe_snapshot(
        &classification,
        final_events,
        max_bars - 1,
        &mut book,
        &mut audit,
    );
    let (full_classification, full_tower) = classifier::classify_with_tower(&l0, &config);
    let classification_diff = usize::from(full_classification != classification);
    let tower_diff = usize::from(full_tower != tower);
    let mut old_semantic_diff = tower_diff;
    let mut lifecycle_diff = 0usize;
    let levels = full_classification
        .levels
        .len()
        .max(classification.levels.len());
    for level in 0..levels {
        match (
            full_classification.levels.get(level),
            classification.levels.get(level),
        ) {
            (Some(full), Some(incremental)) => {
                old_semantic_diff += usize::from(full.moves != incremental.moves)
                    + usize::from(full.centers != incremental.centers)
                    + usize::from(full.bsp != incremental.bsp)
                    + usize::from(full.pan_div != incremental.pan_div);
                lifecycle_diff += usize::from(full.cp_ownership != incremental.cp_ownership);
            }
            _ => old_semantic_diff += 1,
        }
    }
    if classification_diff != 0 {
        let levels = full_classification
            .levels
            .len()
            .max(classification.levels.len());
        for level in 0..levels {
            let full = full_classification.levels.get(level);
            let incremental = classification.levels.get(level);
            let summary = match (full, incremental) {
                (Some(full), Some(incremental)) => format!(
                    "moves={} centers={} ownership={} bsp={} pan={}",
                    usize::from(full.moves != incremental.moves),
                    usize::from(full.centers != incremental.centers),
                    usize::from(full.cp_ownership != incremental.cp_ownership),
                    usize::from(full.bsp != incremental.bsp),
                    usize::from(full.pan_div != incremental.pan_div),
                ),
                _ => "missing_level=1".to_string(),
            };
            println!("P123_BIT_DIFF_LEVEL L{level} {summary}");
        }
    }
    let old_events =
        classifier::cand_delta_tower_cached(&l0, &classification, &tower, &config, &cache);
    let old_candidates = old_events
        .iter()
        .flatten()
        .filter(|event| event.cand_delta)
        .count();
    let old_terminal = old_events
        .iter()
        .enumerate()
        .map(|(level, events)| {
            events
                .iter()
                .filter(|event| {
                    event.cand_delta
                        && terminal_bits_old(
                            &classification,
                            level,
                            event.c_episode_start,
                            event.confirm_src,
                            event.side,
                            event.b_parent.map(|p| p.source_interval.0),
                        )
                        .is_some()
                })
                .count()
        })
        .sum::<usize>();
    let mut old_certificates = 0usize;
    for exec in 0..old_events.len() {
        for top in exec..old_events.len() {
            old_certificates += assemble_certificates_snapshot(&old_events, exec, top, |event| {
                terminal_bits_old(
                    &classification,
                    exec,
                    event.c_episode_start,
                    event.confirm_src,
                    event.side,
                    event.b_parent.map(|p| p.source_interval.0),
                )
            })
            .len();
        }
    }

    let trend_candidates = book
        .candidates
        .keys()
        .filter(|key| key.kind == NestDivergenceKind::Trend)
        .count();
    let pan_candidates = book.candidates.len() - trend_candidates;
    let trend_divergences = book
        .divergences
        .keys()
        .filter(|key| key.kind == NestDivergenceKind::Trend)
        .count();
    let pan_divergences = book.divergences.len() - trend_divergences;
    let d3_rate = if book.d3_edges == 0 {
        0.0
    } else {
        book.d3_violations as f64 / book.d3_edges as f64
    };

    println!(
        "P123_YIELD candidates={} trend_candidates={} pan_candidates={} divergence_confirmed={} trend_divergence={} pan_divergence={} terminal_confirmed={}",
        book.candidates.len(),
        trend_candidates,
        pan_candidates,
        book.divergences.len(),
        trend_divergences,
        pan_divergences,
        book.terminal_confirmed.len()
    );
    println!(
        "P123_CERT caliber_A={} A_trend={} A_pan={} A_mixed={} caliber_B={} B_trend={} B_pan={} B_mixed={}",
        book.cert_a.len(),
        book.cert_a_kind.get("trend").copied().unwrap_or(0),
        book.cert_a_kind.get("pan").copied().unwrap_or(0),
        book.cert_a_kind.get("mixed").copied().unwrap_or(0),
        book.cert_b.len(),
        book.cert_b_kind.get("trend").copied().unwrap_or(0),
        book.cert_b_kind.get("pan").copied().unwrap_or(0),
        book.cert_b_kind.get("mixed").copied().unwrap_or(0),
    );
    println!(
        "P123_D3 edges={} violations={} rate={:.9}",
        book.d3_edges, book.d3_violations, d3_rate
    );
    println!(
        "P123_BASELINE old_candidates={} old_terminal_confirmed={} old_certificates={} new_candidates={} new_terminal_confirmed={} new_B_certificates={}",
        old_candidates,
        old_terminal,
        old_certificates,
        book.candidates.len(),
        book.terminal_confirmed.len(),
        book.cert_b.len()
    );
    println!(
        "P123_PROVIDER snapshots={} views={} too_short={} invalid_seed={} missing_carried_center={} other={} unresolved_targets={} complete={}",
        audit.snapshots,
        audit.views,
        audit.projection_too_short,
        audit.projection_invalid_seed,
        audit.projection_missing_carried_center,
        audit.projection_other,
        unresolved_targets,
        audit.provider_complete() && unresolved_targets == 0
    );
    println!(
        "P123_SNAPSHOT future_violations={} created_at_reads=0 no_forward={}",
        audit.snapshot_future_violations,
        audit.snapshot_future_violations == 0
    );
    println!(
        "P123_BIT_EXACT old_path_diff={} tower_diff={} moves_centers_bsp_pan_diff={} lifecycle_cp_ownership_diff={} classification_total_diff={}",
        old_semantic_diff,
        tower_diff,
        old_semantic_diff.saturating_sub(tower_diff),
        lifecycle_diff,
        classification_diff
    );
    println!(
        "P123_R7 provider_complete={} definition_faithful=true snapshot_no_forward={} B_zero={}",
        audit.provider_complete() && unresolved_targets == 0,
        audit.snapshot_future_violations == 0,
        book.cert_b.is_empty()
    );
    // #97: 遗漏对账 —— 钟位可证的候选却从未被任何 B 链吸收，逐条枚举。
    let missed: Vec<&EventKey> = book
        .terminal_confirmed
        .iter()
        .filter(|key| !book.covered_b.contains(&(key.level, key.turn_source, key.interval_b)))
        .collect();
    println!(
        "P123_MISSED terminal_confirmed={} covered_b={} intake_fallback_events={} missed={}",
        book.terminal_confirmed.len(),
        book.covered_b.len(),
        book.intake_fallbacks.len(),
        missed.len()
    );
    for key in &missed {
        println!(
            "P123_MISSED_EVENT level={} kind={:?} short={} seg_a={:?} interval_b={:?} interval_a={:?} turn_source={} intake_fallback={}",
            key.level,
            key.kind,
            key.short,
            key.seg_a,
            key.interval_b,
            key.interval_a,
            key.turn_source,
            book.intake_fallbacks.contains(*key)
        );
    }
    dump_flush();
    Ok(())
}

fn signal_signature(
    classification: &classifier::Classification,
) -> Vec<(usize, usize, usize, usize)> {
    classification
        .levels
        .iter()
        .map(|level| {
            (
                level.bsp.len(),
                level
                    .bsp
                    .last()
                    .map_or(usize::MAX, |point| point.source_index),
                level.pan_div.len(),
                level
                    .pan_div
                    .last()
                    .map_or(usize::MAX, |cert| cert.source_index),
            )
        })
        .collect()
}

fn run_terminal_pass(bars: &[Bar], config: &ThetaConfig) -> Result<TerminalState, String> {
    let mut parser = ParseLayerIncr::new(config);
    let mut cache = classifier::TowerCache::new();
    let mut terminal = None;
    let started = Instant::now();
    for (index, bar) in bars.iter().copied().enumerate() {
        let l0 = parser.append(bar);
        let (classification, tower) =
            classifier::classify_with_tower_incremental(&l0, config, &mut cache);
        if index > 0 && index % 500_000 == 0 {
            eprintln!(
                "P123_TERMINAL_PROGRESS bar={index}/{} elapsed={:.1}s",
                bars.len(),
                started.elapsed().as_secs_f64()
            );
        }
        if index + 1 == bars.len() {
            terminal = Some((l0, classification, tower));
        }
    }
    let (l0, classification, tower) = terminal.ok_or("空 replay")?;
    Ok(TerminalState {
        l0,
        classification,
        tower,
        cache,
    })
}

/// p123 稀疏 targeted prefix pass：与 p116 `run_targeted_prefix_pass` 同一 trigger 判据、
/// 同一 pending 生命周期、同一事件应用循环（下方逐行保留并标注），仅「每次 trigger 对全部
/// arrived run 全量重估」换为 per-(level,run) 缓存 + 四类 dirty 判据（模块头）。
#[allow(clippy::too_many_arguments)]
fn run_targeted_prefix_pass(
    bars: &[Bar],
    config: &ThetaConfig,
    targets: &BTreeMap<EventKey, NestCandidateEvent>,
) -> Result<(YieldBook, usize, usize, SparseStats), String> {
    let mut parser = ParseLayerIncr::new(config);
    let mut cache = classifier::TowerCache::new();
    let mut book = YieldBook::default();
    let mut turns = TurnBook::default();
    let mut pending: BTreeSet<EventKey> = targets.keys().cloned().collect();
    let mut views = 0usize;
    let mut last_trigger = None;
    let started = Instant::now();
    // #103 侧信道：P116_CKPT=<K> 时每 K bars 做一次全量快照装配并 dump（只写不判）。
    let ckpt_every: usize = std::env::var("P116_CKPT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    // ── p123 稀疏状态 ──
    let mut derived: BTreeMap<usize, LevelDerived> = BTreeMap::new();
    let mut entries: BTreeMap<(usize, usize), RunEntry> = BTreeMap::new();
    // 判据 (iv)：levels[book_level].bsp 上次**已查**内容快照（BspPoint PartialEq 排除
    // force——force 不进 terminal_bits_in_book 判定（nest.rs:520-534 只读
    // source_index/bits.confirm_side），值比对对 TERM 语义充分）。
    let mut bsp_snaps: BTreeMap<usize, Vec<BspPoint>> = BTreeMap::new();
    let mut stats = SparseStats::default();
    // §6.3 shadow 强制对拍开关（长期回归开关；stderr 诊断，不进 dump/stdout 验收面）。
    let shadow = std::env::var("P123_SHADOW").ok().as_deref() == Some("1");
    for (index, bar) in bars.iter().copied().enumerate() {
        let l0 = parser.append(bar);
        let (classification, tower) =
            classifier::classify_with_tower_incremental(&l0, config, &mut cache);
        // #116: TURN 逐 bar 观察（摊还 O(新稳定块数)，不经 trigger 门——pending 空后仍落盘）。
        turns.observe(&classification);
        let trigger = (cache.forest_epoch(), signal_signature(&classification));
        if last_trigger.as_ref() != Some(&trigger) && !pending.is_empty() {
            stats.triggers += 1;
            let (hist, close_src) = cache.causal_series();
            let dif = cache.macd_dif();
            // arrived 分组：与慢版 collect_target_candidates 首段逐行一致。
            let mut runs_by_level: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
            for key in &pending {
                let Some(event) = targets.get(key) else { continue };
                // snapshot 无前视的结构下，turn_source 尚未到达时该对象不可能成为 Cand。
                // 提前投影这些终态目标只增加扫描量，不可能改变首次可证钟。
                if event.turn_source <= index {
                    runs_by_level
                        .entry(event.level as usize)
                        .or_default()
                        .insert(event.provider_window.0);
                }
            }
            // 判据 (iv) 预备：本 trigger 全部 arrived 级的账本级做值比对并刷新快照。
            // 「变 ⟹ 反查」覆盖慢版每个可能新 Some 的 trigger；账本只在 trigger bar 可变
            // （模块头 (iv) 签字），故跨 trigger 快照比对 = 跨 bar 比对。
            let mut bsp_changed: BTreeMap<usize, bool> = BTreeMap::new();
            for &level in runs_by_level.keys() {
                let Some(book_level) = event_bsp_book_level(level as u32) else {
                    continue;
                };
                let Some(state) = classification.levels.get(book_level) else {
                    continue;
                };
                let current = &state.bsp[..];
                let changed = bsp_snaps
                    .get(&book_level)
                    .is_none_or(|snap| snap[..] != *current);
                if changed {
                    bsp_snaps.insert(book_level, current.to_vec());
                }
                bsp_changed.insert(book_level, changed);
            }
            let mut out: Vec<NestCandidateEvent> = Vec::new();
            let mut fresh_levels: BTreeSet<usize> = BTreeSet::new();
            for (level, run_sources) in runs_by_level {
                if level == 0 || level >= tower.len() {
                    continue;
                }
                // ── 派生缓存同步（哨兵 = 内容快照值比对；变更才重建分区/legs）──
                let level_derived = derived.entry(level).or_default();
                if level_derived.self_snap[..] != tower[level][..] {
                    level_derived.self_snap = tower[level][..].to_vec();
                    level_derived.self_gen += 1;
                    level_derived.run_ranges = build_run_ranges(&tower[level]);
                    stats.syncs_self += 1;
                }
                if level_derived.lower_snap[..] != tower[level - 1][..] {
                    level_derived.lower_snap = tower[level - 1][..].to_vec();
                    level_derived.lower_gen += 1;
                    level_derived.lower_legs = lower_legs_from(&tower[level - 1])
                        .map_err(|error| format!("L{level} targeted lower legs 失败: {error:?}"))?;
                    level_derived.lower_ends =
                        level_derived.lower_legs.iter().map(|leg| leg.end_index).collect();
                    stats.syncs_lower += 1;
                }
                let level_derived = &derived[&level];
                for run_source_start in run_sources {
                    // 慢版：run 不在当前分区 ⟹ 本 trigger 无产出（continue）。条目保留不应用——
                    // run 重现时代次差（分区已随内容变同步过）⟹ dirty ⟹ 重估，无陈旧复用。
                    let Some(&(start, end)) = level_derived.run_ranges.get(&run_source_start)
                    else {
                        continue;
                    };
                    *stats
                        .per_level_views_slow_would
                        .entry(level)
                        .or_default() += 1;
                    let watermark_crossed = entries
                        .get(&(level, run_source_start))
                        .is_some_and(|entry| {
                            let before = level_derived
                                .lower_ends
                                .partition_point(|&end_index| end_index <= entry.last_as_of);
                            let now = level_derived
                                .lower_ends
                                .partition_point(|&end_index| end_index <= index);
                            now != before
                        });
                    let dirty = match entries.get(&(level, run_source_start)) {
                        None => true, // 冷条目：首次评估
                        Some(entry) => {
                            let self_changed = entry.self_gen != level_derived.self_gen;
                            let lower_changed = entry.lower_gen != level_derived.lower_gen;
                            if watermark_crossed {
                                if lower_changed {
                                    stats.wm_cross_with_lower += 1;
                                } else {
                                    // 模块头证明 (iii)⊂(ii)：此计数恒 0，非 0 即 bug。
                                    stats.wm_cross_without_lower += 1;
                                }
                            }
                            self_changed || lower_changed || watermark_crossed
                        }
                    };
                    if dirty {
                        let events = evaluate_run(
                            level,
                            &tower[level],
                            (start, end),
                            &level_derived.lower_legs,
                            index,
                            hist,
                            dif,
                            close_src,
                            &pending,
                        )?;
                        views += 1;
                        stats.reevals += 1;
                        *stats.per_level_reevals.entry(level).or_default() += 1;
                        fresh_levels.insert(level);
                        entries.insert(
                            (level, run_source_start),
                            RunEntry {
                                self_gen: level_derived.self_gen,
                                lower_gen: level_derived.lower_gen,
                                last_as_of: index,
                                events,
                            },
                        );
                    } else {
                        stats.reuses += 1;
                    }
                    // 应用（保序）：缓存事件 ∩ 当前 pending。pending 只缩不增 ⟹
                    // 等价慢版「本 trigger 全量重估后 ∩ pending」（模块头判据完备性）。
                    let applied_start = out.len();
                    if let Some(entry) = entries.get(&(level, run_source_start)) {
                        out.extend(
                            entry
                                .events
                                .iter()
                                .copied()
                                .filter(|event| pending.contains(&EventKey::from(event))),
                        );
                    }
                    // §6.3 shadow 强制对拍（P123_SHADOW=1 开启，长期回归开关）：无视 dirty
                    // 判定强制全量重估，与缓存路径应用集逐字比对（judge_at 是 as_of 戳，
                    // 不消费，不参与比对）。任何 mismatch = dirty 判据漏判现场（090 停线）。
                    if shadow {
                        let forced = evaluate_run(
                            level,
                            &tower[level],
                            (start, end),
                            &level_derived.lower_legs,
                            index,
                            hist,
                            dif,
                            close_src,
                            &pending,
                        )?;
                        stats.shadow_checks += 1;
                        let applied = &out[applied_start..];
                        let same = applied.len() == forced.len()
                            && applied.iter().zip(forced.iter()).all(|(a, b)| {
                                EventKey::from(a) == EventKey::from(b)
                                    && a.divergence_confirmed == b.divergence_confirmed
                            });
                        if !same {
                            stats.shadow_mismatches += 1;
                            if stats.shadow_mismatches <= 20 {
                                eprintln!(
                                    "P123_SHADOW_MISMATCH level={level} run_source={run_source_start} as_of={index} dirty={dirty} applied={:?} forced={:?}",
                                    applied
                                        .iter()
                                        .map(|e| (EventKey::from(e), e.divergence_confirmed))
                                        .collect::<Vec<_>>(),
                                    forced
                                        .iter()
                                        .map(|e| (EventKey::from(e), e.divergence_confirmed))
                                        .collect::<Vec<_>>(),
                                );
                            }
                        }
                    }
                }
            }
            // ── 事件应用循环：与 p116 逐行一致，仅 TERM 查法加判据 (iv) 门控 ──
            for event in out {
                let key = EventKey::from(&event);
                if !pending.contains(&key) {
                    continue;
                }
                book.candidates.entry(key.clone()).or_insert(index);
                let target = targets.get(&key).expect("pending target");
                if event.divergence_confirmed {
                    // #116: DIV = divergences 账本首次插入瞬间（账本语义同 p92 or_insert）。
                    if !book.divergences.contains_key(&key) {
                        dump_line(format_args!(
                            "DIV level={} end={} kind={} side={:?}",
                            event.level,
                            event.turn_source,
                            nest_kind_tag(event.kind),
                            event.side,
                        ));
                    }
                    book.divergences.entry(key.clone()).or_insert(index);
                }
                // #116: TERM = 事件键首次终端 bits 确认（dump 专用 term_seen，不动账本）。
                // p123 判据 (iv)：fresh（本 trigger 重估过 ⟹ 窗口可能变）∨ 账本内容变
                // ⟹ 反查；其余情形结果为上次已查的同一 None（纯函数同输入），跳过逐位等价。
                let recheck = fresh_levels.contains(&(event.level as usize))
                    || event_bsp_book_level(event.level)
                        .is_some_and(|book_level| bsp_changed.get(&book_level).copied().unwrap_or(true));
                if recheck {
                    stats.term_rechecks += 1;
                    if !book.term_seen.contains(&key)
                        && terminal_bits_new(&classification, &event).is_some()
                    {
                        book.term_seen.insert(key.clone());
                        dump_line(format_args!(
                            "TERM level={} bar={} side={:?}",
                            event.level, event.turn_source, event.side,
                        ));
                    }
                } else {
                    stats.term_skips += 1;
                    // §6.3 shadow (iv) 门控对拍：跳过的 TERM 检查若本 trigger 实为 Some 且
                    // term_seen 未录 ⟹ 慢版本 trigger 会首见而 sparse 漏 = 判据 (iv) 漏判。
                    if shadow
                        && !book.term_seen.contains(&key)
                        && terminal_bits_new(&classification, &event).is_some()
                    {
                        stats.shadow_term_mismatches += 1;
                        if stats.shadow_term_mismatches <= 20 {
                            eprintln!(
                                "P123_SHADOW_TERM_MISMATCH level={} as_of={index} key={key:?}",
                                event.level,
                            );
                        }
                    }
                }
                if event.divergence_confirmed == target.divergence_confirmed {
                    pending.remove(&key);
                }
            }
            last_trigger = Some(trigger);
        }
        if ckpt_every > 0 && index > 0 && index % ckpt_every == 0 {
            let (hist, close_src) = cache.causal_series();
            let dif = cache.macd_dif();
            let mut ckpt_audit = ProviderAudit::default();
            match collect_snapshot_candidates(&tower, index, hist, dif, close_src, &mut ckpt_audit) {
                Ok(by_level) => checkpoint_certificates(&by_level, &classification, index),
                Err(error) => {
                    dump_line(format_args!("CKPT_ERR as_of={index} err={error}"));
                }
            }
        }
        if index > 0 && index % 500_000 == 0 {
            eprintln!(
                "P123_PREFIX_PROGRESS bar={index}/{} elapsed={:.1}s pending={}/{} views={views} triggers={} reevals={}",
                bars.len(),
                started.elapsed().as_secs_f64(),
                pending.len(),
                targets.len(),
                stats.triggers,
                stats.reevals,
            );
        }
    }
    Ok((book, views, pending.len(), stats))
}

/// run 分区扫描（慢版 collect_target_candidates 内联块的提取，逐字同算法）：
/// run_source_start = run 首窗单窗投影首 seed 的 start_index。
fn build_run_ranges(windows: &Rc<Vec<LeveledMove>>) -> BTreeMap<usize, (usize, usize)> {
    let mut run_ranges = BTreeMap::new();
    let mut run_start = None;
    let mut run_source = None;
    for index in 0..=windows.len() {
        let seed_start = (index < windows.len())
            .then(|| {
                project_extended_windows_carried_only(std::slice::from_ref(&windows[index])).ok()
            })
            .flatten()
            .and_then(|projection| projection.seeds.first().map(|seed| seed.start_index));
        match (run_start, seed_start) {
            (None, Some(source)) => {
                run_start = Some(index);
                run_source = Some(source);
            }
            (Some(start), None) => {
                run_ranges.insert(run_source.expect("合法 run 有 source"), (start, index));
                run_start = None;
                run_source = None;
            }
            _ => {}
        }
    }
    run_ranges
}

/// per-run 完整评估（慢版 collect_target_candidates per-run 块的提取，逐字同调用链）：
/// run 投影 → decompose → assemble_level_view → provide_nest_candidate_events → ∩ pending。
/// 产出保持 provide 输出序（turn_source/interval_b/kind/side 排序，与慢版事件拼接序一致）。
#[allow(clippy::too_many_arguments)]
fn evaluate_run(
    level: usize,
    windows: &Rc<Vec<LeveledMove>>,
    run: (usize, usize),
    lower: &[LowerLeg],
    as_of: usize,
    hist: &[f64],
    dif: &[f64],
    close_src: &[usize],
    pending: &BTreeSet<EventKey>,
) -> Result<Vec<NestCandidateEvent>, String> {
    let (start, end) = run;
    let projection = project_extended_windows_carried_only(&windows[start..end])
        .map_err(|error| format!("L{level} targeted projection 失败: {error:?}"))?;
    let centers: Vec<_> = projection.seeds.iter().map(|seed| seed.center).collect();
    let blocks = decompose::decompose(&centers);
    let query = LevelViewQuery {
        level: level as u32,
        coordinate_window: CoordinateWindow {
            start: projection.seeds.first().expect("nonempty").start_index,
            end: projection.seeds.last().expect("nonempty").end_index,
        },
        as_of,
        version: C2VersionTuple::auto_pairing(),
    };
    let view = assemble_level_view(
        C2LevelViewConfig { enabled: true },
        query,
        LevelViewMaterial {
            projection: ProjectionMaterial::ExactThree(&projection),
            move_blocks: &blocks,
            lower_legs: lower,
            hist,
            dif,
            close_src,
        },
    )
    .map_err(|error| format!("L{level} targeted C2 assemble 失败: {error:?}"))?;
    Ok(provide_nest_candidate_events(
        level as u32,
        &projection,
        &blocks,
        lower,
        &view,
        hist,
        dif,
        close_src,
    )
    .into_iter()
    .filter(|event| pending.contains(&EventKey::from(event)))
    .collect())
}

fn collect_snapshot_candidates(
    tower: &[Rc<Vec<LeveledMove>>],
    as_of: usize,
    hist: &[f64],
    dif: &[f64],
    close_src: &[usize],
    audit: &mut ProviderAudit,
) -> Result<Vec<Vec<NestCandidateEvent>>, String> {
    audit.snapshots += 1;
    let mut by_level = vec![Vec::new(); tower.len()];
    for level in 1..tower.len() {
        let lower = lower_legs_from(&tower[level - 1])
            .map_err(|error| format!("L{level} lower legs 失败: {error:?}"))?;
        let windows = &tower[level];
        let mut run_start = None;
        for index in 0..=windows.len() {
            let valid = if index < windows.len() {
                match project_extended_windows_carried_only(std::slice::from_ref(&windows[index])) {
                    Ok(_) => true,
                    Err(error) => {
                        audit.observe_projection(error);
                        false
                    }
                }
            } else {
                false
            };
            match (run_start, valid) {
                (None, true) => run_start = Some(index),
                (Some(start), false) => {
                    let projection = project_extended_windows_carried_only(&windows[start..index])
                        .map_err(|error| format!("L{level} run projection 失败: {error:?}"))?;
                    let centers: Vec<_> = projection.seeds.iter().map(|seed| seed.center).collect();
                    let blocks = decompose::decompose(&centers);
                    let start_source = projection.seeds.first().expect("nonempty run").start_index;
                    let end_source = projection.seeds.last().expect("nonempty run").end_index;
                    let query = LevelViewQuery {
                        level: level as u32,
                        coordinate_window: CoordinateWindow {
                            start: start_source,
                            end: end_source,
                        },
                        as_of,
                        version: C2VersionTuple::auto_pairing(),
                    };
                    let view = assemble_level_view(
                        C2LevelViewConfig { enabled: true },
                        query,
                        LevelViewMaterial {
                            projection: ProjectionMaterial::ExactThree(&projection),
                            move_blocks: &blocks,
                            lower_legs: &lower,
                            hist,
                            dif,
                            close_src,
                        },
                    )
                    .map_err(|error| format!("L{level} C2 assemble 失败: {error:?}"))?;
                    audit.views += 1;
                    by_level[level].extend(provide_nest_candidate_events(
                        level as u32,
                        &projection,
                        &blocks,
                        &lower,
                        &view,
                        hist,
                        dif,
                        close_src,
                    ));
                    run_start = None;
                }
                _ => {}
            }
        }
    }
    for events in &mut by_level {
        events.sort_by_key(|event| EventKey::from(&*event));
        events.dedup_by(|left, right| EventKey::from(&*left) == EventKey::from(&*right));
    }
    Ok(by_level)
}

fn observe_snapshot(
    classification: &classifier::Classification,
    mut current: Vec<Vec<NestCandidateEvent>>,
    as_of: usize,
    book: &mut YieldBook,
    audit: &mut ProviderAudit,
) {
    for events in &mut current {
        for event in events {
            if event.interval_a.1 > as_of || event.interval_b.1 > as_of || event.turn_source > as_of
            {
                audit.snapshot_future_violations += 1;
            }
            let key = EventKey::from(&*event);
            if event.intake_fallback && book.intake_fallbacks.insert(key.clone()) {
                dump_line(format_args!(
                    "FALLBACK as_of={} level={} kind={:?} side={:?} turn_source={} seg_a={:?} interval_a={:?} interval_b={:?} divergence_confirmed={} judge_at={}",
                    as_of,
                    event.level,
                    event.kind,
                    event.side,
                    event.turn_source,
                    event.seg_a,
                    event.interval_a,
                    event.interval_b,
                    event.divergence_confirmed,
                    event.judge_at,
                ));
            }
            let candidate_at = *book.candidates.entry(key.clone()).or_insert(as_of);
            if event.interval_a.1 > candidate_at
                || event.interval_b.1 > candidate_at
                || event.turn_source > candidate_at
            {
                audit.snapshot_future_violations += 1;
            }
            if event.divergence_confirmed {
                // #116: DIV 终态兜底（prefix 未观察到的首插同样落盘；账本语义同 p92）。
                if !book.divergences.contains_key(&key) {
                    dump_line(format_args!(
                        "DIV level={} end={} kind={} side={:?}",
                        event.level,
                        event.turn_source,
                        nest_kind_tag(event.kind),
                        event.side,
                    ));
                }
                let first = *book.divergences.entry(key.clone()).or_insert(as_of);
                event.judge_at = first;
                if terminal_bits_new(classification, event).is_some() {
                    book.terminal_confirmed.insert(key.clone());
                    // #116: TERM 终态兜底（term_seen 全程去重）。
                    if book.term_seen.insert(key.clone()) {
                        dump_line(format_args!(
                            "TERM level={} bar={} side={:?}",
                            event.level, event.turn_source, event.side,
                        ));
                    }
                }
            } else {
                event.judge_at = *book.candidates.get(&key).expect("inserted");
            }
        }
    }
    for exec in 1..current.len() {
        for top in exec..current.len() {
            observe_certificates(
                &current,
                classification,
                exec,
                top,
                as_of,
                NestIntervalCaliber::A,
                &mut book.cert_a,
                &mut book.cert_a_kind,
                &mut book.d3_edges,
                &mut book.d3_violations,
                false,
                None,
            );
            observe_certificates(
                &current,
                classification,
                exec,
                top,
                as_of,
                NestIntervalCaliber::B,
                &mut book.cert_b,
                &mut book.cert_b_kind,
                &mut book.d3_edges,
                &mut book.d3_violations,
                true,
                Some(&mut book.covered_b),
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn observe_certificates(
    events: &[Vec<NestCandidateEvent>],
    classification: &classifier::Classification,
    exec: usize,
    top: usize,
    as_of: usize,
    caliber: NestIntervalCaliber,
    seen: &mut BTreeSet<String>,
    kinds: &mut BTreeMap<&'static str, usize>,
    d3_edges: &mut usize,
    d3_violations: &mut usize,
    count_d3: bool,
    mut covered: Option<&mut BTreeSet<IdentityKey>>,
) {
    let certificates = assemble_typed_certificates(events, exec, top, caliber, |event| {
        terminal_bits_new(classification, event)
    });
    for certificate in certificates {
        if let Some(covered) = covered.as_deref_mut() {
            for identity in certificate.identities() {
                covered.insert((identity.level, identity.turn_source, identity.interval_b));
            }
        }
        let key = certificate_key(exec, top, &certificate);
        if seen.insert(key) {
            let bucket = certificate_kind(&certificate);
            *kinds.entry(bucket).or_default() += 1;
            if count_d3 {
                let (edges, violations) = certificate.d3_descent_stats();
                *d3_edges += edges;
                *d3_violations += violations;
            }
            // #98/#99 侧信道：身份向量（高→低，含基例）为 A/B 跨口径对账主键；
            // judge_at 向量供 D3 逐边离线复算。只写不判。
            let ids = certificate
                .identities()
                .iter()
                .map(|id| format!("{}:{}:{}-{}", id.level, id.turn_source, id.interval_b.0, id.interval_b.1))
                .collect::<Vec<_>>()
                .join("|");
            let kinds_str = certificate
                .kinds()
                .iter()
                .map(|kind| format!("{kind:?}"))
                .collect::<Vec<_>>()
                .join(",");
            let clocks = certificate
                .judge_at()
                .iter()
                .map(|clock| clock.to_string())
                .collect::<Vec<_>>()
                .join(",");
            dump_line(format_args!(
                "CERT caliber={:?} exec={} top={} as_of={} side={:?} bucket={} kinds={} judge_at={} ids={}",
                certificate.caliber(),
                exec,
                top,
                as_of,
                certificate.certificate().side(),
                bucket,
                kinds_str,
                clocks,
                ids,
            ));
        }
    }
}

/// #103 侧信道：检查点全量证书导出（只写不判，不触碰 book/seen 等主路径状态）。
/// 与 observe_certificates 的差异：seen 为检查点局部——每个检查点导出该时刻完整在场集合，
/// 供离线做存在性 diff（主键 ids 身份向量）；judge_at 取快照原值，不参与对账。
fn checkpoint_certificates(
    events: &[Vec<NestCandidateEvent>],
    classification: &classifier::Classification,
    as_of: usize,
) {
    for caliber in [NestIntervalCaliber::A, NestIntervalCaliber::B] {
        let mut seen = BTreeSet::new();
        let mut total = 0usize;
        for exec in 1..events.len() {
            for top in exec..events.len() {
                let certificates =
                    assemble_typed_certificates(events, exec, top, caliber, |event| {
                        terminal_bits_new(classification, event)
                    });
                for certificate in certificates {
                    let key = certificate_key(exec, top, &certificate);
                    if !seen.insert(key) {
                        continue;
                    }
                    total += 1;
                    let ids = certificate
                        .identities()
                        .iter()
                        .map(|id| {
                            format!(
                                "{}:{}:{}-{}",
                                id.level, id.turn_source, id.interval_b.0, id.interval_b.1
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("|");
                    dump_line(format_args!(
                        "CKPT caliber={:?} as_of={} exec={} top={} side={:?} bucket={} ids={}",
                        certificate.caliber(),
                        as_of,
                        exec,
                        top,
                        certificate.certificate().side(),
                        certificate_kind(&certificate),
                        ids,
                    ));
                }
            }
        }
        dump_line(format_args!(
            "CKPT_STATS caliber={caliber:?} as_of={as_of} certs={total}"
        ));
    }
}

fn certificate_kind(certificate: &TypedNestCertificate) -> &'static str {
    let trend = certificate
        .kinds()
        .iter()
        .any(|kind| *kind == NestDivergenceKind::Trend);
    let pan = certificate
        .kinds()
        .iter()
        .any(|kind| *kind == NestDivergenceKind::Consolidation);
    match (trend, pan) {
        (true, false) => "trend",
        (false, true) => "pan",
        _ => "mixed",
    }
}

fn certificate_key(exec: usize, top: usize, certificate: &TypedNestCertificate) -> String {
    let cert = certificate.certificate();
    let rungs = cert
        .rungs()
        .iter()
        .map(|rung| (rung.interval(), rung.child_interval()))
        .collect::<Vec<_>>();
    // #97: 身份标签参与去重键，同结构不同 provider 身份的链不互相吞并。
    let ids = certificate
        .identities()
        .iter()
        .map(|id| (id.level, id.turn_source, id.interval_b))
        .collect::<Vec<_>>();
    format!(
        "{exec}:{top}:{:?}:{:?}:{:?}:{:?}:{:?}:{ids:?}",
        certificate.caliber(),
        cert.side(),
        cert.base_interval(),
        rungs,
        certificate.kinds()
    )
}

/// ★p117 T1 终端背书生产口径常数（bsp-terminal-endorsement-ruling-20260718 裁决2）：
/// C-b = `[c_start, t*]` 窗口最早 confirm_side 点。C-a（`TerminalMatch::Exact`）保留为
/// 敏感性对照口径——与 p92/p116 同一常数同一切换点，各 bin 查法保持一致。
const TERMINAL_MATCH: TerminalMatch = TerminalMatch::CWindow;

/// 终端背书查法（生产）：委托 lib 单一来源 `nest::terminal_bits_at_event`——账本级别移位
/// `levels[ℓ-1].bsp` + 口径常数 [`TERMINAL_MATCH`]。与 p116 同名函数逐字同口径。
fn terminal_bits_new(
    classification: &classifier::Classification,
    event: &NestCandidateEvent,
) -> Option<BspBits> {
    // 关③ P3：lib 返回形状扩为 `TerminalEndorsement`（bits + owner start_index）——
    // 生产装配消费 bits 层；owner 两维构成归收紧后重放审计读数，非本 bin 职责。
    terminal_bits_at_event(classification, event, TERMINAL_MATCH).map(|t| t.bits)
}

/// 旧事件路径（`CandDeltaEvent`，P1 基线对账/P123_BASELINE 类审计）的终端查法：同一级别
/// 移位 + C-b 口径平移——账本 = `levels[ℓ-1].bsp`；窗口 = `[c_episode_start, confirm_src]`。
/// 委托 lib 单一来源 `nest::terminal_bits_in_book`，同口径平移并标注（S1a 图 H2）。
fn terminal_bits_old(
    classification: &classifier::Classification,
    level: usize,
    c_start: usize,
    source: usize,
    side: Side,
    b_center_start: Option<usize>,
) -> Option<BspBits> {
    let book = &classification.levels.get(event_bsp_book_level(level as u32)?)?.bsp;
    // 关③ P3 平移：旧事件 = Cand^δ 趋势族线（pan_div_diag 为 cand_delta=false 纯诊断，
    // 结构性不入终端查询）⟹ kind=Trend；B 身份 = 事件自带 `b_parent.source_interval.0`
    //（ParentCenterIdentity 已携 B start_index 快照，单一来源，无第二查法）。
    terminal_bits_in_book(
        book, c_start, source, side, NestDivergenceKind::Trend, b_center_start, TERMINAL_MATCH,
    )
    .map(|t| t.bits)
}

#[derive(Debug)]
struct LoadedBars {
    bars: Vec<Bar>,
    first_date: String,
    last_date: String,
}

fn load_bars(path: &Path, tick_size: f64) -> Result<LoadedBars, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("读取 {} 失败: {error}", path.display()))?;
    let raw: BarsJson = serde_json::from_str(&text)
        .map_err(|error| format!("{} JSON 解析失败: {error}", path.display()))?;
    let n = raw.closes.len();
    for (name, len) in [
        ("opens", raw.opens.len()),
        ("highs", raw.highs.len()),
        ("lows", raw.lows.len()),
        ("volumes", raw.volumes.len()),
        ("dates", raw.dates.len()),
    ] {
        if len != n {
            return Err(format!("列长度不一致: {name}={len}, closes={n}"));
        }
    }
    let bars = (0..n)
        .map(|index| {
            let open = raw.opens[index];
            let high = raw.highs[index];
            let low = raw.lows[index];
            let close = raw.closes[index];
            let volume = raw.volumes[index];
            Bar {
                source_index: index,
                timestamp: date_to_timestamp(&raw.dates[index]),
                open: quantize(open, tick_size),
                high: quantize(high, tick_size),
                low: quantize(low, tick_size),
                close: quantize(close, tick_size),
                volume: volume as i64,
                untradable: high < open.max(close).max(low)
                    || low > open.min(close).min(high)
                    || open <= 0.0
                    || high <= 0.0
                    || low <= 0.0
                    || close <= 0.0
                    || volume <= 0.0,
            }
        })
        .collect();
    Ok(LoadedBars {
        bars,
        first_date: raw.dates.first().cloned().unwrap_or_default(),
        last_date: raw.dates.last().cloned().unwrap_or_default(),
    })
}

#[derive(Debug, Deserialize)]
struct BarsJson {
    opens: Vec<f64>,
    highs: Vec<f64>,
    lows: Vec<f64>,
    closes: Vec<f64>,
    volumes: Vec<f64>,
    dates: Vec<String>,
}

fn date_to_timestamp(date: &str) -> Timestamp {
    let mut digits = String::with_capacity(14);
    for ch in date.chars().take_while(|value| *value != '+') {
        if ch.is_ascii_digit() {
            digits.push(ch);
            if digits.len() == 14 {
                break;
            }
        }
    }
    digits.parse().unwrap_or(0)
}
