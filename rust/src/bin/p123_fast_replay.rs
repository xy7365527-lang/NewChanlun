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
//! or_insert、pending 出清条件、term_seen 去重）逐行一致。**既有管线的唯一**差异在 prefix pass：
//! 慢版「每次 trigger 对全部 arrived (level,run) 全量重估」换为 per-(level,run) 评估缓存 +
//! 四类 dirty 判据稀疏重估（设计 §3）。判据代码零新增——dirty run 用与慢版逐字相同的 lib
//! 调用链同一输入重估（project run → decompose → assemble_level_view →
//! provide_nest_candidate_events），无新判据路径。
//! #421 另挂独立活假设 sidecar：活窗按 p409 同构路径在 `forest_epoch` 变化时独立重算，
//! 其余 bar 只延展右端；完成事件仍从同源 provider trigger 取得。账本每根 bar 先喂此刻
//! 可见活窗、再喂该 bar 首完成信号；同 bar 闪现照实记零寿命，首完成事实与终态独立。
//! 它不反流既有 YieldBook/事件流，修订只写 `P421_LIFECYCLE_DUMP`，累计诊断只写 stderr。
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
//! ── trend_confirm 游标驻留（#69 5a，090 能力声明）──
//! `LevelDerived` 持有 per-pair `ConfirmCursorStore`；dirty 评估把
//! `TowerCache::tower_confirmed_len(level - 1)` 作为唯一稳定下级水位传入 resident 核，
//! 仅 sealed prefix 的 env/acc_hi/area/dif/hist 极值跨评估驻留。run 消失或结构代次变化均
//! 失效；forced shadow 明确传 `None`，始终走冷核。冷核与 resident 核共用同一扫描函数。
//! `assemble_level_view_resident` 一次生成 pair confirmation sidecar，move completion 与
//! provider 同读该 sidecar，不再重复调用 trend_confirm。
//!
//! ── pan run memo（#69 5b，090 能力声明）──
//! `RunEntry` 按 run 持有 `PanMemo`；dirty 评估显式传入，forced shadow 显式传 `None`。
//! 写入/复用只在 source 水位严格封口与目标 block 已有两个后继块两链合取时成立；
//! segment/center/block/run 身份回缩或重折逐项失效，MACD 映射未到齐不写负缓存，动态字段
//! 每次重物化。R1 已接受的 TURN 末窗多子中枢残余不在本实现加第三门，仍由 V0、shadow 与
//! 双跑 diff 拦截。
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
//! P123_SPARSE_SUMMARY；长期回归开关，不进验收面）。#421 自有可选侧信道：
//! `P421_LIFECYCLE_DUMP=<path>`（活假设 feed/revision/异常审计；只写不判）。
//! #553（N1-T4）自有可选侧信道：`P123_EVENT_DUMP=<path>`（候选事件流逐条：key/级别/状态/
//! 钟/区间/修订号；行口径见 [`EventDump`] 文档与
//! `chanlun/review-results/issue553-t4-acceptance-20260728.md`）。**独立 sink 独立文件**——
//! 既有 `P116_DUMP` 行的产生条件、字段与行序零扰动，既有封印面不含本路任何字节；
//! 只写不判（无第二读点），env 未设时零行为差异。本路不进 §5 验收面。
//! #641（N3）自有可选侧信道：`P123_CHAIN_DUMP=<path>`（级别链证书 Delta 逐条：路径/三态/边
//! 形态/留痕/钟/修订号；行口径见 [`ChainDump`] 文档）+ `P123_CHAIN_DUMP_EVERY=<K>`（推进节拍，
//! 未设 ⟹ 只在 pass 末根推进一次）。同为**独立 sink 独立文件**，N1-T4 口径不变：只写不判、
//! env 未设零行为差异、不进 §5 验收面。

use newchan_rust::theta_v0::classifier;
use newchan_rust::theta_v0::classifier::bsp::BspPoint;
use newchan_rust::theta_v0::classifier::center::UnitRange;
use newchan_rust::theta_v0::classifier::decompose;
use newchan_rust::theta_v0::classifier::level_view::{
    assemble_level_view, assemble_level_view_resident, lower_legs_from,
    project_extended_windows_carried_only, provide_nest_candidate_events,
    provide_nest_candidate_events_resident, C2LevelViewConfig, C2VersionTuple, ConfirmCursorStore,
    ConfirmResidence, CoordinateWindow, LevelViewMaterial, LevelViewQuery, LowerLeg,
    NestCandidateEvent, NestDivergenceKind, PanMemo, PanResidence, ProjectionError,
    ProjectionMaterial,
};
use newchan_rust::theta_v0::classifier::nest::{
    assemble_certificates_snapshot, assemble_typed_certificates, event_bsp_book_level,
    terminal_bits_at_event, terminal_bits_in_book, NestIntervalCaliber, TerminalMatch,
    TypedNestCertificate,
};
use newchan_rust::theta_v0::classifier::nest_lifecycle::{
    active_l1_window_frontier, active_l2_window_frontier, active_segment_frontier,
    active_window_right_edge, feed_replay_bar, provide_active_pan_live_windows,
    ActiveSegmentFrontier, ActiveWindowFrontier, ForceMaterial, LifecycleRevision,
    LifecycleSettlementStats, NestLifecycleBook, PanCompletionEvent, PanLiveWindow,
    PanProviderPhase, ReplayBarFeed, ReplayFeedStats,
};
use newchan_rust::theta_v0::classifier::recursive_tower::{
    find_move_by_end_index, LeveledMove, WindowScanCursor,
};
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::{ParseLayer, ParseLayerIncr};
use newchan_rust::theta_v0::types::{
    quantize, Bar, BspBits, Center, Direction, MoveKind, Segment, Side, Timestamp,
};
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
            let _ = writer
                .write_fmt(args)
                .and_then(|()| writer.write_all(b"\n"));
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
                let key = (
                    block.start_center,
                    block.end_center,
                    move_kind_tag(block.kind),
                );
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

/// #553（N1-T4）候选事件 dump 侧信道的 env 开关名（唯一字面量来源）。
const EVENT_DUMP_ENV: &str = "P123_EVENT_DUMP";

/// #553（N1-T4）候选事件 dump 侧信道：`P123_EVENT_DUMP=<path>` 时把**候选事件流**逐条落盘。
///
/// 只写不判（090 能力声明，三条构造性保证）：
/// 1. **既有封印零扰动**——本路持有自己的 writer 与自己的文件，与 `P116_DUMP`
///    （CERT/DIV/TERM/TURN/TURN_CLASS/FALLBACK 全套含行序）分属两个 sink；既有行的
///    产生条件、字段与落盘顺序不被本路读写触碰，既有验收面不含本路任何字节。
/// 2. **零判定消费 / 零生产路径读取**——`revisions`/`seq` 只在 [`EventDump::observe`]
///    内自增自读，无第二个读点；账本（candidates/divergences/term_seen）、dirty 判据、
///    pending 出清、证书装配、stdout 门行都不读本结构。本结构也不回写事件。
/// 3. **env 未设 ⟹ 零行为差异**——`writer=None` 时 `observe` 首行即返回，不记 revision、
///    不推 seq、不格式化，成本 = 一次 `Option::is_none`。
///
/// **边界（写失败的传播口径，照实声明）**：`writer` 为 `Some` 时，[`EventDump::observe`]
/// 的写失败经调用点的 `?` 上抛（调用点在 prefix replay 的事件应用循环
/// [`apply_targeted_events`]，逐层经 [`evaluate_and_apply_targeted_trigger`]、
/// [`process_targeted_bar`] 的 `?` 冒到 [`run_targeted_prefix_pass`]），会**中止 prefix
/// pass**；此时 [`finalize_targeted_pass`] 内既有的 `P421_LIFECYCLE_DUMP` sidecar 收尾
/// flush（先执行）与随后的 [`EventDump::flush`] 一并被跳过（`run_targeted_prefix_pass`
/// 提前返回 ⟹ 收尾函数整体不可达）。**上抛不止于此**：该 `Err` 继续经
/// `main` 内 [`run_targeted_prefix_pass`] 调用点的 `?` 逃出 `main`，`main` 尾部的
/// [`dump_flush`] 调用**同样不可达**；而 [`DUMP`] 是 `static OnceLock<..BufWriter..>`，
/// Rust 的 `static` **不执行 `Drop`** ⟹ 既有 `P116_DUMP` 的缓冲尾字节**静默丢失**
///（受损面 = 既有封印文件；本结构自己的 `BufWriter` 由局部的 [`TargetedPassState`]
/// 持有，提前返回时随之 drop、drop 时会冲刷）。
/// 这与 `#421` 的 `P421_LIFECYCLE_DUMP` 侧信道写失败同形（同样经 [`write_lifecycle_line`]
/// 以 `?` 上抛，同样使 [`dump_flush`] 不可达）——**既有形状，非本路新引入**；与既有
/// [`dump_line`]（`P116_DUMP`）的**吞错**口径不同款——`dump_line` 写失败被 `let _ = ...`
/// 吸收，不上抛、不中止重放。关灯路径（env 未设 ⟹ `writer=None`）不受本边界影响：
/// `observe` 首行即返回 `Ok(())`，不存在写失败面，第 3 条「env 未设 ⟹ 零行为差异」的
/// 声明不因本边界而弱化。
///
/// 行口径（一行一条候选事件观察，字段序固定；见验收报告 §dump 口径）：
/// ```text
/// EVENT seq=<全局行序,0基> rev=<同 EventKey 第几次观察,1基> as_of=<观察 bar>
///       level=<级别> side=<Long|Short> kind=<trend|pan> div=<0|1 背驰确认位>
///       pending=<0|1 观察瞬间是否仍在 pending 集合> turn_source=<拐点源坐标>
///       judge_at=<provider 判定钟> seg_a=<s>..<e> interval_b=<s>..<e>
///       interval_a=<s>..<e> provider_window=<s>..<e> b_center_start=<B 中枢身份快照>
///       intake_fallback=<0|1>
/// ```
/// 「状态」= `div`+`pending`；「钟」= `as_of`/`judge_at`/`turn_source`；
/// 「区间」= `seg_a`/`interval_b`/`interval_a`/`provider_window`；「修订号」= `rev`
///（同一 EventKey 被候选事件流重复观察的次数，`div` 与钟不进 key ⟹ 同 key 的状态迁移
/// 在 rev 递增的同一序列里可读出）。
struct EventDump {
    writer: Option<Box<dyn Write>>,
    /// dump 专用修订计数；无第二读点（见上「零判定消费」）。
    revisions: BTreeMap<EventKey, u64>,
    seq: u64,
}

impl EventDump {
    fn new(writer: Option<Box<dyn Write>>) -> Self {
        Self {
            writer,
            revisions: BTreeMap::new(),
            seq: 0,
        }
    }

    /// `P123_EVENT_DUMP=<path>` 门控构造；未设 env ⟹ 关灯（零行为差异）。
    fn from_env() -> Result<Self, String> {
        let writer = std::env::var(EVENT_DUMP_ENV)
            .ok()
            .map(|path| {
                File::create(&path)
                    .map(|file| Box::new(BufWriter::new(file)) as Box<dyn Write>)
                    .map_err(|error| format!("创建 {EVENT_DUMP_ENV}={path} 失败: {error}"))
            })
            .transpose()?;
        Ok(Self::new(writer))
    }

    fn observe(
        &mut self,
        event: &NestCandidateEvent,
        as_of: usize,
        pending_hit: bool,
    ) -> Result<(), String> {
        let Some(writer) = self.writer.as_mut() else {
            return Ok(());
        };
        let revision = self.revisions.entry(EventKey::from(event)).or_insert(0);
        *revision += 1;
        let line = format!(
            "EVENT seq={} rev={} as_of={} level={} side={:?} kind={} div={} pending={} \
turn_source={} judge_at={} seg_a={}..{} interval_b={}..{} interval_a={}..{} \
provider_window={}..{} b_center_start={} intake_fallback={}",
            self.seq,
            revision,
            as_of,
            event.level,
            event.side,
            nest_kind_tag(event.kind),
            u8::from(event.divergence_confirmed),
            u8::from(pending_hit),
            event.turn_source,
            event.judge_at,
            event.seg_a.0,
            event.seg_a.1,
            event.interval_b.0,
            event.interval_b.1,
            event.interval_a.0,
            event.interval_a.1,
            event.provider_window.0,
            event.provider_window.1,
            event.b_center_start,
            u8::from(event.intake_fallback),
        );
        self.seq += 1;
        writeln!(writer, "{line}").map_err(|error| format!("写 {EVENT_DUMP_ENV} 失败: {error}"))
    }

    fn flush(&mut self) -> Result<(), String> {
        match self.writer.as_mut() {
            Some(writer) => writer
                .flush()
                .map_err(|error| format!("刷新 {EVENT_DUMP_ENV} 失败: {error}")),
            None => Ok(()),
        }
    }
}

/// #641（N3）级别链证书 dump 侧信道的 env 开关名（唯一字面量来源）。
const CHAIN_DUMP_ENV: &str = "P123_CHAIN_DUMP";
/// #641（N3）链簿推进节拍的 env 名（唯一字面量来源）。
const CHAIN_DUMP_EVERY_ENV: &str = "P123_CHAIN_DUMP_EVERY";

/// #641（N3）级别链证书 dump 侧信道：`P123_CHAIN_DUMP=<path>` 时把**链簿 Delta** 逐条落盘。
///
/// 只写不判（N1-T4 口径逐条对齐 [`EventDump`]）：
/// 1. **既有封印零扰动**——自己的 writer、自己的文件，与 `P116_DUMP`、`P421_LIFECYCLE_DUMP`、
///    `P123_EVENT_DUMP` 分属四个 sink；既有行的产生条件、字段与落盘顺序不被本路读写触碰。
/// 2. **零判定消费 / 零生产路径读取**——`book`/`seq` 只在 [`ChainDump::observe`] 内自增自读，
///    无第二个读点；账本、dirty 判据、pending 出清、证书装配、stdout 门行都不读本结构。
///    本结构对分类器**只读**（经 `TowerCache::candidate_streams()` 取候选事件流快照）。
/// 3. **env 未设 ⟹ 零行为差异**——`writer=None` 时 `observe` 首行即返回，不建簿、不推进、
///    不格式化，成本 = 一次 `Option::is_none`。
///
/// **推进节拍（显式声明，不是静默采样）**：链的覆盖边计算是 O(存活候选²)，逐 bar 推进在十万级
/// 窗口上不可行。`P123_CHAIN_DUMP_EVERY=<K>` 给节拍，未设 ⟹ 只在 pass 末根推进一次；实际取值
/// 随首行 `CHAIN_META` 印出，读 dump 的人不必猜。
///
/// **边界（写失败的传播口径）**：与 [`EventDump`] 同款——写失败经调用点 `?` 上抛，中止 prefix
/// pass，收尾 flush 不可达。既有形状，非本路新引入。
///
/// 行口径（一行一条链证书 revision；`CHAIN_META` 一行在最前）：
/// ```text
/// CHAIN_META every=<推进节拍> bars=<本 pass 总 bar 数>
/// CHAIN seq=<全局行序,0基> as_of=<推进 bar> rev=<修订号,0基> status=<Open|Closed|Invalidated>
///       root_level=<链头级别> leaf_level=<链尾级别> nodes=<路径节点数>
///       alive=<存活> falsified=<证伪留痕> absent=<查无留痕>
///       edges=<存活端点间边数> adjacent=<相邻边> skip=<跨级边> fact=<事实边(谓词判不过)>
///       crossed=<被边跨过的节点数> extendable=<0|1> extends=<0|1 是否为路径扩展>
///       observed_at=<入簿钟> closed_at=<n|-> invalidated_at=<n|->
///       path=<lvl:c_start;lvl:c_start;...>
/// ```
struct ChainDump {
    writer: Option<Box<dyn Write>>,
    book: classifier::chain_cert::ChainCertificateBook,
    every: usize,
    seq: u64,
    /// 节拍探针（#691 LOW-1 实修）：`self.book.advance(...)` **真正调用之后**记下 `as_of`
    /// ——见证的是「推进执行」，不是「越过节拍门」（此前记录点在 `advance` 调用之前，探针名不副实，
    /// 见 #676 LOW-1）。
    ///
    /// 只在 `cfg(test)` 编译进结构（同 `chain_cert::chain_probe` 的先例）——生产面零字段、零成本。
    /// 存在理由：节拍门的可观察后果（Delta 行）在空事件流上恒为零行，「节拍正确 / 永不推进 /
    /// 每根都推进」三种实现在行数与 `seq` 上**完全同形**，不加探针则测试名承诺的节拍语义
    /// 无一被夹住（#676 票面 LOW-2 = Standards MED-2）。
    #[cfg(test)]
    advanced_at: Vec<usize>,
}

impl ChainDump {
    fn new(writer: Option<Box<dyn Write>>, every: usize) -> Self {
        Self {
            writer,
            book: classifier::chain_cert::ChainCertificateBook::default(),
            every,
            seq: 0,
            #[cfg(test)]
            advanced_at: Vec::new(),
        }
    }

    /// `P123_CHAIN_DUMP=<path>` 门控构造；未设 env ⟹ 关灯（零行为差异）。
    ///
    /// 节拍解析失败是输入损坏，不默认化——返回 `Err` 并带上原始串（与电池 bin 的时间戳解析
    /// 同款纪律：绝不用哨兵值冒充有效配置）。
    fn from_env(total_bars: usize) -> Result<Self, String> {
        let writer = std::env::var(CHAIN_DUMP_ENV)
            .ok()
            .map(|path| {
                File::create(&path)
                    .map(|file| Box::new(BufWriter::new(file)) as Box<dyn Write>)
                    .map_err(|error| format!("创建 {CHAIN_DUMP_ENV}={path} 失败: {error}"))
            })
            .transpose()?;
        let every = match std::env::var(CHAIN_DUMP_EVERY_ENV) {
            Ok(raw) => raw
                .parse::<usize>()
                .map_err(|error| format!("{CHAIN_DUMP_EVERY_ENV}={raw:?} 非法: {error}"))?
                .max(1),
            Err(_) => total_bars.max(1),
        };
        let mut dump = Self::new(writer, every);
        dump.write_meta(total_bars)?;
        Ok(dump)
    }

    fn write_meta(&mut self, total_bars: usize) -> Result<(), String> {
        let every = self.every;
        let Some(writer) = self.writer.as_mut() else {
            return Ok(());
        };
        writeln!(writer, "CHAIN_META every={every} bars={total_bars}")
            .map_err(|error| format!("写 {CHAIN_DUMP_ENV} 失败: {error}"))
    }

    /// 按节拍推进链簿并落盘本次 Delta；`is_last` 为真时无条件推进（末根必推）。
    fn observe(
        &mut self,
        cache: &classifier::TowerCache,
        as_of: usize,
        is_last: bool,
    ) -> Result<(), String> {
        if self.writer.is_none() {
            return Ok(());
        }
        if !is_last && (as_of + 1) % self.every != 0 {
            return Ok(());
        }
        let delta = self.book.advance(&cache.candidate_streams(), as_of);
        #[cfg(test)]
        self.advanced_at.push(as_of);
        let lines: Vec<String> = delta
            .iter()
            .map(|certificate| {
                let line = chain_dump_line(certificate, self.seq, as_of);
                self.seq += 1;
                line
            })
            .collect();
        let writer = self.writer.as_mut().expect("已在函数首行确认为 Some");
        for line in lines {
            writeln!(writer, "{line}")
                .map_err(|error| format!("写 {CHAIN_DUMP_ENV} 失败: {error}"))?;
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<(), String> {
        match self.writer.as_mut() {
            Some(writer) => writer
                .flush()
                .map_err(|error| format!("刷新 {CHAIN_DUMP_ENV} 失败: {error}")),
            None => Ok(()),
        }
    }
}

fn chain_dump_line(
    certificate: &classifier::chain_cert::TowerChainCertificate,
    seq: u64,
    as_of: usize,
) -> String {
    use classifier::chain_cert::{ChainEdgeKind, ChainNodeStatus, ChainStatus};
    let status = match certificate.status {
        ChainStatus::Open => "Open",
        ChainStatus::Closed => "Closed",
        ChainStatus::Invalidated => "Invalidated",
    };
    let count = |wanted: ChainNodeStatus| {
        certificate
            .nodes
            .iter()
            .filter(|node| node.status == wanted)
            .count()
    };
    let adjacent = certificate
        .edges
        .iter()
        .filter(|edge| edge.kind == ChainEdgeKind::Adjacent)
        .count();
    let crossed: usize = certificate
        .edges
        .iter()
        .map(|edge| edge.crossed_nodes.len())
        .sum();
    let path: Vec<String> = certificate
        .key
        .path
        .iter()
        .map(|node| format!("{}:{}", node.level, node.c_start))
        .collect();
    let clock = |value: Option<usize>| value.map_or_else(|| "-".to_string(), |v| v.to_string());
    format!(
        "CHAIN seq={seq} as_of={as_of} rev={} status={status} root_level={} leaf_level={} \
nodes={} alive={} falsified={} absent={} edges={} adjacent={adjacent} skip={} fact={} \
crossed={crossed} extendable={} extends={} observed_at={} closed_at={} invalidated_at={} path={}",
        certificate.revision,
        certificate.root_level,
        certificate.leaf_level,
        certificate.nodes.len(),
        count(ChainNodeStatus::Alive),
        count(ChainNodeStatus::Falsified),
        count(ChainNodeStatus::Absent),
        certificate.edges.len(),
        certificate.edges.len() - adjacent,
        certificate.fact_edge_count(),
        u8::from(certificate.extendable),
        u8::from(certificate.extends.is_some()),
        certificate.observed_at,
        clock(certificate.closed_at),
        clock(certificate.invalidated_at),
        path.join(";"),
    )
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
    /// #69 5a：本级各 run 的 trend-confirm per-pair resident 游标。
    confirm_cursors: ConfirmCursorStore,
}

/// per-(level, run_source_start) 评估缓存条目（设计 §3 的 per-(level,run) 评估缓存）。
///
/// #421 sidecar 的完成事件与 targeted 路径共用这一份 provider 产物；活窗由独立逐 bar
/// 路径从同一 tower 重算。未来 5a/5b 的 ConfirmCursor/PanMemo 仍分别驻留
/// `LevelDerived`/`RunEntry`，不得另建平行 provider 缓存。
#[derive(Debug)]
struct RunEntry {
    /// 共享 provider 产物评估时的 LevelDerived.self_gen / lower_gen。
    self_gen: u64,
    lower_gen: u64,
    /// 共享 provider 产物评估时的 as_of（dirty 判据 (iii) 水位基线）。
    last_as_of: usize,
    /// provider 评估的完整 run 投影中心/块类别；逃生门不再把它们当活窗时钟。
    #[allow(dead_code)]
    centers: Vec<Center>,
    #[allow(dead_code)]
    kinds: Vec<Option<MoveKind>>,
    /// provider 的完整输出（provide 原序）；targeted 应用时才与当前 pending 求交。
    events: Vec<NestCandidateEvent>,
    /// targeted 消费者自己的旧缓存水位，仅用于保持 p123 原 dirty/物理 views 口径；
    /// payload 仍只有上面一份，sidecar 不持有第二套结果。
    target_self_gen: Option<u64>,
    target_lower_gen: Option<u64>,
    target_last_as_of: Option<usize>,
    /// #69 5b：严格 run-local；dirty 更新不得替换，forced shadow 不得借用。
    pan_memo: PanMemo,
}

impl RunEntry {
    fn new(
        self_gen: u64,
        lower_gen: u64,
        last_as_of: usize,
        centers: Vec<Center>,
        kinds: Vec<Option<MoveKind>>,
        events: Vec<NestCandidateEvent>,
    ) -> Self {
        Self {
            self_gen,
            lower_gen,
            last_as_of,
            centers,
            kinds,
            events,
            target_self_gen: None,
            target_lower_gen: None,
            target_last_as_of: None,
            pan_memo: PanMemo::default(),
        }
    }

    fn update(
        &mut self,
        self_gen: u64,
        lower_gen: u64,
        last_as_of: usize,
        centers: Vec<Center>,
        kinds: Vec<Option<MoveKind>>,
        events: Vec<NestCandidateEvent>,
    ) {
        self.self_gen = self_gen;
        self.lower_gen = lower_gen;
        self.last_as_of = last_as_of;
        self.centers = centers;
        self.kinds = kinds;
        self.events = events;
    }
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
    /// #69 5b run-local pan memo 的终态驻留/累计诊断；只进 stderr。
    pan_entries: usize,
    pan_hits: usize,
    pan_misses: usize,
    pan_writes: usize,
    pan_invalidations: usize,
    per_level_reevals: BTreeMap<usize, usize>,
    per_level_views_slow_would: BTreeMap<usize, usize>,
}

/// #421 生产侧车的累计审计读面；只进 stderr/独立 dump，不参与 p123 既有账本与判定。
#[derive(Debug, Default)]
struct LifecycleReplayStats {
    bars: usize,
    provider_triggers: usize,
    live_windows: usize,
    completion_events: usize,
    completion_signals: usize,
    channel_switches: usize,
    extension_suppressed: usize,
    retrograde_rejected: usize,
    completion_force_unavailable: usize,
    /// sidecar 对共享 RunEntry 的请求 / 实际补算 / 直接复用。
    provider_requests: usize,
    provider_reevals: usize,
    provider_reuses: usize,
    /// #527/#601：活窗定位结果分布，**按级别分列**（key = `(level, reason_tag)`；
    /// `level=0` = 级别无关的前置失败）。「某完成身份为何没有更早 Live」的可审计落点；
    /// L1/L2 共用同一码表命名空间但**禁合并计数**（合记则两级读数互相污染，无法逐级归因）。
    /// 诊断只写不判。
    l1_live_outcomes: BTreeMap<(u32, &'static str), usize>,
    /// #559 C2 段账本完整性观测：活窗产出用的段序列（`tower[0]` 投影）与 parser 段账本
    /// `l0.segments` 是否等长。**短一个即真缺段**——λ_C 的定界窗口会跨过缺失段。
    /// 「行进中段起点 − 末段终点 > 0」**不是**缺段判据（笔构造 `gap_ok` 不足时 `i += 2`
    /// 跳过分型 ⟹ 相邻笔在源坐标上本就可不相接），故另立本计数直接查缺段。
    seg_ledger_complete: usize,
    seg_ledger_short: usize,
    /// 塔尚未构造（`tower` 为空，bootstrap 前几十 bar）——不参与完整性判定。
    seg_ledger_no_tower: usize,
    /// 末 prefix 的终局分布与寿命读面（闪现/非闪现分层）。
    settlement: LifecycleSettlementStats,
}

/// #559 C1：L1 活窗定位的**逐 run** 诊断行——命中与未命中都记。
///
/// 原实装只记未命中行，于是「该 run 定位成功、但定位到的是**别的** C」这一现场在 dump 上
/// 无痕 ⟹ 该身份的归因落到「无任何原因码行」（#559 条件 C1 点名的 11 只）。补记命中行后，
/// 逐身份归因可在同一锚（`b_center_start`）上区分「未能定位」与「定位到别的 C」。
/// 诊断只写不判，不进任何真值路径。
#[derive(Debug, Clone, Copy)]
struct L1LiveDiagRow {
    /// 产该行的级别（票 #601：L1/L2 共用同一诊断面 ⟹ 必须分级，否则两级读数混记）。
    /// `0` = 级别无关的前置失败（无行进中 L0 段 ⟹ 两级都没有活动 C 腿）。
    level: u32,
    /// `PanLiveOutcome::reason_tag()`（命中为 `"window"`）／
    /// `ActiveWindowOutcome::reason_tag()`（L2 的 C 腿派生失败码）。
    reason: &'static str,
    /// 该 run 的锚中枢起点：命中取产窗身份的 B；未命中取 frontier 之前最近中枢。
    b_center_start: Option<usize>,
    /// 命中时产出窗的 λ_C（未命中恒 None）——「同锚多候选 C」的判据。
    c_start: Option<usize>,
    /// 命中时产出窗的 `gap_len`（票 #592；未命中恒 None）。
    gap_len: Option<usize>,
    /// 命中时产出窗的 `seg_a`（票 #618：`(b_center_start, c_start)` 粗键在同一 C 位置上
    /// 先后出现多个不同身份（不同 `seg_a` 解释）时无法区分——#618 §2.1 实证 `c=17170`
    /// 一例。未命中时结构定位本身未成功（`locate_pan_div_structure`/`_front_anchor` 均未
    /// 产出，见 `provide_active_pan_live_windows`），seg_a 无值可记，恒 None——不是遗漏，
    /// 是该原因码下 seg_a 概念不适用。
    seg_a: Option<(usize, usize)>,
}

/// #527/#601/#613：**L1/L2** 活窗结构分量的重算判据键——四项逐值相等 ⟹ 定位输出不变 ⟹
/// 跳过重算是等价优化而非行为改动（逐项来源与「为什么不能少一项」见回放循环内的判据文档）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LifecycleLowerKey {
    forest_epoch: u64,
    frontier: Option<ActiveSegmentFrontier>,
    l1_scan: Option<WindowScanCursor>,
    l1_confirmed_len: usize,
}

/// #602：**L3** 活窗结构分量在 [`LifecycleLowerKey`] 之外**多出**的两项塔侧输入。
///
/// 两项与 L1 层的同名项同理不可由 `forest_epoch` 覆盖：`level_scan_cursor(2)` 可在塔字节
/// 不变时前进（不成立支 `i += 1` 推进 `resume_from` 而无窗口产出）；`tower_confirmed_len(2)`
/// 是**有状态量**（cascade bar 压低的水线跨 bar 保留，恢复那一步不经过 epoch/cursor，#613
/// 实测 4 例）。L1/L2 **不消费**这两项 ⟹ 它们只驱动 L3 的重算（局部依赖，见
/// `recompute_lifecycle_window_stems` 文档）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LifecycleUpperKey {
    l2_scan: Option<WindowScanCursor>,
    l2_confirmed_len: usize,
}

/// #602：L2 那一段派生出、**L3 复用**的中间量（递归复合：L3 的虚拟单元 = L2 的活动 C 腿）。
///
/// 输入全在 [`LifecycleLowerKey`] 内 ⟹ lower 段不重算时其值不会陈旧，L3 单独重算可直接取。
/// L2 那一段每次重算**先清空再回填**，任何失败路径都不给 L3 留旧值。
#[derive(Debug, Clone, Default)]
struct LowerFrontierCarry {
    /// 行进中的 L1 窗口单元（[`active_l1_window_frontier`] 产物）。
    l1_frontier: Option<ActiveWindowFrontier>,
    /// `lower_legs_from(tower[1])` 的首叶方向列（与 `level_scan_units(2)` 逐位对应）。
    l1_leg_dirs: Vec<Direction>,
}

/// #421 逃生门的活窗结构分量；与 p409 `WindowStem` 同键，右端不进身份。`gap_len`
/// 是产窗时刻的诊断快照（票 #592），随身份一起冻结——只延展右端时原样带出，不重算。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct LifecycleWindowStem {
    level: u32,
    side_tag: u8,
    seg_a: (usize, usize),
    c_start: usize,
    b_center_start: usize,
    gap_len: usize,
}

impl LifecycleWindowStem {
    fn of(window: &PanLiveWindow) -> Self {
        Self {
            level: window.level,
            side_tag: match window.side {
                Side::Long => 0,
                Side::Short => 1,
            },
            seg_a: window.seg_a,
            c_start: window.seg_c_live.0,
            b_center_start: window.b_center_start,
            gap_len: window.gap_len,
        }
    }

    fn window_at(self, as_of: usize) -> PanLiveWindow {
        PanLiveWindow {
            level: self.level,
            side: if self.side_tag == 0 {
                Side::Long
            } else {
                Side::Short
            },
            seg_a: self.seg_a,
            seg_c_live: (self.c_start, active_window_right_edge(self.c_start, as_of)),
            b_center_start: self.b_center_start,
            gap_len: self.gap_len,
        }
    }
}

impl LifecycleReplayStats {
    fn observe(&mut self, feed: ReplayFeedStats) {
        self.bars += 1;
        self.live_windows += feed.live_windows;
        self.completion_events += feed.completion_events;
        self.completion_signals += feed.completion_signals;
        self.channel_switches += feed.channel_switches;
        self.extension_suppressed += feed.extension_suppressed;
        self.retrograde_rejected += feed.retrograde_rejected;
        self.completion_force_unavailable += feed.completion_force_unavailable;
    }
}

fn main() -> Result<(), String> {
    let (config, loaded, max_bars) = parse_cli_and_load()?;
    print_startup_banners(&loaded, max_bars);

    let mut audit = ProviderAudit::default();
    let terminal = run_terminal_pass(&loaded.bars[..max_bars], &config)?;
    let terminal_events = collect_terminal_snapshot(&terminal, max_bars, &mut audit)?;
    let targets = build_target_map(&terminal_events);

    let (mut book, unresolved_targets) =
        run_prefix_pass_and_report(&loaded.bars[..max_bars], &config, &targets, &mut audit)?;

    let TerminalState {
        l0,
        classification,
        tower,
        cache,
    } = terminal;
    let final_events = finalize_judge_at(terminal_events, &book, max_bars);
    observe_snapshot(
        &classification,
        final_events,
        max_bars - 1,
        &mut book,
        &mut audit,
    );

    let (full_classification, full_tower) = classifier::classify_with_tower(&l0, &config);
    let bit_diff = compute_bit_diff(&full_classification, &classification, &full_tower, &tower);
    print_bit_diff_levels(
        &full_classification,
        &classification,
        bit_diff.classification_diff,
    );

    let old = compute_old_semantic_counts(&l0, &classification, &tower, &config, &cache);

    print_yield_cert_d3(&book);
    print_baseline_and_provider(&book, &old, &audit, unresolved_targets);
    print_bit_exact_and_r7(&bit_diff, &book, &audit, unresolved_targets);
    report_missed_reconciliation(&book);

    dump_flush();
    Ok(())
}

fn parse_cli_and_load() -> Result<(ThetaConfig, LoadedBars, usize), String> {
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
    Ok((config, loaded, max_bars))
}

fn print_startup_banners(loaded: &LoadedBars, max_bars: usize) {
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
}

fn collect_terminal_snapshot(
    terminal: &TerminalState,
    max_bars: usize,
    audit: &mut ProviderAudit,
) -> Result<Vec<Vec<NestCandidateEvent>>, String> {
    let (hist, close_src) = terminal.cache.causal_series();
    let dif = terminal.cache.macd_dif();
    collect_snapshot_candidates(&terminal.tower, max_bars - 1, hist, dif, close_src, audit)
}

fn build_target_map(
    terminal_events: &[Vec<NestCandidateEvent>],
) -> BTreeMap<EventKey, NestCandidateEvent> {
    let mut targets = BTreeMap::new();
    for event in terminal_events.iter().flatten() {
        targets.insert(EventKey::from(event), *event);
    }
    targets
}

fn run_prefix_pass_and_report(
    bars: &[Bar],
    config: &ThetaConfig,
    targets: &BTreeMap<EventKey, NestCandidateEvent>,
    audit: &mut ProviderAudit,
) -> Result<(YieldBook, usize), String> {
    let prefix_started = Instant::now();
    let (book, prefix_views, unresolved_targets, stats, lifecycle_stats) =
        run_targeted_prefix_pass(bars, config, targets)?;
    let prefix_elapsed = prefix_started.elapsed();
    audit.views += prefix_views;
    audit.snapshots += book.candidates.len();
    print_sparse_summary(&stats, prefix_elapsed);
    print_lifecycle_summary(&lifecycle_stats);
    print_lifetime_summary(&lifecycle_stats.settlement);
    Ok((book, unresolved_targets))
}

fn print_sparse_summary(stats: &SparseStats, prefix_elapsed: std::time::Duration) {
    eprintln!(
        "P123_SPARSE_SUMMARY triggers={} reevals={} reuses={} syncs_self={} syncs_lower={} wm_cross_with_lower={} wm_cross_without_lower={} term_rechecks={} term_skips={} shadow_checks={} shadow_mismatches={} shadow_term_mismatches={} pan_entries={} pan_hits={} pan_misses={} pan_writes={} pan_invalidations={} prefix_s={:.3}",
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
        stats.pan_entries,
        stats.pan_hits,
        stats.pan_misses,
        stats.pan_writes,
        stats.pan_invalidations,
        prefix_elapsed.as_secs_f64(),
    );
    for (level, reevals) in &stats.per_level_reevals {
        let slow_would = stats
            .per_level_views_slow_would
            .get(level)
            .copied()
            .unwrap_or(0);
        eprintln!(
            "P123_SPARSE_LEVEL level={level} reevals={reevals} slow_would_views={slow_would}"
        );
    }
}

fn print_lifecycle_summary(lifecycle_stats: &LifecycleReplayStats) {
    let completion_force_unavailable_rate = if lifecycle_stats.completion_signals == 0 {
        0.0
    } else {
        lifecycle_stats.completion_force_unavailable as f64
            / lifecycle_stats.completion_signals as f64
    };
    eprintln!(
        "P421_LIFECYCLE_SUMMARY bars={} provider_triggers={} live_windows={} completion_events={} completion_signals={} channel_switches={} extension_suppressed={} retrograde_rejected={} completion_force_unavailable={} completion_force_unavailable_rate={:.9} provider_requests={} provider_reevals={} provider_reuses={}",
        lifecycle_stats.bars,
        lifecycle_stats.provider_triggers,
        lifecycle_stats.live_windows,
        lifecycle_stats.completion_events,
        lifecycle_stats.completion_signals,
        lifecycle_stats.channel_switches,
        lifecycle_stats.extension_suppressed,
        lifecycle_stats.retrograde_rejected,
        lifecycle_stats.completion_force_unavailable,
        completion_force_unavailable_rate,
        lifecycle_stats.provider_requests,
        lifecycle_stats.provider_reevals,
        lifecycle_stats.provider_reuses,
    );
    eprintln!(
        "P559_SEG_LEDGER complete={} short={} no_tower={}",
        lifecycle_stats.seg_ledger_complete,
        lifecycle_stats.seg_ledger_short,
        lifecycle_stats.seg_ledger_no_tower,
    );
    // #601：按级别分列（`l{level}:{tag}=n`；`l0:` = 级别无关的前置失败）。
    eprintln!(
        "P527_L1_LIVE_OUTCOMES {}",
        lifecycle_stats
            .l1_live_outcomes
            .iter()
            .map(|((level, tag), count)| format!("l{level}:{tag}={count}"))
            .collect::<Vec<_>>()
            .join(" ")
    );
}

fn print_lifetime_summary(settlement: &LifecycleSettlementStats) {
    eprintln!(
        "P421_LIFETIME_SUMMARY entries={} first_provable={} provisional={} confirmed={} force_overtake={} force_overtake_claimed={} never_constituted={} identity_vanished_refuted={} identity_vanished_seam={} flash_terminal={} nonflash_count={} nonflash_min={:?} nonflash_median={:?} nonflash_max={:?} force_lifetime_count={} force_lifetime_min={:?} force_lifetime_median={:?} force_lifetime_max={:?}",
        settlement.entry_count,
        settlement.first_provable_count,
        settlement.provisional_count,
        settlement.confirmed_count,
        settlement.force_overtake_count,
        settlement.force_overtake_claimed_count,
        settlement.never_constituted_count,
        settlement.identity_vanished_refuted_count,
        settlement.identity_vanished_seam_count,
        settlement.flash_terminal_count,
        settlement.nonflash_lifetime.count,
        settlement.nonflash_lifetime.min,
        settlement.nonflash_lifetime.median,
        settlement.nonflash_lifetime.max,
        settlement.force_overtake_lifetime.count,
        settlement.force_overtake_lifetime.min,
        settlement.force_overtake_lifetime.median,
        settlement.force_overtake_lifetime.max,
    );
}

fn finalize_judge_at(
    mut final_events: Vec<Vec<NestCandidateEvent>>,
    book: &YieldBook,
    max_bars: usize,
) -> Vec<Vec<NestCandidateEvent>> {
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
    final_events
}

/// `main` 拆分抽取（票 #605）：全量分类/塔的字节级差异计数，供 `P123_BIT_EXACT` 与逐级
/// `P123_BIT_DIFF_LEVEL` 共用同一份计数，避免两处重算分叉。
struct BitDiffCounts {
    classification_diff: usize,
    tower_diff: usize,
    old_semantic_diff: usize,
    lifecycle_diff: usize,
}

fn compute_bit_diff(
    full_classification: &classifier::Classification,
    classification: &classifier::Classification,
    full_tower: &[Rc<Vec<LeveledMove>>],
    tower: &[Rc<Vec<LeveledMove>>],
) -> BitDiffCounts {
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
    BitDiffCounts {
        classification_diff,
        tower_diff,
        old_semantic_diff,
        lifecycle_diff,
    }
}

fn print_bit_diff_levels(
    full_classification: &classifier::Classification,
    classification: &classifier::Classification,
    classification_diff: usize,
) {
    if classification_diff == 0 {
        return;
    }
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

struct OldSemanticCounts {
    old_candidates: usize,
    old_terminal: usize,
    old_certificates: usize,
}

fn compute_old_semantic_counts(
    l0: &ParseLayer,
    classification: &classifier::Classification,
    tower: &[Rc<Vec<LeveledMove>>],
    config: &ThetaConfig,
    cache: &classifier::TowerCache,
) -> OldSemanticCounts {
    let old_events =
        classifier::cand_delta_tower_cached(l0, classification, tower, config, cache);
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
                            classification,
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
    let old_certificates = compute_old_certificates(&old_events, classification);
    OldSemanticCounts {
        old_candidates,
        old_terminal,
        old_certificates,
    }
}

fn compute_old_certificates(
    old_events: &[Vec<classifier::recursive_tower::CandDeltaEvent>],
    classification: &classifier::Classification,
) -> usize {
    let mut old_certificates = 0usize;
    for exec in 0..old_events.len() {
        for top in exec..old_events.len() {
            old_certificates += assemble_certificates_snapshot(old_events, exec, top, |event| {
                terminal_bits_old(
                    classification,
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
    old_certificates
}

fn print_yield_cert_d3(book: &YieldBook) {
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
}

fn print_baseline_and_provider(
    book: &YieldBook,
    old: &OldSemanticCounts,
    audit: &ProviderAudit,
    unresolved_targets: usize,
) {
    println!(
        "P123_BASELINE old_candidates={} old_terminal_confirmed={} old_certificates={} new_candidates={} new_terminal_confirmed={} new_B_certificates={}",
        old.old_candidates,
        old.old_terminal,
        old.old_certificates,
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
}

fn print_bit_exact_and_r7(
    bit_diff: &BitDiffCounts,
    book: &YieldBook,
    audit: &ProviderAudit,
    unresolved_targets: usize,
) {
    println!(
        "P123_BIT_EXACT old_path_diff={} tower_diff={} moves_centers_bsp_pan_diff={} lifecycle_cp_ownership_diff={} classification_total_diff={}",
        bit_diff.old_semantic_diff,
        bit_diff.tower_diff,
        bit_diff.old_semantic_diff.saturating_sub(bit_diff.tower_diff),
        bit_diff.lifecycle_diff,
        bit_diff.classification_diff
    );
    println!(
        "P123_R7 provider_complete={} definition_faithful=true snapshot_no_forward={} B_zero={}",
        audit.provider_complete() && unresolved_targets == 0,
        audit.snapshot_future_violations == 0,
        book.cert_b.is_empty()
    );
}

/// #97: 遗漏对账 —— 钟位可证的候选却从未被任何 B 链吸收，逐条枚举。
fn report_missed_reconciliation(book: &YieldBook) {
    let missed: Vec<&EventKey> = book
        .terminal_confirmed
        .iter()
        .filter(|key| {
            !book
                .covered_b
                .contains(&(key.level, key.turn_source, key.interval_b))
        })
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
/// `(forest_epoch, signal_signature)`——targeted 与 lifecycle 两条通道共用的同一枚触发键。
type TriggerKey = (u64, Vec<(usize, usize, usize, usize)>);

/// `run_targeted_prefix_pass` 拆分抽取（票 #605）：逐 bar 稀疏重估循环的全部跨 bar 状态，
/// 原样搬迁为字段——判据/口径一个比特不动，纯粹是状态载体从一串 `let mut` 改成一个结构体。
struct TargetedPassState<'c> {
    parser: ParseLayerIncr<'c>,
    cache: classifier::TowerCache,
    book: YieldBook,
    turns: TurnBook,
    pending: BTreeSet<EventKey>,
    views: usize,
    last_trigger: Option<TriggerKey>,
    last_lifecycle_trigger: Option<TriggerKey>,
    lifecycle_book: NestLifecycleBook,
    lifecycle_stats: LifecycleReplayStats,
    lifecycle_window_stems: Vec<LifecycleWindowStem>,
    // 分级持有（#602）：两段各按自己的键刷新，合并后喂账本。
    lifecycle_stems_lower: Vec<LifecycleWindowStem>,
    lifecycle_stems_upper: Vec<LifecycleWindowStem>,
    lower_frontier_carry: LowerFrontierCarry,
    last_lifecycle_lower_key: Option<LifecycleLowerKey>,
    last_lifecycle_upper_key: Option<LifecycleUpperKey>,
    lifecycle_dump: Option<BufWriter<File>>,
    // #553（N1-T4）候选事件 dump：独立 sink，与 P116_DUMP 既有封印面互不触碰（只写不判）。
    event_dump: EventDump,
    // #641（N3）链证书 dump：同款独立 sink（只写不判），与 event_dump 分属两个文件、两把
    // env 开关，互不读写；env 未设 ⟹ 零行为差异。
    chain_dump: ChainDump,
    // ── p123 稀疏状态 ──
    derived: BTreeMap<usize, LevelDerived>,
    entries: BTreeMap<(usize, usize), RunEntry>,
    // 判据 (iv)：levels[book_level].bsp 上次**已查**内容快照（BspPoint PartialEq 排除
    // force——force 不进 terminal_bits_in_book 判定，值比对对 TERM 语义充分）。
    bsp_snaps: BTreeMap<usize, Vec<BspPoint>>,
    stats: SparseStats,
}

impl<'c> TargetedPassState<'c> {
    /// `total_bars` 只被 [`ChainDump::from_env`] 用作「节拍未设 ⟹ 只在 pass 末根推进一次」的
    /// 默认节拍与 `CHAIN_META` 行的 `bars=` 字段；不进任何既有判据。
    fn new(
        config: &'c ThetaConfig,
        targets: &BTreeMap<EventKey, NestCandidateEvent>,
        total_bars: usize,
    ) -> Result<Self, String> {
        let lifecycle_dump = std::env::var("P421_LIFECYCLE_DUMP")
            .ok()
            .map(|path| {
                File::create(&path)
                    .map(BufWriter::new)
                    .map_err(|error| format!("创建 P421_LIFECYCLE_DUMP={path} 失败: {error}"))
            })
            .transpose()?;
        Ok(Self {
            parser: ParseLayerIncr::new(config),
            cache: classifier::TowerCache::new(),
            book: YieldBook::default(),
            turns: TurnBook::default(),
            pending: targets.keys().cloned().collect(),
            views: 0,
            last_trigger: None,
            last_lifecycle_trigger: None,
            lifecycle_book: NestLifecycleBook::new(),
            lifecycle_stats: LifecycleReplayStats::default(),
            lifecycle_window_stems: Vec::new(),
            lifecycle_stems_lower: Vec::new(),
            lifecycle_stems_upper: Vec::new(),
            lower_frontier_carry: LowerFrontierCarry::default(),
            last_lifecycle_lower_key: None,
            last_lifecycle_upper_key: None,
            lifecycle_dump,
            event_dump: EventDump::from_env()?,
            chain_dump: ChainDump::from_env(total_bars)?,
            derived: BTreeMap::new(),
            entries: BTreeMap::new(),
            bsp_snaps: BTreeMap::new(),
            stats: SparseStats::default(),
        })
    }
}

fn run_targeted_prefix_pass(
    bars: &[Bar],
    config: &ThetaConfig,
    targets: &BTreeMap<EventKey, NestCandidateEvent>,
) -> Result<(YieldBook, usize, usize, SparseStats, LifecycleReplayStats), String> {
    let mut state = TargetedPassState::new(config, targets, bars.len())?;
    // #103 侧信道：P116_CKPT=<K> 时每 K bars 做一次全量快照装配并 dump（只写不判）。
    let ckpt_every: usize = std::env::var("P116_CKPT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    // §6.3 shadow 强制对拍开关（长期回归开关；stderr 诊断，不进 dump/stdout 验收面）。
    let shadow = std::env::var("P123_SHADOW").ok().as_deref() == Some("1");
    let started = Instant::now();
    for (index, bar) in bars.iter().copied().enumerate() {
        process_targeted_bar(
            &mut state, bar, index, bars.len(), targets, config, shadow, ckpt_every, started,
        )?;
    }
    finalize_targeted_pass(state)
}

#[allow(clippy::too_many_arguments)]
fn process_targeted_bar(
    state: &mut TargetedPassState<'_>,
    bar: Bar,
    index: usize,
    bars_len: usize,
    targets: &BTreeMap<EventKey, NestCandidateEvent>,
    config: &ThetaConfig,
    shadow: bool,
    ckpt_every: usize,
    started: Instant,
) -> Result<(), String> {
    let l0 = state.parser.append(bar);
    let (classification, tower) =
        classifier::classify_with_tower_incremental(&l0, config, &mut state.cache);
    // #641（N3）链 dump 侧信道（只写不判；关灯时 `observe` 首行返回）。放在既有 TURN 观察
    // **之前**（与驻车线原口径同位），只对 `state.cache` 做只读取数（`candidate_streams()`），
    // 不改 classification/tower/turns 的任何输入与产生条件，也不触碰 `event_dump` 的 sink。
    state
        .chain_dump
        .observe(&state.cache, index, index + 1 == bars_len)?;
    // #116: TURN 逐 bar 观察（摊还 O(新稳定块数)，不经 trigger 门——pending 空后仍落盘）。
    state.turns.observe(&classification);
    let forest_epoch = state.cache.forest_epoch();

    let keys = compute_lifecycle_due_keys(state, &l0, forest_epoch);
    if keys.lower_due || keys.upper_due {
        recompute_stems_if_due(state, &tower, &l0, keys, index)?;
    }

    let trigger = (forest_epoch, signal_signature(&classification));
    let lifecycle_due = state.last_lifecycle_trigger.as_ref() != Some(&trigger);
    if state.last_trigger.as_ref() != Some(&trigger) && !state.pending.is_empty() {
        evaluate_and_apply_targeted_trigger(state, &tower, &classification, targets, index, shadow)?;
        state.last_trigger = Some(trigger.clone());
    }
    feed_lifecycle_and_checkpoint(
        state, &tower, &classification, index, lifecycle_due, trigger, bars_len, targets.len(),
        ckpt_every, started,
    )?;
    Ok(())
}

/// #527/#601/#602 活窗重算判据的当前 bar 读数（键 + 是否到期）。**不携带** `TowerScanView`
/// 的借用面（`units: &[UnitRange]` 借自 `state.cache`）——那会拖着对 `state` 的不可变借用跨过
/// [`recompute_stems_if_due`] 需要的可变借用；`l1_view`/`l2_view` 由该函数按需重取
/// （`TowerCache` 的四个访问器都是纯读、零副作用，重取与传参同值同效）。
#[derive(Clone, Copy)]
struct DueKeys {
    frontier: Option<ActiveSegmentFrontier>,
    lower_key: LifecycleLowerKey,
    upper_key: LifecycleUpperKey,
    lower_due: bool,
    upper_due: bool,
}

/// #527：活窗结构分量在「confirmed 侧变（forest_epoch）∨ active C frontier 变」时重算；
/// 两次重算之间逐 bar 只延展右端。#601/#613：L2 另需 L1 层扫描断点 + confirmed 侧整窗截断
/// 水线两项（均可在 `forest_epoch` 不变时前进/为有状态量）——BTC 100k 实测 4 例三元组不变
/// 而水线变化（as_of=53461/85046/94694/97241），根因是 `confirmed_watermark` 的 cascade
/// 压低值跨 bar 保留、恢复步不经过 epoch/cursor。#602：L3 的两项塔侧输入同理不可由
/// `forest_epoch` 覆盖，但**另立一把键**（[`LifecycleUpperKey`]）——并键会过度失效 L1/L2
/// （实测 `l1:window` 610→615），故两段各按自己的键重算，`lower_due ⟹ upper_due`（单向）。
fn compute_lifecycle_due_keys(
    state: &TargetedPassState<'_>,
    l0: &ParseLayer,
    forest_epoch: u64,
) -> DueKeys {
    // #527 L1 活窗：C 只能是 parser 的行进中段（active frontier），禁 confirmed 段回放重建。
    let frontier = active_segment_frontier(l0);
    let lower_key = LifecycleLowerKey {
        forest_epoch,
        frontier,
        l1_scan: state.cache.level_scan_cursor(1),
        l1_confirmed_len: state.cache.tower_confirmed_len(1),
    };
    let upper_key = LifecycleUpperKey {
        l2_scan: state.cache.level_scan_cursor(2),
        l2_confirmed_len: state.cache.tower_confirmed_len(2),
    };
    let lower_due = state.last_lifecycle_lower_key != Some(lower_key);
    let upper_due = lower_due || state.last_lifecycle_upper_key != Some(upper_key);
    DueKeys {
        frontier,
        lower_key,
        upper_key,
        lower_due,
        upper_due,
    }
}

fn recompute_stems_if_due(
    state: &mut TargetedPassState<'_>,
    tower: &[Rc<Vec<LeveledMove>>],
    l0: &ParseLayer,
    keys: DueKeys,
    index: usize,
) -> Result<(), String> {
    let before = state.lifecycle_window_stems.len();
    let diag_rows = recompute_and_merge_stems(state, tower, keys, index);

    let seg_ledger =
        record_seg_ledger_observation(&mut state.lifecycle_stats, tower, l0, keys.lower_due);
    write_pan_live_diag_rows(&mut state.lifecycle_dump, diag_rows, keys.frontier, index)?;
    // 重取用于本行落盘（与判据/重算同源同值，`TowerCache` 访问器纯读零副作用，重取不改变
    // 行为，只避免借用跨越上面 `recompute_and_merge_stems` 需要的 `&mut state`）。
    let l1_view = TowerScanView {
        units: state.cache.level_scan_units(1),
        cursor: state.cache.level_scan_cursor(1),
        confirmed_len: state.cache.tower_confirmed_len(1),
    };
    let l2_view = TowerScanView {
        units: state.cache.level_scan_units(2),
        cursor: state.cache.level_scan_cursor(2),
        confirmed_len: state.cache.tower_confirmed_len(2),
    };
    write_pan_live_recompute_line(
        &mut state.lifecycle_dump,
        index,
        keys.frontier,
        l1_view,
        l2_view,
        l0.segments.len(),
        seg_ledger.confirmed_last_end,
        seg_ledger.tower0_units,
        before,
        state.lifecycle_window_stems.len(),
    )?;
    state.last_lifecycle_lower_key = Some(keys.lower_key);
    state.last_lifecycle_upper_key = Some(keys.upper_key);
    Ok(())
}

fn merge_lifecycle_stems(state: &mut TargetedPassState<'_>) {
    // 合并两段（`LifecycleWindowStem` 的 Ord 以 level 为首字段 ⟹ 拼接后重排即全局有序；
    // 两段级别不相交 ⟹ dedup 不会跨段吞并）。
    state.lifecycle_window_stems = state
        .lifecycle_stems_lower
        .iter()
        .chain(state.lifecycle_stems_upper.iter())
        .copied()
        .collect();
    state.lifecycle_window_stems.sort();
    state.lifecycle_window_stems.dedup();
}

fn recompute_and_merge_stems(
    state: &mut TargetedPassState<'_>,
    tower: &[Rc<Vec<LeveledMove>>],
    keys: DueKeys,
    index: usize,
) -> Vec<L1LiveDiagRow> {
    // #601/#602 L2/L3 活窗的塔侧只读输入三元——只在真到期时取（与判据键同源同值）。
    let l1_view = TowerScanView {
        units: state.cache.level_scan_units(1),
        cursor: state.cache.level_scan_cursor(1),
        confirmed_len: state.cache.tower_confirmed_len(1),
    };
    let l2_view = TowerScanView {
        units: state.cache.level_scan_units(2),
        cursor: state.cache.level_scan_cursor(2),
        confirmed_len: state.cache.tower_confirmed_len(2),
    };
    let mut diag_rows: Vec<L1LiveDiagRow> = Vec::new();
    if keys.lower_due {
        state.lifecycle_stems_lower = recompute_lifecycle_window_stems(
            1..3,
            tower,
            keys.frontier.as_ref(),
            l1_view,
            l2_view,
            &mut state.lower_frontier_carry,
            index,
            &mut state.lifecycle_stats.l1_live_outcomes,
            &mut diag_rows,
        );
    }
    if keys.upper_due {
        state.lifecycle_stems_upper = recompute_lifecycle_window_stems(
            3..4,
            tower,
            keys.frontier.as_ref(),
            l1_view,
            l2_view,
            &mut state.lower_frontier_carry,
            index,
            &mut state.lifecycle_stats.l1_live_outcomes,
            &mut diag_rows,
        );
    }
    merge_lifecycle_stems(state);
    diag_rows
}

/// #559 C2 观测面读数：末 L0 单元终点 + 段账本完整性分类。
struct SegLedgerObservation {
    confirmed_last_end: usize,
    tower0_units: usize,
}

fn record_seg_ledger_observation(
    lifecycle_stats: &mut LifecycleReplayStats,
    tower: &[Rc<Vec<LeveledMove>>],
    l0: &ParseLayer,
    lower_due: bool,
) -> SegLedgerObservation {
    // #559 C2 观测面：末 L0 单元终点（衔接差 = frontier.start − 该值）与段账本完整性。
    let confirmed_last_end = tower
        .first()
        .and_then(|level0| level0.last())
        .map_or(usize::MAX, |unit| unit.end_index);
    let tower0_units_raw = tower.first().map(|level0| level0.len());
    // **只在 lower 段重算时计数**（口径不动）：本观测面问的是「产 L1/L2 活窗那一刻段账本
    // 是否完整」，与 L3 的两项塔侧输入无关——挂到 union 上会让读数随 #602 新增重算次数漂移。
    if lower_due {
        match tower0_units_raw {
            None => lifecycle_stats.seg_ledger_no_tower += 1,
            Some(units) if units == l0.segments.len() => {
                lifecycle_stats.seg_ledger_complete += 1
            }
            Some(_) => lifecycle_stats.seg_ledger_short += 1,
        }
    }
    SegLedgerObservation {
        confirmed_last_end,
        tower0_units: tower0_units_raw.map_or(usize::MAX, |units| units),
    }
}

fn write_pan_live_diag_rows(
    dump: &mut Option<BufWriter<File>>,
    diag_rows: Vec<L1LiveDiagRow>,
    frontier: Option<ActiveSegmentFrontier>,
    index: usize,
) -> Result<(), String> {
    // 票 #601：行 tag 去掉写死的 "L1_"（同一诊断面现在同时承载 L1/L2），级别改由
    // 显式 `level=` 字段携带——tag 名与实际级别不符是声明膨胀（090）。
    for row in diag_rows {
        let tag = if row.c_start.is_some() {
            "PAN_LIVE_HIT"
        } else {
            "PAN_LIVE_MISS"
        };
        write_lifecycle_line(
            dump,
            format_args!(
                "{tag} as_of={index} level={} frontier_start={} reason={} b_center_start={} c_start={} gap_len={} seg_a={}",
                row.level,
                frontier.map_or(usize::MAX, |f| f.start_index),
                row.reason,
                row.b_center_start.map_or(usize::MAX, |value| value),
                row.c_start.map_or(usize::MAX, |value| value),
                row.gap_len.map_or(usize::MAX, |value| value),
                row.seg_a.map_or("none".to_string(), |(a, b)| format!("({a},{b})")),
            ),
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_pan_live_recompute_line(
    dump: &mut Option<BufWriter<File>>,
    index: usize,
    frontier: Option<ActiveSegmentFrontier>,
    l1_view: TowerScanView<'_>,
    l2_view: TowerScanView<'_>,
    l0_segments_len: usize,
    confirmed_last_end: usize,
    tower0_units: usize,
    before: usize,
    stems_after: usize,
) -> Result<(), String> {
    // 定位现场只在结构分量变化时落一行（诊断只写不判；每 bar 写会淹没 dump）。
    write_lifecycle_line(
        dump,
        format_args!(
            "PAN_LIVE_RECOMPUTE as_of={index} frontier={} l1_resume_from={} l2_resume_from={} confirmed_last_end={confirmed_last_end} tower0_units={tower0_units} l0_segments={} stems_before={before} stems_after={}",
            frontier.map_or("none".to_string(), |f| format!(
                "({},{},{:?})",
                f.start_index, f.extreme_at, f.direction
            )),
            l1_view.cursor.map_or(usize::MAX, |cursor| cursor.resume_from),
            l2_view.cursor.map_or(usize::MAX, |cursor| cursor.resume_from),
            l0_segments_len,
            stems_after,
        ),
    )
}

fn evaluate_and_apply_targeted_trigger(
    state: &mut TargetedPassState<'_>,
    tower: &[Rc<Vec<LeveledMove>>],
    classification: &classifier::Classification,
    targets: &BTreeMap<EventKey, NestCandidateEvent>,
    index: usize,
    shadow: bool,
) -> Result<(), String> {
    state.stats.triggers += 1;
    let runs_by_level = group_pending_runs_by_level(&state.pending, targets, index);
    let bsp_changed = refresh_bsp_snapshots(&runs_by_level, classification, &mut state.bsp_snaps);

    let mut out: Vec<NestCandidateEvent> = Vec::new();
    let mut fresh_levels: BTreeSet<usize> = BTreeSet::new();
    for (level, run_sources) in runs_by_level {
        if level == 0 || level >= tower.len() {
            continue;
        }
        evaluate_level_runs(
            state, level, run_sources, tower, index, shadow, &mut out, &mut fresh_levels,
        )?;
    }
    apply_targeted_events(state, out, targets, classification, &fresh_levels, &bsp_changed, index, shadow)
}

fn group_pending_runs_by_level(
    pending: &BTreeSet<EventKey>,
    targets: &BTreeMap<EventKey, NestCandidateEvent>,
    index: usize,
) -> BTreeMap<usize, BTreeSet<usize>> {
    // arrived 分组：与慢版 collect_target_candidates 首段逐行一致。
    let mut runs_by_level: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    for key in pending {
        let Some(event) = targets.get(key) else {
            continue;
        };
        // snapshot 无前视的结构下，turn_source 尚未到达时该对象不可能成为 Cand。
        // 提前投影这些终态目标只增加扫描量，不可能改变首次可证钟。
        if event.turn_source <= index {
            runs_by_level
                .entry(event.level as usize)
                .or_default()
                .insert(event.provider_window.0);
        }
    }
    runs_by_level
}

fn refresh_bsp_snapshots(
    runs_by_level: &BTreeMap<usize, BTreeSet<usize>>,
    classification: &classifier::Classification,
    bsp_snaps: &mut BTreeMap<usize, Vec<BspPoint>>,
) -> BTreeMap<usize, bool> {
    // 判据 (iv) 预备：本 trigger 全部 arrived 级的账本级做值比对并刷新快照。
    // 「变 ⟹ 反查」覆盖慢版每个可能新 Some 的 trigger；账本只在 trigger bar 可变
    // （模块头 (iv) 签字），故跨 trigger 快照比对 = 跨 bar 比对。
    let mut bsp_changed: BTreeMap<usize, bool> = BTreeMap::new();
    for &level in runs_by_level.keys() {
        let Some(book_level) = event_bsp_book_level(level as u32) else {
            continue;
        };
        let Some(level_state) = classification.levels.get(book_level) else {
            continue;
        };
        let current = &level_state.bsp[..];
        let changed = bsp_snaps
            .get(&book_level)
            .is_none_or(|snap| snap[..] != *current);
        if changed {
            bsp_snaps.insert(book_level, current.to_vec());
        }
        bsp_changed.insert(book_level, changed);
    }
    bsp_changed
}

#[allow(clippy::too_many_arguments)]
fn evaluate_level_runs(
    state: &mut TargetedPassState<'_>,
    level: usize,
    run_sources: BTreeSet<usize>,
    tower: &[Rc<Vec<LeveledMove>>],
    index: usize,
    shadow: bool,
    out: &mut Vec<NestCandidateEvent>,
    fresh_levels: &mut BTreeSet<usize>,
) -> Result<(), String> {
    // ── 派生缓存同步（targeted 与 lifecycle 共用同一 LevelDerived）──
    let (self_synced, lower_synced) = sync_level_derived(level, tower, &mut state.derived)?;
    if self_synced {
        state.stats.syncs_self += 1;
    }
    if lower_synced {
        state.stats.syncs_lower += 1;
    }
    {
        let level_derived = state.derived.get_mut(&level).expect("刚同步的 level 必须存在");
        let active_run_starts: Vec<_> = level_derived.run_ranges.keys().copied().collect();
        level_derived
            .confirm_cursors
            .retain_run_starts(level as u32, active_run_starts);
    }
    let stable_lower_len = state.cache.tower_confirmed_len(level - 1);
    let pan_freeze_boundary = state.cache.freeze_boundary(level - 1).unwrap_or(0);
    for run_source_start in run_sources {
        evaluate_single_run(
            state,
            level,
            run_source_start,
            tower,
            stable_lower_len,
            pan_freeze_boundary,
            index,
            shadow,
            out,
            fresh_levels,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn evaluate_single_run(
    state: &mut TargetedPassState<'_>,
    level: usize,
    run_source_start: usize,
    tower: &[Rc<Vec<LeveledMove>>],
    stable_lower_len: usize,
    pan_freeze_boundary: usize,
    index: usize,
    shadow: bool,
    out: &mut Vec<NestCandidateEvent>,
    fresh_levels: &mut BTreeSet<usize>,
) -> Result<(), String> {
    // 慢版：run 不在当前分区 ⟹ 本 trigger 无产出（continue）。条目保留不应用——
    // run 重现时代次差（分区已随内容变同步过）⟹ dirty ⟹ 重估，无陈旧复用。
    let Some(&range) = state.derived[&level].run_ranges.get(&run_source_start) else {
        return Ok(());
    };
    *state.stats.per_level_views_slow_would.entry(level).or_default() += 1;
    let watermark_crossed = compute_watermark_crossed(state, level, run_source_start, index);
    let dirty = compute_dirty(state, level, run_source_start, watermark_crossed);

    if dirty {
        reevaluate_run(
            state, level, run_source_start, range, tower, stable_lower_len, pan_freeze_boundary,
            index,
        )?;
        fresh_levels.insert(level);
    } else {
        state.stats.reuses += 1;
    }
    apply_run_events_and_shadow_check(
        state, level, run_source_start, range, tower, index, shadow, dirty, out,
    )
}

#[allow(clippy::too_many_arguments)]
fn apply_run_events_and_shadow_check(
    state: &mut TargetedPassState<'_>,
    level: usize,
    run_source_start: usize,
    range: (usize, usize),
    tower: &[Rc<Vec<LeveledMove>>],
    index: usize,
    shadow: bool,
    dirty: bool,
    out: &mut Vec<NestCandidateEvent>,
) -> Result<(), String> {
    // 应用（保序）：缓存事件 ∩ 当前 pending。pending 只缩不增 ⟹
    // 等价慢版「本 trigger 全量重估后 ∩ pending」（模块头判据完备性）。
    let applied_start = out.len();
    if let Some(entry) = state.entries.get(&(level, run_source_start)) {
        out.extend(
            entry
                .events
                .iter()
                .copied()
                .filter(|event| state.pending.contains(&EventKey::from(event))),
        );
    }
    // §6.3 shadow 强制对拍（P123_SHADOW=1 开启，长期回归开关）：无视 dirty 判定强制全量
    // 重估，与缓存路径应用集逐字比对。任何 mismatch = dirty 判据漏判现场（090 停线）。
    if shadow {
        shadow_check_run(
            state, level, run_source_start, range, tower, index, dirty, &out[applied_start..],
        )?;
    }
    Ok(())
}

fn compute_watermark_crossed(
    state: &TargetedPassState<'_>,
    level: usize,
    run_source_start: usize,
    index: usize,
) -> bool {
    let level_derived = &state.derived[&level];
    state
        .entries
        .get(&(level, run_source_start))
        .and_then(|entry| entry.target_last_as_of)
        .is_some_and(|last_as_of| {
            let before = level_derived
                .lower_ends
                .partition_point(|&end_index| end_index <= last_as_of);
            let now = level_derived
                .lower_ends
                .partition_point(|&end_index| end_index <= index);
            now != before
        })
}

fn compute_dirty(
    state: &mut TargetedPassState<'_>,
    level: usize,
    run_source_start: usize,
    watermark_crossed: bool,
) -> bool {
    let level_derived = &state.derived[&level];
    match state.entries.get(&(level, run_source_start)) {
        None => true, // 冷条目：首次评估
        Some(entry) => {
            let self_changed = entry.target_self_gen != Some(level_derived.self_gen);
            let lower_changed = entry.target_lower_gen != Some(level_derived.lower_gen);
            if watermark_crossed {
                if lower_changed {
                    state.stats.wm_cross_with_lower += 1;
                } else {
                    // 模块头证明 (iii)⊂(ii)：此计数恒 0，非 0 即 bug。
                    state.stats.wm_cross_without_lower += 1;
                }
            }
            self_changed || lower_changed || watermark_crossed
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn get_or_insert_run_entry(
    entries: &mut BTreeMap<(usize, usize), RunEntry>,
    level: usize,
    run_source_start: usize,
    self_gen: u64,
    lower_gen: u64,
    index: usize,
) -> &mut RunEntry {
    entries.entry((level, run_source_start)).or_insert_with(|| {
        RunEntry::new(self_gen, lower_gen, index, Vec::new(), Vec::new(), Vec::new())
    })
}

#[allow(clippy::too_many_arguments)]
fn call_evaluate_run(
    level: usize,
    tower: &[Rc<Vec<LeveledMove>>],
    range: (usize, usize),
    level_derived: &mut LevelDerived,
    entry: &mut RunEntry,
    index: usize,
    hist: &[f64],
    dif: &[f64],
    close_src: &[usize],
    stable_lower_len: usize,
    pan_freeze_boundary: usize,
    structure_generation: u64,
) -> Result<(Vec<Center>, Vec<Option<MoveKind>>, Vec<NestCandidateEvent>), String> {
    let lower_legs = &level_derived.lower_legs;
    let confirm_cursors = &mut level_derived.confirm_cursors;
    evaluate_run(
        level,
        &tower[level],
        range,
        lower_legs,
        index,
        hist,
        dif,
        close_src,
        Some(ConfirmResidence {
            store: confirm_cursors,
            stable_lower_len,
            structure_generation,
        }),
        Some(PanResidence {
            memo: &mut entry.pan_memo,
            freeze_boundary_src: pan_freeze_boundary,
        }),
    )
}

#[allow(clippy::too_many_arguments)]
fn reevaluate_run(
    state: &mut TargetedPassState<'_>,
    level: usize,
    run_source_start: usize,
    range: (usize, usize),
    tower: &[Rc<Vec<LeveledMove>>],
    stable_lower_len: usize,
    pan_freeze_boundary: usize,
    index: usize,
) -> Result<(), String> {
    let (hist, close_src) = state.cache.causal_series();
    let dif = state.cache.macd_dif();
    let level_derived = state.derived.get_mut(&level).expect("刚同步的 level 必须存在");
    let structure_generation = level_derived.self_gen;
    let entry = get_or_insert_run_entry(
        &mut state.entries,
        level,
        run_source_start,
        level_derived.self_gen,
        level_derived.lower_gen,
        index,
    );
    let (centers, kinds, events) = call_evaluate_run(
        level,
        tower,
        range,
        level_derived,
        entry,
        index,
        hist,
        dif,
        close_src,
        stable_lower_len,
        pan_freeze_boundary,
        structure_generation,
    )?;
    record_reevaluation_result(
        &mut state.views,
        &mut state.stats,
        level,
        entry,
        level_derived.self_gen,
        level_derived.lower_gen,
        index,
        centers,
        kinds,
        events,
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn record_reevaluation_result(
    views: &mut usize,
    stats: &mut SparseStats,
    level: usize,
    entry: &mut RunEntry,
    self_gen: u64,
    lower_gen: u64,
    index: usize,
    centers: Vec<Center>,
    kinds: Vec<Option<MoveKind>>,
    events: Vec<NestCandidateEvent>,
) {
    *views += 1;
    stats.reevals += 1;
    *stats.per_level_reevals.entry(level).or_default() += 1;
    entry.update(self_gen, lower_gen, index, centers, kinds, events);
    entry.target_self_gen = Some(self_gen);
    entry.target_lower_gen = Some(lower_gen);
    entry.target_last_as_of = Some(index);
}

#[allow(clippy::too_many_arguments)]
fn shadow_check_run(
    state: &mut TargetedPassState<'_>,
    level: usize,
    run_source_start: usize,
    range: (usize, usize),
    tower: &[Rc<Vec<LeveledMove>>],
    index: usize,
    dirty: bool,
    applied: &[NestCandidateEvent],
) -> Result<(), String> {
    let (hist, close_src) = state.cache.causal_series();
    let dif = state.cache.macd_dif();
    let lower_legs = &state.derived[&level].lower_legs;
    let (_, _, forced) = evaluate_run(
        level, &tower[level], range, lower_legs, index, hist, dif, close_src, None, None,
    )?;
    let forced: Vec<NestCandidateEvent> = forced
        .into_iter()
        .filter(|event| state.pending.contains(&EventKey::from(event)))
        .collect();
    state.stats.shadow_checks += 1;
    let same = applied.len() == forced.len()
        && applied.iter().zip(forced.iter()).all(|(a, b)| {
            EventKey::from(a) == EventKey::from(b) && a.divergence_confirmed == b.divergence_confirmed
        });
    if !same {
        state.stats.shadow_mismatches += 1;
        if state.stats.shadow_mismatches <= 20 {
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
    Ok(())
}

fn apply_targeted_events(
    state: &mut TargetedPassState<'_>,
    out: Vec<NestCandidateEvent>,
    targets: &BTreeMap<EventKey, NestCandidateEvent>,
    classification: &classifier::Classification,
    fresh_levels: &BTreeSet<usize>,
    bsp_changed: &BTreeMap<usize, bool>,
    index: usize,
    shadow: bool,
) -> Result<(), String> {
    // ── 事件应用循环：与 p116 逐行一致，仅 TERM 查法加判据 (iv) 门控 ──
    for event in out {
        let key = EventKey::from(&event);
        let pending_hit = state.pending.contains(&key);
        // #553 候选事件 dump（只写不判）：写在既有 pending 短路**之前** ⟹ 落盘的是
        // 完整候选事件流（含本 trigger 内已被前序事件出清的观察）。`pending.contains`
        // 是纯查询，提出到 `pending_hit` 不改变既有控制流与既有 dump 行的产生顺序。
        state.event_dump.observe(&event, index, pending_hit)?;
        if !pending_hit {
            continue;
        }
        state.book.candidates.entry(key.clone()).or_insert(index);
        let target = targets.get(&key).expect("pending target");
        if event.divergence_confirmed {
            // #116: DIV = divergences 账本首次插入瞬间（账本语义同 p92 or_insert）。
            if !state.book.divergences.contains_key(&key) {
                dump_line(format_args!(
                    "DIV level={} end={} kind={} side={:?}",
                    event.level,
                    event.turn_source,
                    nest_kind_tag(event.kind),
                    event.side,
                ));
            }
            state.book.divergences.entry(key.clone()).or_insert(index);
        }
        record_term_signal(state, &key, &event, classification, fresh_levels, bsp_changed, index, shadow);
        if event.divergence_confirmed == target.divergence_confirmed {
            state.pending.remove(&key);
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn record_term_signal(
    state: &mut TargetedPassState<'_>,
    key: &EventKey,
    event: &NestCandidateEvent,
    classification: &classifier::Classification,
    fresh_levels: &BTreeSet<usize>,
    bsp_changed: &BTreeMap<usize, bool>,
    index: usize,
    shadow: bool,
) {
    // #116: TERM = 事件键首次终端 bits 确认（dump 专用 term_seen，不动账本）。
    // p123 判据 (iv)：fresh（本 trigger 重估过 ⟹ 窗口可能变）∨ 账本内容变
    // ⟹ 反查；其余情形结果为上次已查的同一 None（纯函数同输入），跳过逐位等价。
    let recheck = fresh_levels.contains(&(event.level as usize))
        || event_bsp_book_level(event.level).is_some_and(|book_level| {
            bsp_changed.get(&book_level).copied().unwrap_or(true)
        });
    if recheck {
        state.stats.term_rechecks += 1;
        if !state.book.term_seen.contains(key) && terminal_bits_new(classification, event).is_some() {
            state.book.term_seen.insert(key.clone());
            dump_line(format_args!(
                "TERM level={} bar={} side={:?}",
                event.level, event.turn_source, event.side,
            ));
        }
    } else {
        state.stats.term_skips += 1;
        // §6.3 shadow (iv) 门控对拍：跳过的 TERM 检查若本 trigger 实为 Some 且
        // term_seen 未录 ⟹ 慢版本 trigger 会首见而 sparse 漏 = 判据 (iv) 漏判。
        if shadow
            && !state.book.term_seen.contains(key)
            && terminal_bits_new(classification, event).is_some()
        {
            state.stats.shadow_term_mismatches += 1;
            if state.stats.shadow_term_mismatches <= 20 {
                eprintln!(
                    "P123_SHADOW_TERM_MISMATCH level={} as_of={index} key={key:?}",
                    event.level,
                );
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn feed_lifecycle_and_checkpoint(
    state: &mut TargetedPassState<'_>,
    tower: &[Rc<Vec<LeveledMove>>],
    classification: &classifier::Classification,
    index: usize,
    lifecycle_due: bool,
    trigger: TriggerKey,
    bars_len: usize,
    targets_len: usize,
    ckpt_every: usize,
    started: Instant,
) -> Result<(), String> {
    // #421：targeted 先按原物理 views 口径更新共享条目；sidecar 的 provider trigger
    // 随后只补 dirty 非目标 run，并刷新完成事件。活窗由上方独立 p409 同构循环发现，
    // 不借生产 RunEntry 的完成时刻快照。账本本身每根 bar 都喂。
    let completion_events = refresh_lifecycle_if_due(state, tower, index, lifecycle_due, trigger)?;
    feed_lifecycle_bar_and_dump(state, tower, index, &completion_events)?;
    run_periodic_checkpoint(state, tower, classification, index, ckpt_every)?;
    print_prefix_progress(state, index, bars_len, targets_len, started);
    Ok(())
}

fn refresh_lifecycle_if_due(
    state: &mut TargetedPassState<'_>,
    tower: &[Rc<Vec<LeveledMove>>],
    index: usize,
    lifecycle_due: bool,
    trigger: TriggerKey,
) -> Result<Vec<NestCandidateEvent>, String> {
    if !lifecycle_due {
        return Ok(Vec::new());
    }
    let (hist, close_src) = state.cache.causal_series();
    let dif = state.cache.macd_dif();
    let active_runs = refresh_lifecycle_cache(
        &state.cache,
        RefreshWorld {
            tower,
            as_of: index,
            series: CausalSeries {
                hist,
                dif,
                close_src,
            },
        },
        &mut LifecycleCacheState {
            derived: &mut state.derived,
            entries: &mut state.entries,
            stats: &mut state.lifecycle_stats,
        },
    )?;
    let completion_events = active_runs
        .iter()
        .flat_map(|key| state.entries[key].events.iter().copied())
        .collect();
    state.lifecycle_stats.provider_triggers += 1;
    state.last_lifecycle_trigger = Some(trigger);
    Ok(completion_events)
}

fn feed_lifecycle_bar_and_dump(
    state: &mut TargetedPassState<'_>,
    tower: &[Rc<Vec<LeveledMove>>],
    index: usize,
    completion_events: &[NestCandidateEvent],
) -> Result<(), String> {
    let bar_windows: Vec<PanLiveWindow> = state
        .lifecycle_window_stems
        .iter()
        .copied()
        .map(|stem| stem.window_at(index))
        .collect();
    // #527：完成相是显式 typed 输出——每条完成事件都必须能在塔上查到那只已完成的
    // lower unit（身份 + 物理完成 bar），否则停线（禁按 kind 猜 provenance）。
    let phases = lifecycle_bar_phases(tower, &bar_windows, completion_events, index)?;
    let (hist, close_src) = state.cache.causal_series();
    let dif = state.cache.macd_dif();
    feed_lifecycle_bar(
        &mut state.lifecycle_book,
        &phases,
        index,
        CausalSeries {
            hist,
            dif,
            close_src,
        },
        &mut LifecycleDumpSink {
            sink: &mut state.lifecycle_dump,
            stats: &mut state.lifecycle_stats,
        },
    )?;
    state.lifecycle_book.assert_invariants();
    Ok(())
}

fn run_periodic_checkpoint(
    state: &TargetedPassState<'_>,
    tower: &[Rc<Vec<LeveledMove>>],
    classification: &classifier::Classification,
    index: usize,
    ckpt_every: usize,
) -> Result<(), String> {
    if !(ckpt_every > 0 && index > 0 && index % ckpt_every == 0) {
        return Ok(());
    }
    let (hist, close_src) = state.cache.causal_series();
    let dif = state.cache.macd_dif();
    let mut ckpt_audit = ProviderAudit::default();
    match collect_snapshot_candidates(tower, index, hist, dif, close_src, &mut ckpt_audit) {
        Ok(by_level) => checkpoint_certificates(&by_level, classification, index),
        Err(error) => {
            dump_line(format_args!("CKPT_ERR as_of={index} err={error}"));
        }
    }
    Ok(())
}

fn print_prefix_progress(
    state: &TargetedPassState<'_>,
    index: usize,
    bars_len: usize,
    targets_len: usize,
    started: Instant,
) {
    if !(index > 0 && index % 500_000 == 0) {
        return;
    }
    eprintln!(
        "P123_PREFIX_PROGRESS bar={index}/{} elapsed={:.1}s pending={}/{} views={} triggers={} reevals={}",
        bars_len,
        started.elapsed().as_secs_f64(),
        state.pending.len(),
        targets_len,
        state.views,
        state.stats.triggers,
        state.stats.reevals,
    );
}

fn finalize_targeted_pass(
    mut state: TargetedPassState<'_>,
) -> Result<(YieldBook, usize, usize, SparseStats, LifecycleReplayStats), String> {
    for entry in state.entries.values() {
        let pan = entry.pan_memo.stats();
        state.stats.pan_entries += entry.pan_memo.len();
        state.stats.pan_hits += pan.hits;
        state.stats.pan_misses += pan.misses;
        state.stats.pan_writes += pan.writes;
        state.stats.pan_invalidations += pan.invalidations;
    }
    state.lifecycle_stats.settlement = state.lifecycle_book.settlement_stats();
    write_lifetime_dump_line(&mut state.lifecycle_dump, state.lifecycle_stats.settlement)?;
    if let Some(writer) = state.lifecycle_dump.as_mut() {
        writer
            .flush()
            .map_err(|error| format!("刷新 P421_LIFECYCLE_DUMP 失败: {error}"))?;
    }
    // #553：事件 dump 收尾 flush 排在既有 P421_LIFECYCLE_DUMP 收尾 flush **之后**
    //（与 ticket-553 原口径同序；两者分属独立 sink，互不触碰）。
    state.event_dump.flush()?;
    // #641：链 dump 收尾 flush 排在 #553 事件 dump 收尾 flush **之后**（与驻车线原口径同序；
    // 四个 sink 各自独立，互不触碰）。
    state.chain_dump.flush()?;
    let pending_len = state.pending.len();
    Ok((
        state.book,
        state.views,
        pending_len,
        state.stats,
        state.lifecycle_stats,
    ))
}

fn write_lifetime_dump_line(
    dump: &mut Option<BufWriter<File>>,
    settlement: LifecycleSettlementStats,
) -> Result<(), String> {
    write_lifecycle_line(
        dump,
        format_args!(
            "LIFETIME entries={} first_provable={} provisional={} confirmed={} force_overtake={} never_constituted={} identity_vanished_refuted={} identity_vanished_seam={} flash_terminal={} nonflash_count={} nonflash_min={:?} nonflash_median={:?} nonflash_max={:?} force_lifetime_count={} force_lifetime_min={:?} force_lifetime_median={:?} force_lifetime_max={:?}",
            settlement.entry_count,
            settlement.first_provable_count,
            settlement.provisional_count,
            settlement.confirmed_count,
            settlement.force_overtake_count,
            settlement.never_constituted_count,
            settlement.identity_vanished_refuted_count,
            settlement.identity_vanished_seam_count,
            settlement.flash_terminal_count,
            settlement.nonflash_lifetime.count,
            settlement.nonflash_lifetime.min,
            settlement.nonflash_lifetime.median,
            settlement.nonflash_lifetime.max,
            settlement.force_overtake_lifetime.count,
            settlement.force_overtake_lifetime.min,
            settlement.force_overtake_lifetime.median,
            settlement.force_overtake_lifetime.max,
        ),
    )
}

fn write_lifecycle_line(
    sink: &mut Option<BufWriter<File>>,
    args: std::fmt::Arguments<'_>,
) -> Result<(), String> {
    let Some(writer) = sink.as_mut() else {
        return Ok(());
    };
    writer
        .write_fmt(args)
        .and_then(|()| writer.write_all(b"\n"))
        .map_err(|error| format!("写 P421_LIFECYCLE_DUMP 失败: {error}"))
}

/// 同步 targeted/lifecycle 共用的 per-level 派生面；只在内容变化时重建。
fn sync_level_derived(
    level: usize,
    tower: &[Rc<Vec<LeveledMove>>],
    derived: &mut BTreeMap<usize, LevelDerived>,
) -> Result<(bool, bool), String> {
    let level_derived = derived.entry(level).or_default();
    let self_synced = level_derived.self_snap[..] != tower[level][..];
    if self_synced {
        level_derived.self_snap = tower[level][..].to_vec();
        level_derived.self_gen += 1;
        level_derived.run_ranges = build_run_ranges(&tower[level]);
    }
    let lower_synced = level_derived.lower_snap[..] != tower[level - 1][..];
    if lower_synced {
        level_derived.lower_snap = tower[level - 1][..].to_vec();
        level_derived.lower_gen += 1;
        level_derived.lower_legs = lower_legs_from(&tower[level - 1])
            .map_err(|error| format!("L{level} shared lower legs 失败: {error:?}"))?;
        level_derived.lower_ends = level_derived
            .lower_legs
            .iter()
            .map(|leg| leg.end_index)
            .collect();
    }
    Ok((self_synced, lower_synced))
}

/// `level_view.rs:436` 私有转换的 sidecar 本地复制；与 p409 探针逐字同口径。
fn lifecycle_leg_as_segment(value: &LowerLeg) -> Segment {
    let (start_price, end_price) = match value.direction {
        Direction::Up => (value.lo, value.hi),
        Direction::Down => (value.hi, value.lo),
    };
    Segment {
        direction: value.direction,
        start_index: value.start_index,
        end_index: value.end_index,
        start_price,
        end_price,
    }
}

/// #601/#602：塔侧活动语义的只读输入三元，**按「产出 `tower[level]` 的那一级」取值**
/// （三个访问器同一下标口径，见 `TowerCache::level_scan_units` 文档）。
#[derive(Clone, Copy)]
struct TowerScanView<'a> {
    /// `TowerCache::level_scan_units(level)`——该级窗口扫描的 confirmed 输入 units。
    units: Option<&'a [UnitRange]>,
    /// `TowerCache::level_scan_cursor(level)`——该级扫描断点（重扫锚取其 `resume_from`）。
    cursor: Option<WindowScanCursor>,
    /// `TowerCache::tower_confirmed_len(level)`——`tower[level]` 的确认水线（#613 整窗截断口径）。
    confirmed_len: usize,
}

/// #527/#601/#613/#602 活窗发现：**confirmed A/B 锚 + 该级别的 active C 腿**。
///
/// 与 #421 逃生门原实装的差别（#523 根因）：C 不再从已完成 lower legs 回放重建，而只能是
/// **塔上尚不存在的行进中腿**——故活窗可在完成前出生。B 中枢仍取 confirmed 侧
/// （`tower[level]` 的 run 投影 seeds），A 锚仍取 confirmed 段（窄锚/A′ 与 provider 同序同判）。
///
/// **级别范围（票 #601 立 L2、票 #602 补 L3；#598 裁定路线 i，先 L2 后 L3）**：
/// - **L1**：C = parser 的行进中段（`ActiveSegmentFrontier`，`OpenTail.pendingSegment`）；
/// - **L2**：C = **行进中的 L1 窗口单元**（[`active_l1_window_frontier`]）——由「`tower[0]`
///   confirmed units + L0 行进中段虚拟追加」重跑 L1 层窗口判据（`center_from_segments`）派生；
/// - **L3**：C = **行进中的 L2 窗口单元**（[`active_l2_window_frontier`]）——由「`tower[1]`
///   投影 units + 上一条的产物虚拟追加」重跑 L2 层窗口判据（`center_from_window`，几何路径）
///   派生。**递归复合一次，不是把 L2 的实装换个参数**：build、输入层、方向来源三处全变
///   （见该函数文档）。
///
/// 三级共同的 #523 红线：**都不是** `tower[level]` 的任何已产出窗口（拿后者冒充活动 =
/// 与完成事件同源同判 = 重演「首见即完成」）。
///
/// - **L4 及以上**：本函数不产活窗（上界写死 `1..4`）。其身份仍只经完成相进账本，闪现是
///   provider 能力缺口，不是 true-flash——**不外推未验证的级别**（#527 §7 纪律）。
///
/// **分级重算（票 #602；`lead-parallel-dispatch` 局部依赖原则的直接落地）**：`levels` 指定
/// 本次要重算哪几级。L1/L2 与 L3 的塔侧输入**不同**——L3 多两个输入（`level_scan_cursor(2)` /
/// `tower_confirmed_len(2)`），少的那两级不消费它们。把六项并成一个键会**过度失效** L1/L2：
/// 重算频率变高 ⟹ 含 `as_of` 的守卫（`PanLiveOutcome::FrontierAheadOfClock` 是
/// `active.end_index > as_of`）在更早的 bar 上被重判 ⟹ L1/L2 的产出**真的会变**
/// （实测：并键版 `l1:window` 610→615）。故两段各按自己的键重算，L1/L2 逐位不动。
///
/// `carry` 承载 L2 派生的中间量供 L3 复用（递归复合；其输入全在 lower 键内 ⟹ L3 单独重算
/// 时取用不会陈旧）。
#[allow(clippy::too_many_arguments)]
fn recompute_lifecycle_window_stems(
    levels: std::ops::Range<usize>,
    tower: &[Rc<Vec<LeveledMove>>],
    frontier: Option<&ActiveSegmentFrontier>,
    l1_view: TowerScanView<'_>,
    l2_view: TowerScanView<'_>,
    carry: &mut LowerFrontierCarry,
    as_of: usize,
    outcome_tally: &mut BTreeMap<(u32, &'static str), usize>,
    diag_rows: &mut Vec<L1LiveDiagRow>,
) -> Vec<LifecycleWindowStem> {
    // 调用点约定（票 #629 S3 订正）：本函数目前只由两处驱动——`1..3`（L1/L2，`levels.start==1`）
    // 与 `3..4`（L3）——其并集覆盖 `level ∈ {1,2,3}`，是下方 `match level` 穷尽 `1`/`2|3` 两臂
    // 以及 L1 臂 `frontier.expect(...)` 成立的前提。本函数不接受第三个调用点，该断言是对
    // 「调用点约定」本身的显式核验，而非对循环上界的核验（`for level in levels` 没有写死的
    // 上界，`levels` 由调用方传入）。
    debug_assert!(
        levels.clone() == (1..3) || levels.clone() == (3..4),
        "recompute_lifecycle_window_stems 仅由两个调用点驱动（1..3 / 3..4）；\
         其它 levels 范围未经验证，match level 的穷尽性与 L1 臂的 frontier.expect 均系于此约定"
    );
    let mut windows_out = Vec::new();
    /// 记一条未命中诊断（tally + dump 行，两处同码同源——禁两处各写一遍产生漂移）。
    fn miss(
        outcome_tally: &mut BTreeMap<(u32, &'static str), usize>,
        diag_rows: &mut Vec<L1LiveDiagRow>,
        level: usize,
        reason: &'static str,
    ) {
        *outcome_tally.entry((level as u32, reason)).or_default() += 1;
        diag_rows.push(L1LiveDiagRow {
            level: level as u32,
            reason,
            b_center_start: None,
            c_start: None,
            gap_len: None,
            seg_a: None,
        });
    }
    if frontier.is_none() && levels.start == 1 {
        // 无行进中 L0 段 ⟹ L1/L2 都没有活动 C 腿（L2 的腿也由该段虚拟追加派生），且 L3 的
        // 虚拟单元（= L2 的活动腿）随之失效 ⟹ 清 carry，L3 那一段自会落 `no_lower_frontier`。
        // 不回落到 confirmed 回放重建。
        miss(outcome_tally, diag_rows, 0, "no_active_frontier");
        *carry = LowerFrontierCarry::default();
        return Vec::new();
    }
    // 上界写死 4（= 覆盖 L1/L2/L3），级别在塔上是否已涌现由**显式原因码**回答而不是静默跳过：
    // 完成事件的物理完成 bar 可以远早于其首次可见 bar（BTC 100k 滞后最大 4702），故一只 L2/L3
    // 身份的 C 活跃期可能整段落在「塔还没长出该级」的时期——那时不产活窗是结构事实，但必须
    // 能落到码上，否则该身份在归因表里没有任何行（#527 §9.2 补记命中行同一纪律）。
    for level in levels {
        if level == 2 {
            // 本级产物先清空——**任何**一条 `continue` 路径都不得给 L3 留下上一轮的陈旧活动腿
            // （陈旧腿 = 拿别的 bar 的塔状态派生 L3，是凭空发明状态）。成功时在下方回填。
            *carry = LowerFrontierCarry::default();
        }
        if level >= tower.len() {
            miss(outcome_tally, diag_rows, level, "tower_level_absent");
            continue;
        }
        // 本级 confirmed 腿源。投影失败（`first_leaf_direction` 为 None ⟹ 空 Compose）原实装
        // 静默 `continue`，在归因表上不留任何行（「零不知道」的一条静默通道）——票 #602 补码。
        let Ok(lower) = lower_legs_from(&tower[level - 1]) else {
            miss(outcome_tally, diag_rows, level, "lower_legs_unprojectable");
            continue;
        };
        if level == 2 {
            // `lower_legs_from(&tower[1])` 在这一轮已算（作 confirmed 段源），L3 需要它的
            // direction 列作首叶方向表——复用同一份，不二次投影（禁第二查法）。
            carry.l1_leg_dirs = lower.iter().map(|leg| leg.direction).collect();
        }
        let mut segments: Vec<Segment> = lower.iter().map(lifecycle_leg_as_segment).collect();
        // 该级别的 active C 腿 + confirmed 侧的整窗截断水线（`None` = 本级不截断，仅 L1）。
        let (active, truncate_to) = match level {
            1 => (
                // 本臂能成立系于调用点约定（函数头 debug_assert，票 #629 S3）：level==1 只在
                // `levels.start==1` 的那次调用（即 `1..3`）里出现，而该调用在 `frontier.is_none()`
                // 时已于上面 `levels.start == 1` 分支早退——故走到这里时 `frontier` 恒为 `Some`。
                // 这不是本函数内部可推导的不变式，是调用点保证；`levels` 若被传入其它范围，
                // 本 `.expect` 与函数头断言会同时失守。
                frontier
                    .expect("levels.start==1 ⟹ frontier 为 None 时已在上面早退（调用点约定，见函数头 debug_assert）")
                    .as_segment(),
                None,
            ),
            2 | 3 => {
                // 视图下标 = 「产出 `tower[level-1]` 的那一级」：level==2 用 L1 层扫描
                // （消费 `tower[0]` 的投影 units），level==3 用 L2 层扫描（消费 `tower[1]`）。
                let view = if level == 2 { l1_view } else { l2_view };
                // ★#613（收 #609 F2）：扫描输入 units 与 `tower[level-1]` 的同长契约在消费点复核。
                // 根因已在 classifier 侧修死（段账本回缩的 `cache.clear()` 前移到 units 构建之前，
                // 见 `TowerCache::l0_units` / `level_scan_units` 的同长不变式），本守卫是**契约面
                // 的独立可观测**：若未来重构再次破坏该不变式，本级落**本码**而不是让派生静默走到
                // 重扫上、因 `resume_from==0` 时 `i + 2 < 1` 不成立而伪装成 `no_window_formed`
                // （#609 F2 钉出的静默通道）。不变式成立时本码恒 0。
                let Some(units) = view.units else {
                    miss(outcome_tally, diag_rows, level, "scan_units_absent");
                    continue;
                };
                // 同长基准是 `tower[level - 2]`（该扫描的**输入**塔层），不是 `tower[level - 1]`
                // （它的**产出**）：level==2 的输入是 `tower[0]`、level==3 的输入是 `tower[1]`。
                if units.len() != tower[level - 2].len() {
                    miss(outcome_tally, diag_rows, level, "scan_units_out_of_sync");
                    continue;
                }
                // 本级扫描断点缺失（塔尚未产出该级缓存）⟹ 不猜锚，记原因码后跳过本级。
                let Some(cursor) = view.cursor else {
                    miss(outcome_tally, diag_rows, level, "resume_anchor_out_of_range");
                    continue;
                };
                // ★#602 **输入侧**的整窗截断（与 #613 F1 的 confirmed 段侧截断同一条水线，
                // 但作用在扫描输入上；**L3 专有，是结构差异不是折中**）：
                // - L2 的扫描输入 `l0_units` 是 parser **已 emit** 段的投影，行进中段不在其中
                //   ⟹ 输入与虚拟单元天然不重叠，无需截断；
                // - L3 的扫描输入 `l1_units` 是 `tower[1]` 的投影，**含塔的开放末窗单元**，而
                //   虚拟单元正是那批开放单元「计入行进中 L0 段」后的形态 ⟹ 不截断则恒倒灌
                //   （BTC 100k 探针实测 `lower_frontier_not_after_units` 728 次）。
                // 截到 `tower_confirmed_len(1)`——与本级 confirmed 段侧用的是同一条水线证书
                // （#93 单一来源，禁第二查法）。截断后 `resume_from` 可能落在被截区间内，此时
                // `scan_active_window` 的锚守卫落 `resume_anchor_out_of_range`（诚实空产出，
                // **不**把锚往前拽——那是猜塔没走过的扫描路径）。
                let scan_units = if level == 3 {
                    &units[..l1_view.confirmed_len.min(units.len())]
                } else {
                    units
                };
                let outcome = if level == 2 {
                    active_l1_window_frontier(scan_units, frontier, cursor.resume_from)
                } else {
                    // L3：虚拟单元 = L2 那一段派生的行进中 L1 窗口单元；首叶方向表 = 同一段
                    // 的 `lower_legs_from(tower[1])`。两者缺失（L2 派生失败或本 bar 无行进中
                    // L0 段）⟹ 本级落 `no_lower_frontier`，成因在同 bar 的 L2 诊断行上。
                    //
                    // 登记（票 #629 S2，订正登记口径）：下面这个 `min()` 对 `l1_leg_dirs` 与
                    // `scan_units` 做静默截齐，使 `ActiveWindowOutcome::LowerLegDirsOutOfSync`
                    // 守卫在**本调用点**结构性不可达——`lower_legs_from`/`project_to_units` 均是
                    // 对 `tower[1]` 的 1:1 map（长度天然相等），且上面的 `scan_units_out_of_sync`
                    // 已先拦同长破坏，两者不可能不等长走到这里。这与 `scan_units_absent`/
                    // `lower_legs_unprojectable` 的「数据性未触发」（能力已具备、本窗数据未命中）
                    // 不是同一类——那两码在生产数据变化下可能触发，这一条在当前唯一调用点上
                    // 恒不可达。守卫本身对 API 的其它潜在调用方仍然有效，不删除。
                    active_l2_window_frontier(
                        scan_units,
                        &carry.l1_leg_dirs[..carry.l1_leg_dirs.len().min(scan_units.len())],
                        carry.l1_frontier.as_ref(),
                        cursor.resume_from,
                    )
                };
                let Some(active_window) = outcome.frontier() else {
                    miss(outcome_tally, diag_rows, level, outcome.reason_tag());
                    continue;
                };
                if level == 2 {
                    carry.l1_frontier = Some(active_window);
                }
                (active_window.as_segment(), Some(view.confirmed_len))
            }
            // level ∈ {1,2,3} 不是循环上界写死的结果——`levels` 是调用方传入的参数，本函数
            // 内部不设上界。成立系于调用点约定（票 #629 S3）：仅 `1..3` ∪ `3..4` 两处调用，
            // 并集恰为 {1,2,3}（函数头 debug_assert 核验该约定）。
            _ => unreachable!(
                "level ∈ {{1,2,3}} 由调用点保证（1..3 ∪ 3..4，见函数头 debug_assert），\
                 非循环上界写死"
            ),
        };
        // ★confirmed 与 active 不得重叠（L1 的对应事实：parser 的 `l0.segments` 天然不含
        // pending 段）。塔在这一点上**不同**：`tower[k]` 的末窗即使仍开放（未被 non-extension
        // 单元终结、每 bar pop 重扫），在类型上也与确认窗口不可区分（#598 §1.1）。行进中的
        // 上级单元正是该末窗「计入行进中下级单元」后的形态。若不截断，末窗会同时以 confirmed
        // 与 active 两个身份进入判定，`active.start_index < last.end_index` 恒真 ⟹ 该级恒判
        // `frontier_not_after_confirmed`（L2 上 BTC 100k 实测 844 次）。
        //
        // ★#613（收 #609 F1）：截断口径 = **整窗**，取塔自己的确认水线
        // `TowerCache::tower_confirmed_len(level - 1)`（`upper_moves.len() -
        // scan_cursor.last_window_emitted`，cascade bar 另取保留前缀 min(P)），即「哪些塔单元
        // 算确认」的单一来源（#93 水线证书，`classifier/mod.rs` 自称禁第二查法）。**不**按
        // 「同起点 pop 1 个」：塔的 frontier 回退域是整窗产出
        // （`WindowScanCursor::last_window_emitted` 原文「只 pop 1 会残留旧子中枢」），#148
        // 升级重切窗一窗产 ⌊n/3⌋ 个子中枢，BTC 100k 实测该游标 3–7 占 20.0%（227/1134）；单 pop
        // 在这些 bar 上残留 2–6 个**跨 bar 可变**的子单元留在 confirmed 侧，而 seg_a 正是从这批
        // 段里定位、且 seg_a 进身份键 ⟹ 身份漂移通道（#609 §3 实测覆盖 58/314 真产出活窗）。
        // 水线口径把这条通道整段关掉。
        //
        // ★票 #602：**L3 与 L2 同构**（`tower[2]` 的开放末窗与确认窗口同样类型上不可区分，
        // #601 §7 遗留 1 点名的「tower[2] 侧开放末窗截断未做」即本条）——按级别取
        // `tower_confirmed_len(level - 1)`，同一口径同一来源，不为 L3 另立判据。
        //
        // 方向安全：水线是**保守下界**（over-shrink 恒 sound、over-grow 禁止），截多了只会少产
        // 活窗，不会凭空产出；截断后 confirmed 侧全部元素跨 bar 逐字节稳定 ⟹ seg_a 的输入稳定。
        //
        // **L1 不套用本口径**（不是折中，是结构差异）：L1 的 confirmed 侧是 `tower[0]` = parser
        // 段账本投影，其 active（parser 的 pending 段）**不在** `l0.segments` 里 ⟹ F1 的重叠机制
        // 在 L1 上不存在，按水线截断只会误删真已终结的单元。「L1 confirmed 尾段跨 bar 可重划
        // （古怪线段）」是**另一条**缺陷线索，与本条重叠无关，不在本票 Scope（见交付报告遗留）。
        if let Some(watermark) = truncate_to {
            segments.truncate(watermark);
            // 截断口径的**可执行契约**（不是注释保证）：水线之外的单元全部属于最后一个成立窗口
            // （或 cascade 保留前缀之后），而行进中的上级单元由该窗口起点（`resume_from`）重扫
            // 派生 ⟹ 其起点不可能早于末确认单元的终点。旧的「同起点 pop 1 个」在
            // `last_window_emitted > 1` 时**违反**本式（#609 F1 实测 45/93 残留），故本式同时是
            // 「整窗口径已生效」的判别。
            debug_assert!(
                segments
                    .last()
                    .is_none_or(|last| last.end_index <= active.start_index),
                "#613 F1 / #602：level={level} 截断到确认水线后 confirmed 侧仍与 active 重叠（末端 {:?} > active 起点 {}）",
                segments.last().map(|last| last.end_index),
                active.start_index
            );
        }
        let windows = &tower[level];
        let mut run_start = None;
        for index in 0..=windows.len() {
            let valid = index < windows.len()
                && project_extended_windows_carried_only(std::slice::from_ref(&windows[index]))
                    .is_ok();
            match (run_start, valid) {
                (None, true) => run_start = Some(index),
                (Some(start), false) => {
                    if let Ok(projection) =
                        project_extended_windows_carried_only(&windows[start..index])
                    {
                        let centers: Vec<_> =
                            projection.seeds.iter().map(|seed| seed.center).collect();
                        let blocks = decompose::decompose(&centers);
                        let kinds = decompose::center_block_kind(centers.len(), &blocks);
                        let outcome = provide_active_pan_live_windows(
                            level as u32,
                            &centers,
                            &kinds,
                            &segments,
                            active,
                            as_of,
                        );
                        *outcome_tally
                            .entry((level as u32, outcome.reason_tag()))
                            .or_default() += 1;
                        // 逐 run 落一行（命中/未命中都记）：供「这只完成身份为何没有更早
                        // Live」逐身份归因（诊断只写不判）。未命中取该 run 最近中枢起点作锚
                        // 提示；命中取产窗身份的 B 与 λ_C。
                        diag_rows.push(match outcome.window() {
                            Some(window) => L1LiveDiagRow {
                                level: level as u32,
                                reason: outcome.reason_tag(),
                                b_center_start: Some(window.b_center_start),
                                c_start: Some(window.seg_c_live.0),
                                gap_len: Some(window.gap_len),
                                seg_a: Some(window.seg_a),
                            },
                            None => L1LiveDiagRow {
                                level: level as u32,
                                reason: outcome.reason_tag(),
                                b_center_start: centers
                                    .iter()
                                    .rev()
                                    .find(|center| center.end_index <= active.start_index)
                                    .map(|center| center.start_index),
                                c_start: None,
                                gap_len: None,
                                seg_a: None,
                            },
                        });
                        windows_out.extend(outcome.window());
                    }
                    run_start = None;
                }
                _ => {}
            }
        }
    }
    let mut stems: Vec<LifecycleWindowStem> =
        windows_out.iter().map(LifecycleWindowStem::of).collect();
    stems.sort();
    stems.dedup();
    stems
}

/// #532：`hist`/`dif`/`close_src` 三元 Data Clumps 收束——三者恒由同一
/// `cache.causal_series()` + `cache.macd_dif()` 同源产出、逐调用同行同现，收成一个借用体。
#[derive(Clone, Copy)]
struct CausalSeries<'a> {
    hist: &'a [f64],
    dif: &'a [f64],
    close_src: &'a [usize],
}

/// #532：lifecycle 缓存刷新期间需要连带可变的三张账——收成一个上下文体，避免调用方
/// 逐参数搬运。生命周期彼此独立（各自独立 `&mut` 借用），不共享单一底层容器。
struct LifecycleCacheState<'a> {
    derived: &'a mut BTreeMap<usize, LevelDerived>,
    entries: &'a mut BTreeMap<(usize, usize), RunEntry>,
    stats: &'a mut LifecycleReplayStats,
}

/// #532：`refresh_lifecycle_cache` 的只读上游输入（塔、当前 bar、因果序列三元组）打包，
/// 供内部逐 level / 逐 run 提取的辅助函数共享，避免逐层重复搬运同一组只读引用。
struct RefreshWorld<'a> {
    tower: &'a [Rc<Vec<LeveledMove>>],
    as_of: usize,
    series: CausalSeries<'a>,
}

/// 单个 run 在 `refresh_run_entry` 中定位与判脏所需的标量集合。
struct RunRefreshContext {
    level: usize,
    run_source_start: usize,
    run: (usize, usize),
    stable_lower_len: usize,
    pan_freeze_boundary: usize,
}

/// 补齐 lifecycle 本 trigger 所需的全部 active run。targeted 已在同 trigger 更新过的
/// 条目直接复用；其余条目也只在共享 dirty 判据命中时评估，禁 sidecar 每 trigger 全量重算。
fn refresh_lifecycle_cache(
    cache: &classifier::TowerCache,
    world: RefreshWorld,
    state: &mut LifecycleCacheState,
) -> Result<Vec<(usize, usize)>, String> {
    let mut active_runs = Vec::new();
    for level in 1..world.tower.len() {
        active_runs.extend(refresh_level_runs(level, cache, &world, state)?);
    }
    Ok(active_runs)
}

/// 单个 level 内的全部 active run：先同步该 level 的派生面与游标驻留窗口，
/// 再逐 run 判脏/重估（`refresh_run_entry`）。
fn refresh_level_runs(
    level: usize,
    cache: &classifier::TowerCache,
    world: &RefreshWorld,
    state: &mut LifecycleCacheState,
) -> Result<Vec<(usize, usize)>, String> {
    sync_level_derived(level, world.tower, state.derived)?;
    let active_run_starts: Vec<_> = state.derived[&level].run_ranges.keys().copied().collect();
    state
        .derived
        .get_mut(&level)
        .expect("刚同步的 level 必须存在")
        .confirm_cursors
        .retain_run_starts(level as u32, active_run_starts);
    let stable_lower_len = cache.tower_confirmed_len(level - 1);
    let pan_freeze_boundary = cache.freeze_boundary(level - 1).unwrap_or(0);
    let run_ranges: Vec<(usize, (usize, usize))> = state.derived[&level]
        .run_ranges
        .iter()
        .map(|(&source, &range)| (source, range))
        .collect();
    let mut active_runs = Vec::with_capacity(run_ranges.len());
    for (run_source_start, run) in run_ranges {
        let ctx = RunRefreshContext {
            level,
            run_source_start,
            run,
            stable_lower_len,
            pan_freeze_boundary,
        };
        refresh_run_entry(ctx, world, state)?;
        active_runs.push((level, run_source_start));
    }
    Ok(active_runs)
}

/// 单个 run 的判脏/重估：脏则重跑 `evaluate_run` 并更新共享 `RunEntry`，否则记复用命中。
fn refresh_run_entry(
    ctx: RunRefreshContext,
    world: &RefreshWorld,
    state: &mut LifecycleCacheState,
) -> Result<(), String> {
    state.stats.provider_requests += 1;
    if !run_entry_is_dirty(&ctx, world.as_of, state) {
        state.stats.provider_reuses += 1;
        return Ok(());
    }
    reevaluate_run_entry(&ctx, world, state)?;
    state.stats.provider_reevals += 1;
    Ok(())
}

/// run 是否需要重估：条目缺失，或所属 level 的自身/lower 世代已前进，
/// 或所属 lower legs 的完成水位线跨过了条目上次评估时的 `as_of`。
fn run_entry_is_dirty(ctx: &RunRefreshContext, as_of: usize, state: &LifecycleCacheState) -> bool {
    let level_derived = &state.derived[&ctx.level];
    let entry = state.entries.get(&(ctx.level, ctx.run_source_start));
    let watermark_crossed = entry.is_some_and(|entry| {
        let before = level_derived
            .lower_ends
            .partition_point(|&end_index| end_index <= entry.last_as_of);
        let now = level_derived
            .lower_ends
            .partition_point(|&end_index| end_index <= as_of);
        now != before
    });
    entry.is_none_or(|entry| {
        entry.self_gen != level_derived.self_gen
            || entry.lower_gen != level_derived.lower_gen
            || watermark_crossed
    })
}

/// run 首次被看见时的空白 `RunEntry`——中心/类别/事件三项留给 `evaluate_run` 首次填充。
fn empty_run_entry(self_gen: u64, lower_gen: u64, as_of: usize) -> RunEntry {
    RunEntry::new(self_gen, lower_gen, as_of, Vec::new(), Vec::new(), Vec::new())
}

/// 脏 run 的实际重估：调用 `evaluate_run` 并把结果写回共享 `RunEntry`
/// （首次见到该 run 时先以当前世代新建一条空条目）。
fn reevaluate_run_entry(
    ctx: &RunRefreshContext,
    world: &RefreshWorld,
    state: &mut LifecycleCacheState,
) -> Result<(), String> {
    let level_derived = &state.derived[&ctx.level];
    let self_gen = level_derived.self_gen;
    let lower_gen = level_derived.lower_gen;
    let entry = state
        .entries
        .entry((ctx.level, ctx.run_source_start))
        .or_insert_with(|| empty_run_entry(self_gen, lower_gen, world.as_of));
    let (centers, kinds, events) = {
        let level_derived = state
            .derived
            .get_mut(&ctx.level)
            .expect("刚同步的 level 必须存在");
        let lower_legs = &level_derived.lower_legs;
        let confirm_cursors = &mut level_derived.confirm_cursors;
        evaluate_run(
            ctx.level,
            &world.tower[ctx.level],
            ctx.run,
            lower_legs,
            world.as_of,
            world.series.hist,
            world.series.dif,
            world.series.close_src,
            Some(ConfirmResidence {
                store: confirm_cursors,
                stable_lower_len: ctx.stable_lower_len,
                structure_generation: self_gen,
            }),
            Some(PanResidence {
                memo: &mut entry.pan_memo,
                freeze_boundary_src: ctx.pan_freeze_boundary,
            }),
        )
    }?;
    entry.update(self_gen, lower_gen, world.as_of, centers, kinds, events);
    Ok(())
}

/// 组装本 bar 的 provider 两相（票 #527：完成相是显式 typed 输出）。
///
/// 完成相的构造前提 = **该完成事件对应的 lower unit 已在塔上作为已完成单元存在**：
/// 用 `interval_b` 右端（= 完成 C 段终点，pan 分支 `structure.seg_c.1` 单一来源）在
/// `tower[level-1]` 上按 `end_index` 查证，取其 `ElementId` 与物理完成 bar。
/// 查不到 ⟹ 报错停线（**不**按 `kind == Consolidation` 猜完成，#523 根因之二）。
///
/// 完成钟两分（票 #559 条件 C3 订正）：`completed_at` = lower unit `end_index`（物理完成）；
/// 账本收到 bar 由 `advance` 记（= 本 bar）。**不再另设「事件首次可见」第三钟**——本函数与
/// `feed_lifecycle_bar` 在同一 bar 同一调用链内执行，该钟恒等于账本 `as_of`，
/// 分列它等于声明一个代码不具备的分辨力（090；见 `CompletionSignal` 文档 C3 订正节）。
/// 事件重发时首见性由 book 的 `completion_signals` 按桥身份唯一保证（重发不入分母）。
fn lifecycle_bar_phases(
    tower: &[Rc<Vec<LeveledMove>>],
    live_windows: &[PanLiveWindow],
    completion_events: &[NestCandidateEvent],
    as_of: usize,
) -> Result<Vec<PanProviderPhase>, String> {
    let mut phases: Vec<PanProviderPhase> = live_windows
        .iter()
        .copied()
        .map(PanProviderPhase::Live)
        .collect();
    for event in completion_events {
        if event.kind != NestDivergenceKind::Consolidation {
            continue; // trend 域不在盘整完成相范围（口径同 feed 出口的域过滤）。
        }
        let level = event.level as usize;
        let lower = tower
            .get(level.wrapping_sub(1))
            .ok_or_else(|| format!("完成事件 level={level} 无 lower 塔层（as_of={as_of}）"))?;
        let unit_index = find_move_by_end_index(lower, event.interval_b.1).ok_or_else(|| {
            format!(
                "完成事件的 lower unit 不在塔上（level={level} seg_c_end={} as_of={as_of}）：\
                 无法证明结构已完成，停线",
                event.interval_b.1
            )
        })?;
        let unit = &lower[unit_index];
        phases.push(PanProviderPhase::Completed(PanCompletionEvent {
            event: *event,
            completed_lower_id: unit.id,
            completed_at: unit.end_index,
        }));
    }
    Ok(phases)
}

/// #532：dump sink + 累计统计恒同调用点同现，收成一个上下文体。
struct LifecycleDumpSink<'a> {
    sink: &'a mut Option<BufWriter<File>>,
    stats: &'a mut LifecycleReplayStats,
}

fn feed_lifecycle_bar(
    book: &mut NestLifecycleBook,
    phases: &[PanProviderPhase],
    as_of: usize,
    series: CausalSeries,
    dump: &mut LifecycleDumpSink,
) -> Result<(), String> {
    let completion_signal_start = book.completion_signals().len();
    let audit_start = book.completion_force_unavailable_audits().len();
    let material = ForceMaterial {
        hist: Some(series.hist),
        dif: Some(series.dif),
        close_src: series.close_src,
    };
    let (delta, stats) = feed_replay_bar(book, &ReplayBarFeed { as_of, phases }, &material);

    write_feed_summary_line(dump.sink, as_of, &stats)?;
    write_completion_signal_lines(dump.sink, book, completion_signal_start)?;
    write_revision_lines(dump.sink, &delta)?;
    write_force_unavailable_lines(dump.sink, book, audit_start)?;

    dump.stats.observe(stats);
    Ok(())
}

fn write_feed_summary_line(
    sink: &mut Option<BufWriter<File>>,
    as_of: usize,
    stats: &ReplayFeedStats,
) -> Result<(), String> {
    write_lifecycle_line(
        sink,
        format_args!(
            "FEED cadence=bar as_of={as_of} live_windows={} completion_events={} completion_signals={} channel_switches={} extension_suppressed={} retrograde_rejected={} completion_force_unavailable={}",
            stats.live_windows,
            stats.completion_events,
            stats.completion_signals,
            stats.channel_switches,
            stats.extension_suppressed,
            stats.retrograde_rejected,
            stats.completion_force_unavailable,
        ),
    )
}

fn write_completion_signal_lines(
    sink: &mut Option<BufWriter<File>>,
    book: &NestLifecycleBook,
    start: usize,
) -> Result<(), String> {
    for signal in &book.completion_signals()[start..] {
        write_lifecycle_line(
            sink,
            format_args!(
                "COMPLETION_SIGNAL as_of={} level={} side={:?} kind={:?} seg_a={:?} seg_c_full={:?} b_center_start={} lower_id={:?} completed_at={}",
                signal.as_of,
                signal.key.level,
                signal.key.side,
                signal.key.kind,
                signal.key.seg_a,
                signal.key.seg_c_full,
                signal.key.b_center_start,
                (signal.completed_lower_id.level, signal.completed_lower_id.ordinal),
                signal.completed_at,
            ),
        )?;
    }
    Ok(())
}

fn write_revision_lines(
    sink: &mut Option<BufWriter<File>>,
    delta: &[LifecycleRevision],
) -> Result<(), String> {
    for revision in delta {
        write_lifecycle_line(
            sink,
            format_args!(
                "REV as_of={} level={} side={:?} kind={:?} seg_a={:?} seg_c_full={:?} b_center_start={} revision={:?} evidence={:?}",
                revision.as_of,
                revision.key.level,
                revision.key.side,
                revision.key.kind,
                revision.key.seg_a,
                revision.key.seg_c_full,
                revision.key.b_center_start,
                revision.kind,
                revision.evidence,
            ),
        )?;
    }
    Ok(())
}

fn write_force_unavailable_lines(
    sink: &mut Option<BufWriter<File>>,
    book: &NestLifecycleBook,
    start: usize,
) -> Result<(), String> {
    for audit in &book.completion_force_unavailable_audits()[start..] {
        write_lifecycle_line(
            sink,
            format_args!(
                "COMPLETION_FORCE_UNAVAILABLE as_of={} level={} side={:?} kind={:?} seg_a={:?} seg_c_full={:?} b_center_start={} reason={:?}",
                audit.as_of,
                audit.key.level,
                audit.key.side,
                audit.key.kind,
                audit.key.seg_a,
                audit.key.seg_c_full,
                audit.key.b_center_start,
                audit.reason,
            ),
        )?;
    }
    Ok(())
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
/// run 投影 → decompose → assemble_level_view → provide_nest_candidate_events。
/// 返回的中心、类别与完整事件只存进共享 RunEntry；targeted 应用侧再与 pending 求交。
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
    confirm_residence: Option<ConfirmResidence<'_>>,
    pan_residence: Option<PanResidence<'_>>,
) -> Result<(Vec<Center>, Vec<Option<MoveKind>>, Vec<NestCandidateEvent>), String> {
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
    let view = assemble_level_view_resident(
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
        confirm_residence,
    )
    .map_err(|error| format!("L{level} targeted C2 assemble 失败: {error:?}"))?;
    let kinds = decompose::center_block_kind(centers.len(), &blocks);
    let events = provide_nest_candidate_events_resident(
        level as u32,
        &projection,
        &blocks,
        lower,
        &view,
        hist,
        dif,
        close_src,
        pan_residence,
    );
    Ok((centers, kinds, events))
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
                .map(|id| {
                    format!(
                        "{}:{}:{}-{}",
                        id.level, id.turn_source, id.interval_b.0, id.interval_b.1
                    )
                })
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

/// #218 面 B 研究 bin 锚供给说明（诚实，090）：owner 判同已换两族锚（一/三类核心区间
/// 带判同经账本 `centers` 全功能；二类一类点身份锚判同需 T1 oracle + 事件锚账本）。
/// 本 bin 是归档研究/审计工具，未接事件锚账本——二类判同锚不可解 = 诚实判负（与
/// 生产 gate 全接线读数有别，面 B 注册项；一/三类判同不受影响）。
fn bin_anchor_ctx() -> newchan_rust::theta_v0::classifier::nest::OwnerAnchorCtx<'static> {
    fn never(_: usize) -> Option<(newchan_rust::theta_v0::types::Tick, usize)> {
        None
    }
    newchan_rust::theta_v0::classifier::nest::OwnerAnchorCtx {
        anchor_at: &never,
        event_anchor: (None, None),
    }
}

const TERMINAL_MATCH: TerminalMatch = TerminalMatch::CWindow;

/// 终端背书查法（生产）：委托 lib 单一来源 `nest::terminal_bits_at_event`——账本级别移位
/// `levels[ℓ-1].bsp` + 口径常数 [`TERMINAL_MATCH`]。与 p116 同名函数逐字同口径。
fn terminal_bits_new(
    classification: &classifier::Classification,
    event: &NestCandidateEvent,
) -> Option<BspBits> {
    // 关③ P3：lib 返回形状扩为 `TerminalEndorsement`（bits + owner start_index）——
    // 生产装配消费 bits 层；owner 两维构成归收紧后重放审计读数，非本 bin 职责。
    terminal_bits_at_event(classification, event, TERMINAL_MATCH, &bin_anchor_ctx()).map(|t| t.bits)
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
    let book_level = classification
        .levels
        .get(event_bsp_book_level(level as u32)?)?;
    let book = &book_level.bsp;
    // 关③ P3 平移：旧事件 = Cand^δ 趋势族线（pan_div_diag 为 cand_delta=false 纯诊断，
    // 结构性不入终端查询）⟹ kind=Trend；B 身份 = 事件自带 `b_parent.source_interval.0`
    //（ParentCenterIdentity 已携 B start_index 快照，单一来源，无第二查法）。
    // #218 面 B：一/三类判同的 B 带由同层 `centers` 查出（b_center_start 只当查找键）。
    terminal_bits_in_book(
        book,
        &book_level.centers,
        c_start,
        source,
        side,
        NestDivergenceKind::Trend,
        b_center_start,
        TERMINAL_MATCH,
        &bin_anchor_ctx(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// 逃生门身份键不含活窗右端：相邻 bar 只延展 `seg_c_live.1`，不得另造身份。
    /// `gap_len`（票 #592）是产窗时刻冻结的诊断快照，随身份一起延展，不因右端前进而重算。
    #[test]
    fn lifecycle_window_stem_keeps_identity_while_extending_bar() {
        let stem = LifecycleWindowStem {
            level: 2,
            side_tag: 0,
            seg_a: (10, 19),
            c_start: 30,
            b_center_start: 20,
            gap_len: 7,
        };
        let first = stem.window_at(30);
        let later = stem.window_at(99);

        assert_eq!(LifecycleWindowStem::of(&first), stem);
        assert_eq!(LifecycleWindowStem::of(&later), stem);
        assert_eq!(first.seg_c_live, (30, 30));
        assert_eq!(later.seg_c_live, (30, 99));
        assert_eq!(first.gap_len, 7, "gap_len 随身份延展原样带出，不因右端前进重算");
        assert_eq!(later.gap_len, 7);
    }

    /// ★#613（收 #609 F1）：L2 的 confirmed 侧必须按**整窗**（塔的确认水线）截断，
    /// 「同起点 pop 1 个」只修对一半。
    ///
    /// 场景取自实测形态：`tower[1]` 末尾 3 个 L1 单元来自同一个**仍开放**的窗口
    /// （`WindowScanCursor::last_window_emitted == 3`，#148 升级重切窗一窗产 ⌊n/3⌋ 个子中枢；
    /// BTC 100k 上该游标 >1 占 20.0%）⟹ `tower_confirmed_len(1) == 3`。行进中 L1 单元由该
    /// 窗口起点重扫派生 ⟹ 与**首个**残留子单元同起点，而不是与末项同起点。
    ///
    /// 三分支各自钉一条：不截断 = 重叠恒判 `frontier_not_after_confirmed`；单 pop = 仍重叠
    /// （残留 2 个）；整窗截断 = 重叠消除，判据推进到下一条（此处 centers 为空 ⟹
    /// `no_confirmed_center_before`）。
    #[test]
    fn l2_confirmed_side_truncates_by_whole_window_watermark() {
        use newchan_rust::theta_v0::classifier::nest_lifecycle::PanLiveOutcome;

        // 6 个 L1 单元，共端点（生产 lower_legs 约定）。末 3 个（下标 3/4/5）= 开放末窗产物。
        let leg = |si: usize, ei: usize, dir: Direction, sp: i64, ep: i64| Segment {
            direction: dir,
            start_index: si,
            end_index: ei,
            start_price: sp,
            end_price: ep,
        };
        let legs = vec![
            leg(0, 10, Direction::Up, 100, 150),
            leg(10, 20, Direction::Down, 150, 120),
            leg(20, 30, Direction::Up, 120, 148),
            leg(30, 40, Direction::Down, 148, 110), // ← 开放末窗第 1 个子单元
            leg(40, 50, Direction::Up, 110, 145),   // ← 第 2 个
            leg(50, 60, Direction::Down, 145, 115), // ← 第 3 个
        ];
        // 塔的确认水线 = len - last_window_emitted = 6 - 3。
        let l1_confirmed_len = 3usize;
        // 行进中 L1 单元 = 该开放窗口「计入行进中 L0 段」后的形态 ⟹ 与 legs[3] 同起点。
        let active = leg(30, 70, Direction::Down, 148, 105);
        let centers: Vec<Center> = Vec::new();
        let kinds: Vec<Option<MoveKind>> = Vec::new();

        // (a) 不截断：末窗以 confirmed + active 双重身份进入 ⟹ 重叠恒真。
        assert_eq!(
            provide_active_pan_live_windows(2, &centers, &kinds, &legs, active, 70),
            PanLiveOutcome::FrontierNotAfterConfirmed,
            "不截断 ⟹ active.start(30) < confirmed.last().end(60)"
        );

        // (b) 旧修法「同起点 pop 1 个」：末项 legs[5] 起点 50 ≠ active 起点 30 ⟹ 根本不触发 pop；
        //     即便强行摘掉末项，仍残留 legs[3]/legs[4] ⟹ 重叠未消除。这就是「只修对一半」。
        let single_pop = &legs[..legs.len() - 1];
        assert_eq!(
            single_pop.last().map(|last| last.start_index),
            Some(40),
            "同起点判据在 last_window_emitted>1 时不成立（40 != 30），单 pop 甚至不触发"
        );
        assert_eq!(
            provide_active_pan_live_windows(2, &centers, &kinds, single_pop, active, 70),
            PanLiveOutcome::FrontierNotAfterConfirmed,
            "单 pop 后仍有 2 个未确认子单元残留 ⟹ 重叠仍在"
        );

        // (c) 整窗截断到水线：confirmed 侧只剩真确认单元 ⟹ 重叠消除，判据推进到下一条。
        let mut whole_window = legs.clone();
        whole_window.truncate(l1_confirmed_len);
        assert_eq!(
            whole_window.last().map(|last| last.end_index),
            Some(30),
            "截断后末确认单元终点(30) <= active 起点(30) ⟹ 不重叠"
        );
        assert_eq!(
            provide_active_pan_live_windows(2, &centers, &kinds, &whole_window, active, 70),
            PanLiveOutcome::NoConfirmedCenterBefore,
            "★F1：重叠消除后不再落 frontier_not_after_confirmed，判据按序推进"
        );
    }

    /// ★#602：L3 的**扫描输入**必须按 `tower[1]` 的确认水线整窗截断。
    ///
    /// L2 与 L3 在这一点上**结构不同**（不是可省的对称补丁）：
    /// - L2 的扫描输入 `l0_units` 是 parser **已 emit** 段的投影，行进中段在 `tail` 里、
    ///   不在输入中 ⟹ 输入与虚拟单元天然不重叠；
    /// - L3 的扫描输入 = `tower[1]` 的投影，**含塔的开放末窗单元**，而虚拟单元（行进中 L1
    ///   窗口）正是那批开放单元「计入行进中 L0 段」后的形态 ⟹ 二者同起点，不截断即倒灌。
    ///
    /// 三分支各钉一条：不截断 = 恒判 `lower_frontier_not_after_units`（BTC 100k 探针实测
    /// 728 次）；截到水线 = 倒灌消除、判据按序推进；水线取自 `tower_confirmed_len(1)`，与
    /// L3 confirmed 段侧用的**同一条**水线证书（#93 单一来源）。
    #[test]
    fn l3_scan_input_truncates_by_tower1_whole_window_watermark() {
        use newchan_rust::theta_v0::classifier::center::UnitRange as U;
        use newchan_rust::theta_v0::classifier::nest_lifecycle::{
            active_l2_window_frontier, ActiveWindowFrontier, ActiveWindowOutcome,
        };

        // 5 个 L1 单元（= `tower[1]` 投影，共端点）。末 2 个来自同一个**仍开放**的窗口
        // ⟹ `tower_confirmed_len(1) == 3`。
        let unit = |si: usize, ei: usize, lo: i64, hi: i64| U {
            start_index: si,
            end_index: ei,
            direction: Direction::Up,
            lo,
            hi,
        };
        let l1_units = vec![
            unit(0, 10, 100, 120),
            unit(10, 20, 105, 120),
            unit(20, 30, 105, 125),
            unit(30, 40, 108, 122), // ← 开放末窗第 1 个子单元
            unit(40, 50, 106, 124), // ← 第 2 个
        ];
        let leg_dirs = vec![Direction::Down; l1_units.len()];
        let l1_confirmed_len = 3usize;
        // 行进中 L1 单元 = 该开放窗口「计入行进中 L0 段」后的形态 ⟹ 与 l1_units[3] 同起点。
        let active = ActiveWindowFrontier {
            direction: Direction::Down,
            start_index: 30,
            end_index: 60,
            lo: 110,
            hi: 118,
        };

        // (a) 不截断：开放末窗以 confirmed + active 双重身份进入 ⟹ 倒灌恒真。
        assert_eq!(
            active_l2_window_frontier(&l1_units, &leg_dirs, Some(&active), 0),
            ActiveWindowOutcome::LowerFrontierNotAfterUnits,
            "不截断 ⟹ active.start(30) < units.last().end(50)"
        );

        // (b) 截到水线：输入只剩真确认单元 ⟹ 倒灌消除，虚拟单元被末窗吸收。
        let scan_units = &l1_units[..l1_confirmed_len];
        assert_eq!(
            scan_units.last().map(|last| last.end_index),
            Some(30),
            "截断后末确认单元终点(30) <= active 起点(30) ⟹ 不倒灌"
        );
        let outcome = active_l2_window_frontier(
            scan_units,
            &leg_dirs[..l1_confirmed_len],
            Some(&active),
            0,
        );
        let frontier = outcome
            .frontier()
            .expect("★倒灌消除后判据按序推进，行进中 L1 单元被 L2 层末窗吸收");
        assert_eq!(
            (frontier.start_index, frontier.end_index),
            (0, 60),
            "行进中 L2 单元右端落在行进中 L1 单元终点——塔上不存在的形态"
        );
        assert_eq!(frontier.direction, Direction::Down, "首叶方向表口径");
    }

    /// ★#602：L3 的 **confirmed 段侧**同样按整窗水线截断（`tower_confirmed_len(2)`），
    /// 与 #613 在 L2 上的口径逐字同构——#601 §7 遗留 1 点名的「tower[2] 侧开放末窗截断未做」。
    ///
    /// provider 本身级别中立，故本测试钉的是**该级别水线口径生效后判据按序推进**：不截断 ⟹
    /// 恒判 `frontier_not_after_confirmed`；截到水线 ⟹ 不再落该码。
    #[test]
    fn l3_confirmed_side_truncates_by_tower2_whole_window_watermark() {
        use newchan_rust::theta_v0::classifier::nest_lifecycle::PanLiveOutcome;

        let leg = |si: usize, ei: usize, dir: Direction, sp: i64, ep: i64| Segment {
            direction: dir,
            start_index: si,
            end_index: ei,
            start_price: sp,
            end_price: ep,
        };
        // 6 个 L2 单元（= `lower_legs_from(tower[2])`）。末 3 个 = 开放末窗产物 ⟹ 水线 = 3。
        let legs = vec![
            leg(0, 10, Direction::Up, 100, 150),
            leg(10, 20, Direction::Down, 150, 120),
            leg(20, 30, Direction::Up, 120, 148),
            leg(30, 40, Direction::Down, 148, 110),
            leg(40, 50, Direction::Up, 110, 145),
            leg(50, 60, Direction::Down, 145, 115),
        ];
        let l2_confirmed_len = 3usize;
        let active = leg(30, 70, Direction::Down, 148, 105);
        let centers: Vec<Center> = Vec::new();
        let kinds: Vec<Option<MoveKind>> = Vec::new();

        assert_eq!(
            provide_active_pan_live_windows(3, &centers, &kinds, &legs, active, 70),
            PanLiveOutcome::FrontierNotAfterConfirmed,
            "不截断 ⟹ active.start(30) < confirmed.last().end(60)"
        );
        let mut whole_window = legs.clone();
        whole_window.truncate(l2_confirmed_len);
        assert_eq!(
            provide_active_pan_live_windows(3, &centers, &kinds, &whole_window, active, 70),
            PanLiveOutcome::NoConfirmedCenterBefore,
            "★整窗截断后不再落 frontier_not_after_confirmed，判据按序推进"
        );
    }

    /// #553 测试夹具：候选事件（字段全显式，行格式逐字断言的唯一输入源）。
    fn sample_event(turn_source: usize, divergence_confirmed: bool) -> NestCandidateEvent {
        NestCandidateEvent {
            level: 2,
            side: Side::Short,
            kind: NestDivergenceKind::Consolidation,
            seg_a: (10, 19),
            interval_b: (20, 39),
            interval_a: (40, 59),
            divergence_confirmed,
            turn_source,
            judge_at: 777,
            provider_window: (5, 70),
            intake_fallback: false,
            b_center_start: 20,
        }
    }

    /// #553 门控：env 未设（writer=None）⟹ 观察候选事件流零落盘、零副作用可见面。
    #[test]
    fn event_dump_disabled_writes_nothing() {
        let mut dump = EventDump::new(None);
        dump.observe(&sample_event(100, true), 123, true).unwrap();
        dump.observe(&sample_event(101, false), 124, false).unwrap();
        assert!(dump.writer.is_none());
        assert_eq!(dump.seq, 0);
        assert!(dump.revisions.is_empty());
    }

    /// #553 行格式 + 修订号：同 EventKey 复现 ⟹ rev 递增；行序号 seq 全局单调。
    #[test]
    fn event_dump_line_format_and_revision_monotonic() {
        let sink: Rc<RefCell<Vec<u8>>> = Rc::new(RefCell::new(Vec::new()));
        let mut dump = EventDump::new(Some(Box::new(SharedSink(Rc::clone(&sink)))));
        dump.observe(&sample_event(100, false), 123, true).unwrap();
        dump.observe(&sample_event(100, true), 130, true).unwrap();
        dump.observe(&sample_event(200, true), 130, false).unwrap();

        let text = String::from_utf8(sink.borrow().clone()).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(
            lines[0],
            "EVENT seq=0 rev=1 as_of=123 level=2 side=Short kind=pan div=0 pending=1 \
turn_source=100 judge_at=777 seg_a=10..19 interval_b=20..39 interval_a=40..59 \
provider_window=5..70 b_center_start=20 intake_fallback=0"
        );
        // 同 key（div 不进 EventKey）第二次观察 ⟹ rev=2，且 div/时钟按当次实况落盘。
        assert_eq!(
            lines[1],
            "EVENT seq=1 rev=2 as_of=130 level=2 side=Short kind=pan div=1 pending=1 \
turn_source=100 judge_at=777 seg_a=10..19 interval_b=20..39 interval_a=40..59 \
provider_window=5..70 b_center_start=20 intake_fallback=0"
        );
        // 不同 key（turn_source 进 EventKey）⟹ rev 重新从 1 起；seq 继续单调。
        assert_eq!(
            lines[2],
            "EVENT seq=2 rev=1 as_of=130 level=2 side=Short kind=pan div=1 pending=0 \
turn_source=200 judge_at=777 seg_a=10..19 interval_b=20..39 interval_a=40..59 \
provider_window=5..70 b_center_start=20 intake_fallback=0"
        );
    }

    /// #641 门控：env 未设（writer=None）⟹ 链簿不建、不推进、零落盘、零副作用可见面。
    #[test]
    fn chain_dump_disabled_writes_nothing_and_never_advances_the_book() {
        let cache = classifier::TowerCache::new();
        let mut dump = ChainDump::new(None, 1);
        dump.observe(&cache, 0, false).unwrap();
        dump.observe(&cache, 1, true).unwrap();
        assert!(dump.writer.is_none());
        assert_eq!(dump.seq, 0);
        assert!(
            dump.book.certificates().is_empty(),
            "关灯路径不得推进链簿（零行为差异）"
        );
    }

    /// #641 节拍：非末根只在 `(as_of+1) % every == 0` 时推进；末根无条件推进。
    ///
    /// **锁定手段照实（#676 LOW-1 实修）**：断言的是 [`ChainDump::advanced_at`] 探针记下的
    /// **真推进 `as_of` 序列本身**（探针记录点在 `self.book.advance` 真正调用之后，见证「推进
    /// 执行」而非「越过节拍门」——#691 LOW-1 把记录点从 `advance` 之前挪到之后），不是它的可观察
    /// 后果——空事件流上 Delta 恒空，行数/`seq` 在「节拍正确 / 永不推进 / 每根都推进」三种实现下
    /// 同形为 0，靠它们判不出节拍。
    ///
    /// **两组独立 `every` 夹具（#691 LOW-2 实修）**：单一 `every=3` 只锁住节拍*形状*，锁不住
    /// 「节拍由 `every` 参数真驱动」这件事——把 `self.every` 硬编码成字面量 `3` 时，`every=3` 那组
    /// 照样全绿；加一组独立算出期望值的 `every=2` 夹具，硬编码注入 ⟹ `every=2` 那组变红
    /// （实测：left=`[2, 5, 6]`（硬编码值算出）right=`[1, 3, 5, 6]`（真参数值算出））。
    ///
    /// 反事实负控（#676/#691 实做记录，非推想）：
    /// - 把节拍门改成「永不推进」（`observe` 在门后直接 `return Ok(())`）⟹ 两组探针序列都退化为
    ///   只剩末根 `[6]`，各自的断言变红；
    /// - 改成「每根都推进」（删掉 `!is_last && (as_of + 1) % self.every != 0` 这道门）⟹ 两组探针
    ///   序列都变成 `[0, 1, 2, 3, 4, 5, 6]`，各自的断言变红；
    /// - 把 `self.every` 硬编码成 `3`（LOW-2 病态实现）⟹ `every=2` 组变红（`every=3` 组仍绿，
    ///   正是「单一夹具锁不住参数依赖」的反面证据）。
    #[test]
    fn chain_dump_cadence_advances_on_beat_and_on_last_bar() {
        let advanced_at_for = |every: usize| -> Vec<usize> {
            let cache = classifier::TowerCache::new();
            let sink: Rc<RefCell<Vec<u8>>> = Rc::new(RefCell::new(Vec::new()));
            let mut dump = ChainDump::new(Some(Box::new(SharedSink(Rc::clone(&sink)))), every);
            for as_of in 0..7 {
                dump.observe(&cache, as_of, as_of == 6).unwrap();
            }
            dump.flush().unwrap();
            assert!(String::from_utf8(sink.borrow().clone()).unwrap().is_empty());
            assert_eq!(
                dump.seq, 0,
                "空事件流 ⟹ 零 Delta ⟹ 零行序推进（every={every}）"
            );
            dump.advanced_at
        };

        // every=3、bars=0..=6：节拍根 = `(as_of+1)%3==0` 的 2 与 5；末根 6 无条件推进
        // （`(6+1)%3 != 0`，故它只可能来自末根支）。三个数各锁一件事：
        // 少了 2/5 ⟹ 节拍支没走；少了 6 ⟹ 末根支没走；多出 0/1/3/4 ⟹ 门没起作用。
        assert_eq!(
            advanced_at_for(3),
            vec![2, 5, 6],
            "every=3：节拍门必须恰好在节拍根与末根放行（每根都推进 / 永不推进 / 相位错一位都在此变红）"
        );
        // every=2、bars=0..=6：节拍根 = `(as_of+1)%2==0` 的 1/3/5；末根 6 无条件推进
        // （`(6+1)%2 != 0`，同样只可能来自末根支）。期望值与 every=3 组不同，证明节拍确由
        // `every` 参数决定——硬编码 `self.every=3` 的病态实现会让本组变红。
        assert_eq!(
            advanced_at_for(2),
            vec![1, 3, 5, 6],
            "every=2：节拍门必须恰好在节拍根与末根放行（硬编码 every=3 的病态实现在此变红）"
        );
    }

    fn cadence_probe_seg(dir: Direction, si: usize, ei: usize, sp: i64, ep: i64) -> Segment {
        Segment {
            direction: dir,
            start_index: si,
            end_index: ei,
            start_price: sp,
            end_price: ep,
        }
    }

    /// 候选富集段序列（同 `classifier::mod::tests::candidate_rich_segments` 口径，本 bin 内独立
    /// 一份——那份是 classifier 模块内 `#[cfg(test)]` 私有项，跨 crate 不可见）。
    fn cadence_probe_candidate_rich_segments(count: usize) -> Vec<Segment> {
        let mut price = 100_i64;
        let mut state = 0x9e3779b97f4a7c15_u64;
        (0..count)
            .map(|i| {
                state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let swing = 20 + ((state >> 32) % 90) as i64;
                let dir = if i % 2 == 0 {
                    Direction::Up
                } else {
                    Direction::Down
                };
                let start = price;
                price += match dir {
                    Direction::Up => swing,
                    Direction::Down => -swing,
                };
                cadence_probe_seg(dir, i * 4, i * 4 + 4, start, price)
            })
            .collect()
    }

    fn cadence_probe_bars_from_closes(vals: &[i64]) -> Vec<Bar> {
        vals.iter()
            .enumerate()
            .map(|(i, &v)| Bar {
                source_index: i,
                timestamp: i as i64,
                open: v,
                high: v,
                low: v,
                close: v,
                volume: 1,
                untradable: false,
            })
            .collect()
    }

    /// #691 LOW-1 补强：探针改口径（记录点移到 `advance` 之后）本身仍可能被「删掉
    /// `book.advance` 整条、留一个类型匹配的空 `Vec` 占位」这种病态实现绕过——单纯挪动语句顺序
    /// 的探针只见证「程序走到了这一行」，不见证「这一行真的调用了 `advance` 并产生效果」（空事件流
    /// 上 `advance` 恒为 no-op，二者在 `advanced_at` 上完全同形，参见上一用例文档）。
    ///
    /// 本用例改用**真实非空候选流**堵死这个漏洞：真调用 `advance` 时，首个越过节拍门的 `as_of`
    /// 会在簿里留下真实证书（`seq`/`book.certificates()` 非零、Delta 行落盘非空）；病态实现把
    /// `book.advance` 换成空 `Vec` 字面量后，无论探针记在哪一行，这三项永远锁定在零/空。
    ///
    /// 反事实负控（#691 实做记录，非推想）：把 `observe` 里
    /// `let delta = self.book.advance(&cache.candidate_streams(), as_of);` 整条替换为
    /// `let delta: Vec<_> = Vec::new();`（保留节拍门与两条 `#[cfg(test)]` 探针语句不动）⟹
    /// 本用例 `dump.seq > 0` 断言变红（left=`0` right>`0`）。
    #[test]
    fn chain_dump_observe_advance_produces_real_certificates_on_first_beat() {
        let cfg = ThetaConfig::default();
        let segments = cadence_probe_candidate_rich_segments(120);
        let closes: Vec<i64> = (0..=segments.last().unwrap().end_index)
            .map(|i| 100 + if i % 8 < 4 { 35 } else { -35 } + (i as i64 / 16))
            .collect();
        let layer = ParseLayer {
            segments: Rc::new(segments),
            merged_bars: Rc::new(cadence_probe_bars_from_closes(&closes)),
            ..Default::default()
        };
        let mut cache = classifier::TowerCache::new();
        classifier::classify_with_tower_events_incremental(&layer, &cfg, &mut cache);
        assert!(
            cache
                .candidate_streams()
                .iter()
                .any(|stream| !stream.is_empty()),
            "夹具必须产出非空候选流，否则测不到 advance 的可观察后果"
        );

        let sink: Rc<RefCell<Vec<u8>>> = Rc::new(RefCell::new(Vec::new()));
        let mut dump = ChainDump::new(Some(Box::new(SharedSink(Rc::clone(&sink)))), 3);
        dump.observe(&cache, 0, false).unwrap();
        assert_eq!(dump.seq, 0, "as_of=0 未越过 every=3 的节拍门，不该推进");
        dump.observe(&cache, 2, false).unwrap(); // (2+1)%3==0：首个节拍根
        dump.flush().unwrap();

        assert!(
            dump.seq > 0,
            "真实候选流上首个节拍根必须产出非空 Delta（真推进的可观察后果）；\
             为 0 即 advance 未被真正调用（病态实现漏网，见本函数文档反事实负控）"
        );
        assert!(
            !dump.book.certificates().is_empty(),
            "真实候选流上首个节拍根必须在簿中留下证书"
        );
        assert!(
            !String::from_utf8(sink.borrow().clone()).unwrap().is_empty(),
            "非空 Delta 必须落盘为 CHAIN 行"
        );
    }

    /// #641 元行 + 行格式逐字锁（行口径漂移当场变红）。
    #[test]
    fn chain_dump_meta_and_line_format_are_stable() {
        use newchan_rust::theta_v0::classifier::cand_event::{
            CandidateEventBook, CandidateKey, CandidateKind, CandidateObservation, CandidateState,
            ParentFingerprint, StructuralPredicates, CANDIDATE_RULE_VERSION,
        };
        let observation =
            |level: u32, c_start: usize, interval: (usize, usize)| CandidateObservation {
                key: CandidateKey {
                    rule_version: CANDIDATE_RULE_VERSION,
                    level,
                    kind: CandidateKind::Trend,
                    side: Side::Long,
                    previous_center_start: Some(10),
                    parent: ParentFingerprint {
                        center_start: 20,
                        zd: 100,
                        zg: 110,
                    },
                    seg_a: (11, 19),
                    c_start,
                },
                kind: CandidateKind::Trend,
                center_ids: Some((10, 20)),
                candidate_group_id: 1,
                pair_id: 2,
                structural_predicates: StructuralPredicates {
                    direction: true,
                    comparable: true,
                    extreme: true,
                },
                extreme_proof: (11, 19),
                third_class_proof: None,
                interval,
                state: CandidateState::Provisional,
                first_provable_at: Some(interval.1),
                confirmed_at: None,
            };
        // L2 ⊇ L0，L1 域无候选 ⟹ 一条 skip 边的两节点链。
        let mut candidates = CandidateEventBook::default();
        candidates.advance(
            &[observation(2, 0, (0, 100)), observation(0, 20, (20, 40))],
            100,
        );
        let mut chains = classifier::chain_cert::ChainCertificateBook::default();
        let delta = chains.advance(&candidates.streams(), 100);
        assert_eq!(delta.len(), 1);

        let sink: Rc<RefCell<Vec<u8>>> = Rc::new(RefCell::new(Vec::new()));
        let mut dump = ChainDump::new(Some(Box::new(SharedSink(Rc::clone(&sink)))), 7);
        dump.write_meta(500).unwrap();
        let line = chain_dump_line(&delta[0], dump.seq, 100);
        writeln!(dump.writer.as_mut().unwrap(), "{line}").unwrap();
        dump.flush().unwrap();

        let text = String::from_utf8(sink.borrow().clone()).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "CHAIN_META every=7 bars=500");
        assert_eq!(
            lines[1],
            "CHAIN seq=0 as_of=100 rev=0 status=Open root_level=2 leaf_level=0 nodes=2 \
alive=2 falsified=0 absent=0 edges=1 adjacent=0 skip=1 fact=0 crossed=0 extendable=0 \
extends=0 observed_at=100 closed_at=- invalidated_at=- path=2:0;0:20"
        );
    }

    /// 测试用共享 sink：把 dump 落盘面引到内存，验证行内容逐字。
    struct SharedSink(Rc<RefCell<Vec<u8>>>);

    impl Write for SharedSink {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// 测试用故障 sink：`write`/`flush` 按开关返回 `Err`，纯内存构造（不碰文件系统/进程 env），
    /// 用于锁定 [`EventDump::observe`]/[`EventDump::flush`] 写失败经 `?` 上抛的传播边界（#553）。
    /// 不包 `BufWriter`——`BufWriter` 会缓冲写入，写失败要等到缓冲区满/显式 flush 才暴露，
    /// 直接把它作为 `Box<dyn Write>` 传给 `EventDump::new` 才能在 `observe` 单次调用内测到。
    struct FailingSink {
        fail_on_write: bool,
        fail_on_flush: bool,
    }

    impl Write for FailingSink {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            if self.fail_on_write {
                Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "boom"))
            } else {
                Ok(buf.len())
            }
        }

        fn flush(&mut self) -> std::io::Result<()> {
            if self.fail_on_flush {
                Err(std::io::Error::new(std::io::ErrorKind::Other, "boom"))
            } else {
                Ok(())
            }
        }
    }

    /// #553 边界：`observe` 的写失败经调用点 `?` 上抛（模块头「边界」段落），错误消息含
    /// `EVENT_DUMP_ENV` 常量渲染的字面量 `"P123_EVENT_DUMP"`——锁定常量确实被插值，不是占位符。
    #[test]
    fn event_dump_observe_propagates_write_error() {
        let sink = FailingSink {
            fail_on_write: true,
            fail_on_flush: false,
        };
        let mut dump = EventDump::new(Some(Box::new(sink) as Box<dyn Write>));
        let err = dump
            .observe(&sample_event(100, true), 123, true)
            .expect_err("write 失败必须经 ? 上抛为 Err，不得被吞掉");
        assert!(err.contains("写 P123_EVENT_DUMP 失败"), "{err}");
    }

    /// #553 边界：`flush` 的写失败同样经 `?` 上抛，错误消息含同一字面量渲染的 `"刷新 …失败"` 前缀。
    #[test]
    fn event_dump_flush_propagates_flush_error() {
        let sink = FailingSink {
            fail_on_write: false,
            fail_on_flush: true,
        };
        let mut dump = EventDump::new(Some(Box::new(sink) as Box<dyn Write>));
        let err = dump
            .flush()
            .expect_err("flush 失败必须经 ? 上抛为 Err，不得被吞掉");
        assert!(err.contains("刷新 P123_EVENT_DUMP 失败"), "{err}");
    }

    /// #69 5b / T5：dirty 更新只替换代次/事件载荷，run-local pan memo 的持有地址不变。
    #[test]
    fn run_entry_update_preserves_pan_memo_residence() {
        let mut entry = RunEntry::new(1, 2, 3, Vec::new(), Vec::new(), Vec::new());
        let memo_address = std::ptr::addr_of!(entry.pan_memo);
        entry.update(4, 5, 6, Vec::new(), Vec::new(), Vec::new());
        assert_eq!(entry.self_gen, 4);
        assert_eq!(entry.lower_gen, 5);
        assert_eq!(entry.last_as_of, 6);
        assert_eq!(std::ptr::addr_of!(entry.pan_memo), memo_address);
    }
}
