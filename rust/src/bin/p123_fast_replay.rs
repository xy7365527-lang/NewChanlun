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
    active_segment_frontier, feed_replay_bar, provide_l1_active_pan_live_windows,
    ActiveSegmentFrontier, ForceMaterial, LifecycleSettlementStats, NestLifecycleBook,
    PanCompletionEvent, PanLiveWindow, PanProviderPhase, ReplayBarFeed, ReplayFeedStats,
};
use newchan_rust::theta_v0::classifier::recursive_tower::{find_move_by_end_index, LeveledMove};
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
/// [`run_targeted_prefix_pass`]），会**中止 prefix pass**；此时收尾的既有
/// `P421_LIFECYCLE_DUMP` sidecar 收尾 flush（先执行）与随后的 [`EventDump::flush`] 一并
/// 被跳过（函数提前返回，两条收尾语句均不可达）。**上抛不止于此**：该 `Err` 继续经
/// `main` 内 [`run_targeted_prefix_pass`] 调用点的 `?` 逃出 `main`，`main` 尾部的
/// [`dump_flush`] 调用**同样不可达**；而 [`DUMP`] 是 `static OnceLock<..BufWriter..>`，
/// Rust 的 `static` **不执行 `Drop`** ⟹ 既有 `P116_DUMP` 的缓冲尾字节**静默丢失**
///（受损面 = 既有封印文件；本结构自己的 `BufWriter` 是局部变量、drop 时会冲刷）。
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
}

impl ChainDump {
    fn new(writer: Option<Box<dyn Write>>, every: usize) -> Self {
        Self {
            writer,
            book: classifier::chain_cert::ChainCertificateBook::default(),
            every,
            seq: 0,
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
    /// #527：L1 活窗定位结果分布（`L1LiveOutcome::reason_tag`；诊断只写不判）。
    /// 「某完成身份为何没有更早 Live」的可审计落点。
    l1_live_outcomes: BTreeMap<&'static str, usize>,
    /// 末 prefix 的终局分布与寿命读面（闪现/非闪现分层）。
    settlement: LifecycleSettlementStats,
}

/// #421 逃生门的活窗结构分量；与 p409 `WindowStem` 同键，右端不进身份。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct LifecycleWindowStem {
    level: u32,
    side_tag: u8,
    seg_a: (usize, usize),
    c_start: usize,
    b_center_start: usize,
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
            seg_c_live: (self.c_start, as_of.max(self.c_start)),
            b_center_start: self.b_center_start,
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
    let terminal_events = collect_snapshot_candidates(
        &terminal.tower,
        max_bars - 1,
        hist,
        dif,
        close_src,
        &mut audit,
    )?;
    let mut targets = BTreeMap::new();
    for event in terminal_events.iter().flatten() {
        targets.insert(EventKey::from(event), *event);
    }
    let prefix_started = Instant::now();
    let (mut book, prefix_views, unresolved_targets, stats, lifecycle_stats) =
        run_targeted_prefix_pass(&loaded.bars[..max_bars], &config, &targets)?;
    let prefix_elapsed = prefix_started.elapsed();
    audit.views += prefix_views;
    audit.snapshots += book.candidates.len();
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
        "P527_L1_LIVE_OUTCOMES {}",
        lifecycle_stats
            .l1_live_outcomes
            .iter()
            .map(|(tag, count)| format!("{tag}={count}"))
            .collect::<Vec<_>>()
            .join(" ")
    );
    let settlement = lifecycle_stats.settlement;
    eprintln!(
        "P421_LIFETIME_SUMMARY entries={} first_provable={} provisional={} confirmed={} force_overtake={} never_constituted={} identity_vanished={} flash_terminal={} nonflash_count={} nonflash_min={:?} nonflash_median={:?} nonflash_max={:?} force_lifetime_count={} force_lifetime_min={:?} force_lifetime_median={:?} force_lifetime_max={:?}",
        settlement.entry_count,
        settlement.first_provable_count,
        settlement.provisional_count,
        settlement.confirmed_count,
        settlement.force_overtake_count,
        settlement.never_constituted_count,
        settlement.identity_vanished_count,
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
) -> Result<(YieldBook, usize, usize, SparseStats, LifecycleReplayStats), String> {
    let mut parser = ParseLayerIncr::new(config);
    let mut cache = classifier::TowerCache::new();
    let mut book = YieldBook::default();
    let mut turns = TurnBook::default();
    let mut pending: BTreeSet<EventKey> = targets.keys().cloned().collect();
    let mut views = 0usize;
    let mut last_trigger = None;
    let mut last_lifecycle_trigger = None;
    let mut last_lifecycle_forest_epoch = None;
    let mut lifecycle_book = NestLifecycleBook::new();
    let mut lifecycle_stats = LifecycleReplayStats::default();
    // #527：活窗结构分量在「confirmed 侧变（forest_epoch）∨ active C frontier 变」时重算；
    // 两次重算之间逐 bar 只延展右端（身份分量不动）。frontier 是纯值 ⟹ 值不变 ⟹ 定位输出
    // 不变（locate/extreme 都是其纯函数），跳过重算是等价优化而非行为改动。
    let mut lifecycle_window_stems: Vec<LifecycleWindowStem> = Vec::new();
    let mut last_lifecycle_frontier: Option<Option<ActiveSegmentFrontier>> = None;
    let mut lifecycle_dump = std::env::var("P421_LIFECYCLE_DUMP")
        .ok()
        .map(|path| {
            File::create(&path)
                .map(BufWriter::new)
                .map_err(|error| format!("创建 P421_LIFECYCLE_DUMP={path} 失败: {error}"))
        })
        .transpose()?;
    // #553（N1-T4）候选事件 dump：独立 sink，与 P116_DUMP 既有封印面互不触碰（只写不判）。
    let mut event_dump = EventDump::from_env()?;
    // #641（N3）链证书 dump：同款独立 sink（只写不判），env 未设 ⟹ 零行为差异。
    let mut chain_dump = ChainDump::from_env(bars.len())?;
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
        // #641：链 dump 侧信道（只写不判；关灯时首行返回）。放在既有 TURN 观察之前，只对
        // `cache` 做只读取数，不改 classification/tower/turns 的任何输入与产生条件。
        chain_dump.observe(&cache, index, index + 1 == bars.len())?;
        // #116: TURN 逐 bar 观察（摊还 O(新稳定块数)，不经 trigger 门——pending 空后仍落盘）。
        turns.observe(&classification);
        let forest_epoch = cache.forest_epoch();
        // #527 L1 活窗：C 只能是 parser 的行进中段（active frontier），禁 confirmed 段回放重建。
        let frontier = active_segment_frontier(&l0);
        if last_lifecycle_forest_epoch != Some(forest_epoch)
            || last_lifecycle_frontier != Some(frontier)
        {
            let before = lifecycle_window_stems.len();
            let mut miss_rows: Vec<(&'static str, Option<usize>)> = Vec::new();
            lifecycle_window_stems = recompute_lifecycle_window_stems(
                &tower,
                frontier.as_ref(),
                index,
                &mut lifecycle_stats.l1_live_outcomes,
                &mut miss_rows,
            );
            for (reason, center_hint) in miss_rows {
                write_lifecycle_line(
                    &mut lifecycle_dump,
                    format_args!(
                        "L1_LIVE_MISS as_of={index} frontier_start={} reason={reason} b_center_start={}",
                        frontier.map_or(usize::MAX, |f| f.start_index),
                        center_hint.map_or(usize::MAX, |value| value),
                    ),
                )?;
            }
            // 定位现场只在结构分量变化时落一行（诊断只写不判；每 bar 写会淹没 dump）。
            write_lifecycle_line(
                &mut lifecycle_dump,
                format_args!(
                    "L1_LIVE_RECOMPUTE as_of={index} frontier={} stems_before={before} stems_after={}",
                    frontier.map_or("none".to_string(), |f| format!(
                        "({},{},{:?})",
                        f.start_index, f.extreme_at, f.direction
                    )),
                    lifecycle_window_stems.len(),
                ),
            )?;
            last_lifecycle_forest_epoch = Some(forest_epoch);
            last_lifecycle_frontier = Some(frontier);
        }
        let trigger = (forest_epoch, signal_signature(&classification));
        let lifecycle_due = last_lifecycle_trigger.as_ref() != Some(&trigger);
        if last_trigger.as_ref() != Some(&trigger) && !pending.is_empty() {
            stats.triggers += 1;
            let (hist, close_src) = cache.causal_series();
            let dif = cache.macd_dif();
            // arrived 分组：与慢版 collect_target_candidates 首段逐行一致。
            let mut runs_by_level: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
            for key in &pending {
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
                // ── 派生缓存同步（targeted 与 lifecycle 共用同一 LevelDerived）──
                let (self_synced, lower_synced) = sync_level_derived(level, &tower, &mut derived)?;
                if self_synced {
                    stats.syncs_self += 1;
                }
                if lower_synced {
                    stats.syncs_lower += 1;
                }
                let level_derived = derived.get_mut(&level).expect("刚同步的 level 必须存在");
                let active_run_starts: Vec<_> = level_derived.run_ranges.keys().copied().collect();
                level_derived
                    .confirm_cursors
                    .retain_run_starts(level as u32, active_run_starts);
                let stable_lower_len = cache.tower_confirmed_len(level - 1);
                let pan_freeze_boundary = cache.freeze_boundary(level - 1).unwrap_or(0);
                for run_source_start in run_sources {
                    // 慢版：run 不在当前分区 ⟹ 本 trigger 无产出（continue）。条目保留不应用——
                    // run 重现时代次差（分区已随内容变同步过）⟹ dirty ⟹ 重估，无陈旧复用。
                    let Some(&(start, end)) = level_derived.run_ranges.get(&run_source_start)
                    else {
                        continue;
                    };
                    *stats.per_level_views_slow_would.entry(level).or_default() += 1;
                    let watermark_crossed = entries
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
                        });
                    let dirty = match entries.get(&(level, run_source_start)) {
                        None => true, // 冷条目：首次评估
                        Some(entry) => {
                            let self_changed =
                                entry.target_self_gen != Some(level_derived.self_gen);
                            let lower_changed =
                                entry.target_lower_gen != Some(level_derived.lower_gen);
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
                        let structure_generation = level_derived.self_gen;
                        let entry = entries.entry((level, run_source_start)).or_insert_with(|| {
                            RunEntry::new(
                                level_derived.self_gen,
                                level_derived.lower_gen,
                                index,
                                Vec::new(),
                                Vec::new(),
                                Vec::new(),
                            )
                        });
                        let (centers, kinds, events) = {
                            let lower_legs = &level_derived.lower_legs;
                            let confirm_cursors = &mut level_derived.confirm_cursors;
                            evaluate_run(
                                level,
                                &tower[level],
                                (start, end),
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
                        }?;
                        views += 1;
                        stats.reevals += 1;
                        *stats.per_level_reevals.entry(level).or_default() += 1;
                        fresh_levels.insert(level);
                        entry.update(
                            level_derived.self_gen,
                            level_derived.lower_gen,
                            index,
                            centers,
                            kinds,
                            events,
                        );
                        entry.target_self_gen = Some(level_derived.self_gen);
                        entry.target_lower_gen = Some(level_derived.lower_gen);
                        entry.target_last_as_of = Some(index);
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
                        let (_, _, forced) = evaluate_run(
                            level,
                            &tower[level],
                            (start, end),
                            &level_derived.lower_legs,
                            index,
                            hist,
                            dif,
                            close_src,
                            None,
                            None,
                        )?;
                        let forced: Vec<NestCandidateEvent> = forced
                            .into_iter()
                            .filter(|event| pending.contains(&EventKey::from(event)))
                            .collect();
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
                let pending_hit = pending.contains(&key);
                // #553 候选事件 dump（只写不判）：写在既有 pending 短路**之前** ⟹ 落盘的是
                // 完整候选事件流（含本 trigger 内已被前序事件出清的观察）。`pending.contains`
                // 是纯查询，提出到 `pending_hit` 不改变既有控制流与既有 dump 行的产生顺序。
                event_dump.observe(&event, index, pending_hit)?;
                if !pending_hit {
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
                    || event_bsp_book_level(event.level).is_some_and(|book_level| {
                        bsp_changed.get(&book_level).copied().unwrap_or(true)
                    });
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
            last_trigger = Some(trigger.clone());
        }
        // #421：targeted 先按原物理 views 口径更新共享条目；sidecar 的 provider trigger
        // 随后只补 dirty 非目标 run，并刷新完成事件。活窗由上方独立 p409 同构循环发现，
        // 不借生产 RunEntry 的完成时刻快照。账本本身每根 bar 都喂。
        let mut completion_events = Vec::new();
        if lifecycle_due {
            let (hist, close_src) = cache.causal_series();
            let dif = cache.macd_dif();
            let active_runs = refresh_lifecycle_cache(
                &tower,
                &cache,
                index,
                hist,
                dif,
                close_src,
                &mut derived,
                &mut entries,
                &mut lifecycle_stats,
            )?;
            completion_events = active_runs
                .iter()
                .flat_map(|key| entries[key].events.iter().copied())
                .collect();
            lifecycle_stats.provider_triggers += 1;
            last_lifecycle_trigger = Some(trigger);
        }
        let bar_windows: Vec<PanLiveWindow> = lifecycle_window_stems
            .iter()
            .copied()
            .map(|stem| stem.window_at(index))
            .collect();
        // #527：完成相是显式 typed 输出——每条完成事件都必须能在塔上查到那只已完成的
        // lower unit（身份 + 物理完成 bar），否则停线（禁按 kind 猜 provenance）。
        let phases = lifecycle_bar_phases(&tower, &bar_windows, &completion_events, index)?;
        let (hist, close_src) = cache.causal_series();
        let dif = cache.macd_dif();
        feed_lifecycle_bar(
            &mut lifecycle_book,
            &phases,
            index,
            hist,
            dif,
            close_src,
            &mut lifecycle_dump,
            &mut lifecycle_stats,
        )?;
        lifecycle_book.assert_invariants();
        if ckpt_every > 0 && index > 0 && index % ckpt_every == 0 {
            let (hist, close_src) = cache.causal_series();
            let dif = cache.macd_dif();
            let mut ckpt_audit = ProviderAudit::default();
            match collect_snapshot_candidates(&tower, index, hist, dif, close_src, &mut ckpt_audit)
            {
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
    for entry in entries.values() {
        let pan = entry.pan_memo.stats();
        stats.pan_entries += entry.pan_memo.len();
        stats.pan_hits += pan.hits;
        stats.pan_misses += pan.misses;
        stats.pan_writes += pan.writes;
        stats.pan_invalidations += pan.invalidations;
    }
    lifecycle_stats.settlement = lifecycle_book.settlement_stats();
    let settlement = lifecycle_stats.settlement;
    write_lifecycle_line(
        &mut lifecycle_dump,
        format_args!(
            "LIFETIME entries={} first_provable={} provisional={} confirmed={} force_overtake={} never_constituted={} identity_vanished={} flash_terminal={} nonflash_count={} nonflash_min={:?} nonflash_median={:?} nonflash_max={:?} force_lifetime_count={} force_lifetime_min={:?} force_lifetime_median={:?} force_lifetime_max={:?}",
            settlement.entry_count,
            settlement.first_provable_count,
            settlement.provisional_count,
            settlement.confirmed_count,
            settlement.force_overtake_count,
            settlement.never_constituted_count,
            settlement.identity_vanished_count,
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
    )?;
    if let Some(writer) = lifecycle_dump.as_mut() {
        writer
            .flush()
            .map_err(|error| format!("刷新 P421_LIFECYCLE_DUMP 失败: {error}"))?;
    }
    event_dump.flush()?;
    chain_dump.flush()?;
    Ok((book, views, pending.len(), stats, lifecycle_stats))
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

/// #527 L1 活窗发现：**confirmed A/B 锚 + active C frontier**。
///
/// 与 #421 逃生门原实装的差别（#523 根因）：C 不再从已完成 lower legs 回放重建，而只能是
/// parser 的行进中段（`frontier`）——故活窗可在完成前出生。B 中枢仍取 confirmed 侧
/// （tower[1] 的 run 投影 seeds），A 锚仍取 confirmed 段（窄锚/A′ 与 provider 同序同判）。
///
/// **级别范围（诚实登记，票 #527 Scope）**：只有 L1 有可用的 active lower-frontier
/// （parser `OpenTail.pendingSegment`）。L2/L3 的 lower unit 是 `LeveledMove`，塔上没有
/// Active/Completed 表达（#523 遗留问题 1），**故本函数不为 L2/L3 产活窗**——不拿已完成
/// tower unit 外推「行进中」（那会重演同一 bug）。L2/L3 的身份因此只经完成相进账本，
/// 其闪现是 provider 能力缺口，不是「该对象确实同 bar 出生并完成」的 true-flash。
///
/// 只在 `forest_epoch` 或 `frontier` 变化时执行；不写生产缓存、不改 trigger/订单/证书路径。
fn recompute_lifecycle_window_stems(
    tower: &[Rc<Vec<LeveledMove>>],
    frontier: Option<&ActiveSegmentFrontier>,
    as_of: usize,
    misses: &mut BTreeMap<&'static str, usize>,
    outcomes: &mut Vec<(&'static str, Option<usize>)>,
) -> Vec<LifecycleWindowStem> {
    let mut windows_out = Vec::new();
    let Some(frontier) = frontier else {
        // 无行进中段 ⟹ 无活窗（不回落到 confirmed 回放重建）。
        *misses.entry("no_active_frontier").or_default() += 1;
        outcomes.push(("no_active_frontier", None));
        return Vec::new();
    };
    for level in 1..2.min(tower.len()) {
        let Ok(lower) = lower_legs_from(&tower[level - 1]) else {
            continue;
        };
        let segments: Vec<Segment> = lower.iter().map(lifecycle_leg_as_segment).collect();
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
                        let outcome = provide_l1_active_pan_live_windows(
                            level as u32,
                            &centers,
                            &kinds,
                            &segments,
                            frontier,
                            as_of,
                        );
                        *misses.entry(outcome.reason_tag()).or_default() += 1;
                        if outcome.window().is_none() {
                            // 未产窗的 run：记下该 run 最近中枢起点，供「这只完成身份为何
                            // 没有更早 Live」逐身份归因（诊断只写不判）。
                            let center_hint = centers
                                .iter()
                                .rev()
                                .find(|center| center.end_index <= frontier.start_index)
                                .map(|center| center.start_index);
                            outcomes.push((outcome.reason_tag(), center_hint));
                        }
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

/// 补齐 lifecycle 本 trigger 所需的全部 active run。targeted 已在同 trigger 更新过的
/// 条目直接复用；其余条目也只在共享 dirty 判据命中时评估，禁 sidecar 每 trigger 全量重算。
#[allow(clippy::too_many_arguments)]
fn refresh_lifecycle_cache(
    tower: &[Rc<Vec<LeveledMove>>],
    cache: &classifier::TowerCache,
    as_of: usize,
    hist: &[f64],
    dif: &[f64],
    close_src: &[usize],
    derived: &mut BTreeMap<usize, LevelDerived>,
    entries: &mut BTreeMap<(usize, usize), RunEntry>,
    stats: &mut LifecycleReplayStats,
) -> Result<Vec<(usize, usize)>, String> {
    let mut active_runs = Vec::new();
    for level in 1..tower.len() {
        sync_level_derived(level, tower, derived)?;
        let active_run_starts: Vec<_> = derived[&level].run_ranges.keys().copied().collect();
        derived
            .get_mut(&level)
            .expect("刚同步的 level 必须存在")
            .confirm_cursors
            .retain_run_starts(level as u32, active_run_starts);
        let stable_lower_len = cache.tower_confirmed_len(level - 1);
        let pan_freeze_boundary = cache.freeze_boundary(level - 1).unwrap_or(0);
        let run_ranges: Vec<(usize, (usize, usize))> = derived[&level]
            .run_ranges
            .iter()
            .map(|(&source, &range)| (source, range))
            .collect();
        for (run_source_start, run) in run_ranges {
            stats.provider_requests += 1;
            let level_derived = &derived[&level];
            let watermark_crossed = entries
                .get(&(level, run_source_start))
                .is_some_and(|entry| {
                    let before = level_derived
                        .lower_ends
                        .partition_point(|&end_index| end_index <= entry.last_as_of);
                    let now = level_derived
                        .lower_ends
                        .partition_point(|&end_index| end_index <= as_of);
                    now != before
                });
            let dirty = entries.get(&(level, run_source_start)).is_none_or(|entry| {
                entry.self_gen != level_derived.self_gen
                    || entry.lower_gen != level_derived.lower_gen
                    || watermark_crossed
            });
            if dirty {
                let self_gen = level_derived.self_gen;
                let lower_gen = level_derived.lower_gen;
                let entry = entries.entry((level, run_source_start)).or_insert_with(|| {
                    RunEntry::new(
                        self_gen,
                        lower_gen,
                        as_of,
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                    )
                });
                let (centers, kinds, events) = {
                    let level_derived = derived.get_mut(&level).expect("刚同步的 level 必须存在");
                    let lower_legs = &level_derived.lower_legs;
                    let confirm_cursors = &mut level_derived.confirm_cursors;
                    evaluate_run(
                        level,
                        &tower[level],
                        run,
                        lower_legs,
                        as_of,
                        hist,
                        dif,
                        close_src,
                        Some(ConfirmResidence {
                            store: confirm_cursors,
                            stable_lower_len,
                            structure_generation: self_gen,
                        }),
                        Some(PanResidence {
                            memo: &mut entry.pan_memo,
                            freeze_boundary_src: pan_freeze_boundary,
                        }),
                    )
                }?;
                entry.update(self_gen, lower_gen, as_of, centers, kinds, events);
                stats.provider_reevals += 1;
            } else {
                stats.provider_reuses += 1;
            }
            active_runs.push((level, run_source_start));
        }
    }
    Ok(active_runs)
}

/// 组装本 bar 的 provider 两相（票 #527：完成相是显式 typed 输出）。
///
/// 完成相的构造前提 = **该完成事件对应的 lower unit 已在塔上作为已完成单元存在**：
/// 用 `interval_b` 右端（= 完成 C 段终点，pan 分支 `structure.seg_c.1` 单一来源）在
/// `tower[level-1]` 上按 `end_index` 查证，取其 `ElementId` 与物理完成 bar。
/// 查不到 ⟹ 报错停线（**不**按 `kind == Consolidation` 猜完成，#523 根因之二）。
///
/// 完成钟三分：`completed_at` = lower unit `end_index`（物理完成）；
/// `observed_completion_at` = 本 bar（provider 首次可见）；账本收到 bar 由 `advance` 记。
/// 事件重发时 `observed_completion_at` 仍取当前 bar，首见性由 book 的
/// `completion_signals` 按桥身份唯一保证（重发不入分母）。
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
            observed_completion_at: as_of,
        }));
    }
    Ok(phases)
}

#[allow(clippy::too_many_arguments)]
fn feed_lifecycle_bar(
    book: &mut NestLifecycleBook,
    phases: &[PanProviderPhase],
    as_of: usize,
    hist: &[f64],
    dif: &[f64],
    close_src: &[usize],
    sink: &mut Option<BufWriter<File>>,
    replay_stats: &mut LifecycleReplayStats,
) -> Result<(), String> {
    let completion_signal_start = book.completion_signals().len();
    let audit_start = book.completion_force_unavailable_audits().len();
    let material = ForceMaterial {
        hist: Some(hist),
        dif: Some(dif),
        close_src,
    };
    let (delta, stats) = feed_replay_bar(book, &ReplayBarFeed { as_of, phases }, &material);

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
    )?;
    for signal in &book.completion_signals()[completion_signal_start..] {
        write_lifecycle_line(
            sink,
            format_args!(
                "COMPLETION_SIGNAL as_of={} level={} side={:?} kind={:?} seg_a={:?} seg_c_full={:?} b_center_start={} lower_id={:?} completed_at={} observed_completion_at={}",
                signal.as_of,
                signal.key.level,
                signal.key.side,
                signal.key.kind,
                signal.key.seg_a,
                signal.key.seg_c_full,
                signal.key.b_center_start,
                (signal.completed_lower_id.level, signal.completed_lower_id.ordinal),
                signal.completed_at,
                signal.observed_completion_at,
            ),
        )?;
    }
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
    for audit in &book.completion_force_unavailable_audits()[audit_start..] {
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
    replay_stats.observe(stats);
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
    #[test]
    fn lifecycle_window_stem_keeps_identity_while_extending_bar() {
        let stem = LifecycleWindowStem {
            level: 2,
            side_tag: 0,
            seg_a: (10, 19),
            c_start: 30,
            b_center_start: 20,
        };
        let first = stem.window_at(30);
        let later = stem.window_at(99);

        assert_eq!(LifecycleWindowStem::of(&first), stem);
        assert_eq!(LifecycleWindowStem::of(&later), stem);
        assert_eq!(first.seg_c_live, (30, 30));
        assert_eq!(later.seg_c_live, (30, 99));
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
    #[test]
    fn chain_dump_cadence_advances_on_beat_and_on_last_bar() {
        let cache = classifier::TowerCache::new();
        let sink: Rc<RefCell<Vec<u8>>> = Rc::new(RefCell::new(Vec::new()));
        let mut dump = ChainDump::new(Some(Box::new(SharedSink(Rc::clone(&sink)))), 3);
        // 空事件流 ⟹ Delta 恒空，但推进与否可由簿的 revision 数以外的可观测面判定：
        // 这里断言的是**不 panic + 零行**（节拍分支被真走过，行数为 0 是空流的后果）。
        for as_of in 0..5 {
            dump.observe(&cache, as_of, as_of == 4).unwrap();
        }
        dump.flush().unwrap();
        assert!(String::from_utf8(sink.borrow().clone()).unwrap().is_empty());
        assert_eq!(dump.seq, 0, "空事件流 ⟹ 零 Delta ⟹ 零行序推进");
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
