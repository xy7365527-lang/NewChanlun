//! #92/#93 证书索引：确认事件 → typed 证书（`NestEventIdentity` 主键）。
//!
//! 构建口径（cert-index-caliber-audit-20260721）：
//! - 生产区间口径固定 B（由调用方传入，`nest.rs:386-387` 裁定）；
//! - 终端背书口径固定 CWindow（p117 T1 裁定2），唯一落点
//!   [`terminal_bits_at_event`]，构建器不加额外过滤（禁第二查法）；
//! - exec 循环从 1 起扫——`events_by_level[0]` 被跳过（nest 不听 L0，p105 §3）；
//! - 每基例取**最深可行链**（top 自塔顶向下回退到首个可装配级），单级证书
//!   （rungs 空）合法（`assemble_typed_certificate` 边界条件 (2)）；
//! - 去重 first-wins（与 gate `seen` 语义同款；身份键 = 基例 `NestEventIdentity`）。
//!
//! 判定谓词唯一来源不变（`cert.certificate().n_delta()`，nest.rs 递归核）；
//! 本模块只做生产装配与查表，不持判定。
//!
//! GUARD-ROLE: nest-pipeline
//! （#451；tests/nest_isolation_guard.rs 认上面这一整行豁免，不认文件名——
//! nest 管线本体的生产装配件，见 #449 §2 裁定）

use std::collections::HashMap;

use super::super::types::Tick;
use super::level_view::{NestCandidateEvent, NestDivergenceKind};
use super::nest::{
    assemble_typed_certificate, terminal_bits_at_event_measured, EndorsementProbe,
    NestEventIdentity, NestIntervalCaliber, OwnerAnchorCtx, TerminalMatch, TypedNestCertificate,
};
use super::Classification;

/// 索引构建统计（NEST_GATE_INDEX 行消费；只作归因/对账，不参与判定）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NestCertificateIndexStats {
    /// exec 循环实际考察的基例事件数（L1+；L0 被设计性跳过）。
    pub base_events: usize,
    /// 装配成功的证书张数（去重前；基例力度门/终端背书拒证不计）。
    pub assembled: usize,
    /// 去重后实际入索引的证书张数。
    pub indexed: usize,
    /// 入索引证书中纯基例（rungs=0，单级）张数。
    pub rungs_0: usize,
    /// 入索引证书中链深 1（rungs=1）张数。
    pub rungs_1: usize,
    /// 入索引证书中链深 ≥2 张数。
    pub rungs_2_plus: usize,
}

impl NestCertificateIndexStats {
    /// 单级证书占比（`rungs_0 / indexed`）；索引为空时 `None`。
    pub fn single_level_share(&self) -> Option<f64> {
        if self.indexed == 0 {
            None
        } else {
            Some(self.rungs_0 as f64 / self.indexed as f64)
        }
    }
}

/// #214 背书失败原因测量 sidecar（spec endorsement-failure-instrument-20260724 ID-4；
/// 只读计数——既有 [`NestCertificateIndexStats`] 六字段及其语义、`single_level_share`、
/// Copy 语义全保留；本结构经独立访问器 [`NestCertificateIndex::instrument`] 读出，
/// 不进任何判定、不改索引内容一个 bit）。
///
/// 口径（spec ID-2/ID-3/ID-4 + 2026-07-24 编排者钉口径①-⑥）：
/// - 按级 × kind 两维分解：行 = 级别槽（exec 循环级；槽 0 恒零——nest 不听 L0），
///   列 `[Trend, Consolidation]`（钉口径②）；
/// - Trend 域四桶互斥完备（穿透序首因：窗口 → 方向 → owner），成功不计桶；
/// - 点级二维 = Trend 域窗口内同向点 ×（一/二/三类 × owner 等值/不等），事件窗内
///   逐窗各计（钉口径①），成功事件的窗内同向点同样入计（相等率分母覆盖全体）；
/// - Pan 域只计成功/失败总数（钉口径⑥：无 owner 合取，四桶/点级不适用）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NestEndorsementInstrument {
    /// exec 考察的基例事件数按级 × kind 分解（`[级别槽][Trend, Consolidation]`）。
    pub base_by_level_kind: Vec<[usize; 2]>,
    /// 装配成功张数（去重前）按级 × kind 分解。
    pub assembled_by_level_kind: Vec<[usize; 2]>,
    /// 入索引张数（去重后）按级 × kind 分解。
    pub indexed_by_level_kind: Vec<[usize; 2]>,
    /// Trend 域：背书成功事件数（窗内有同向 owner 判等点 ⟹ 判定必命中，不计桶）。
    pub trend_success: usize,
    /// Trend 域四桶（互斥完备，穿透序首因归属，spec ID-2）：窗内有同向点但
    /// owner 判等点数为 0（#218 面 B 起桶名「owner 锚不等」——锚不可解与实不等同归
    /// 本桶、点子计数分列；行族 schema 演进随票登记，非冻结红线面）。
    pub trend_owner_anchor_neq: usize,
    /// 四桶之二：窗内无点 ∧ 账本窗外有点（宽口径，不限方向，钉口径③）。
    pub trend_out_of_window: usize,
    /// 四桶之三：窗内有点但同向点数为 0。
    pub trend_opposite_side: usize,
    /// 四桶之四：账本无任何点 / 账本级别缺失越界（子计数分列见下两字段）。
    pub trend_no_valid_point: usize,
    /// `trend_no_valid_point` 子计数：账本级别缺失/越界（结构性 None）。
    pub nvp_book_missing: usize,
    /// `trend_no_valid_point` 子计数：账本存在但无任何点。
    pub nvp_book_empty: usize,
    /// owner 非等值点子计数（全 Trend 事件窗内同向点口径）：**锚不可解**（#218 面 B
    /// 语义重定，spec ID-4——二类点侧/事件侧锚供给缺失、中枢参照 B 查找失败（生产
    /// 不可达防御）、点无载体（非生产合成形状）同归：身份合取不可证，诚实判负）。
    pub owner_anchor_missing_pts: usize,
    /// owner 非等值点子计数（同上口径）：判同两侧皆可判且实不等（带不等 / 锚不等）。
    pub owner_real_neq_pts: usize,
    /// 带碰撞观察读数（#218 US-08）：中枢参照判同中带等但序号不等（同 (zd,zg) 非同
    /// 中枢）的点数——带判同收紧（加 dd/gg）与否的实测依据；只读计数，不进判定。
    pub band_eq_start_neq_pts: usize,
    /// Trend 域点级二维计数：`[一/二/三类][owner 判等/不等]`（C1 相等率分子/分母，
    /// spec ID-3；一位多类点按所属类逐行各计；「不等」列 = 实不等 + 锚不可解）。
    pub trend_pts: [[usize; 2]; 3],
    /// 遍历计数（#218 ID-5 不变量断言集读数）：全 Trend 事件账本遍历总点数
    ///（Σ `book_total`；窗口/方向谓词不动的对照量）。
    pub scan_book_total: usize,
    /// 遍历计数：全 Trend 事件窗口 `[c_start, t*]` 内点数合计（Σ `in_window`）。
    pub scan_in_window: usize,
    /// 遍历计数：全 Trend 事件窗口内同向点数合计（Σ `in_window_same_side`）。
    pub scan_in_window_same_side: usize,
    /// Pan 域：背书成功事件总数（`confirm_side` 即精确语义，钉口径⑥）。
    pub pan_success: usize,
    /// Pan 域：背书失败事件总数（不进一步分桶）。
    pub pan_fail: usize,
}

impl NestEndorsementInstrument {
    /// base 事件 kind 分解：Trend 计数（C2 分母）。
    pub fn base_trend(&self) -> usize {
        self.base_by_level_kind.iter().map(|row| row[0]).sum()
    }
    /// base 事件 kind 分解：Consolidation（Pan）计数（C2 分母）。
    pub fn base_consolidation(&self) -> usize {
        self.base_by_level_kind.iter().map(|row| row[1]).sum()
    }
    /// Trend 域事件级平账：成功 + 四桶合计（完备性断言恒等 `base_trend()`，测试与
    /// replay 校验共用，spec ID-2 完备性条款）。
    pub fn trend_total(&self) -> usize {
        self.trend_success
            + self.trend_owner_anchor_neq
            + self.trend_out_of_window
            + self.trend_opposite_side
            + self.trend_no_valid_point
    }
    /// 逐事件副产品归集（构建循环每基例一次）。桶归属 = spec ID-2 穿透序
    /// （窗口 → 方向 → owner），与生产合取序同构，不引入任何新谓词。
    fn observe_base(
        &mut self,
        exec: usize,
        kind: NestDivergenceKind,
        endorsed: bool,
        probe: &EndorsementProbe,
    ) {
        self.base_by_level_kind[exec][kind_idx(kind)] += 1;
        match kind {
            NestDivergenceKind::Trend => {
                // ID-3 点级二维（成功+失败全事件窗内同向点；成功不计桶但点入计）。
                for (dst, src) in self.trend_pts.iter_mut().zip(probe.trend_pts.iter()) {
                    dst[0] += src[0];
                    dst[1] += src[1];
                }
                self.owner_anchor_missing_pts += probe.owner_anchor_missing_pts;
                self.owner_real_neq_pts += probe.owner_real_neq_pts;
                self.band_eq_start_neq_pts += probe.band_eq_start_neq_pts;
                // 遍历计数（#218 ID-5 不变量断言集读数，随事件归集）。
                self.scan_book_total += probe.book_total;
                self.scan_in_window += probe.in_window;
                self.scan_in_window_same_side += probe.in_window_same_side;
                // 穿透序首因归属（ID-2）：窗口 → 方向 → owner；规则4 命中不计桶。
                if endorsed {
                    self.trend_success += 1;
                } else if probe.book_missing {
                    self.trend_no_valid_point += 1;
                    self.nvp_book_missing += 1;
                } else if probe.in_window == 0 {
                    // 宽口径（钉口径③）：窗内无点 ∧ 账本窗外有点 ⟹ 窗口外；
                    // 账本无任何点 ⟹ 无合法点（in_window=0 ⟹ 窗外点数 = book_total）。
                    if probe.book_total > 0 {
                        self.trend_out_of_window += 1;
                    } else {
                        self.trend_no_valid_point += 1;
                        self.nvp_book_empty += 1;
                    }
                } else if probe.in_window_same_side == 0 {
                    self.trend_opposite_side += 1;
                } else {
                    // 窗内有同向点但 endorsed=false ⟹ owner 判等 0（规则3，#218 桶名）。
                    self.trend_owner_anchor_neq += 1;
                }
            }
            NestDivergenceKind::Consolidation => {
                // 钉口径⑥：Pan 域只计成功/失败总数，四桶/点级不适用。
                if endorsed {
                    self.pan_success += 1;
                } else {
                    self.pan_fail += 1;
                }
            }
        }
    }
}

/// kind → 按级 × kind 分解矩阵列号（列序 `[Trend, Consolidation]` 的单点定义；
/// sidecar 三矩阵与打印行共用，防两处各自声明漂移）。
fn kind_idx(kind: NestDivergenceKind) -> usize {
    match kind {
        NestDivergenceKind::Trend => 0,
        NestDivergenceKind::Consolidation => 1,
    }
}

/// 证书索引：`NestEventIdentity`（基例身份）→ [`TypedNestCertificate`]。
///
/// 查表侧（进场侧与出场侧统一经 `chain_lookup`——出场侧已随 T5b (#208) 迁链，
/// 旧 `typed_lookup` 删除）经 [`Self::get`] 读出，判定经 `cert.certificate().n_delta()`
/// 单一来源复验。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NestCertificateIndex {
    by_id: HashMap<NestEventIdentity, TypedNestCertificate>,
    stats: NestCertificateIndexStats,
    /// #214 测量 sidecar（只读计数，ID-4；不进判定、不改索引内容）。
    instrument: NestEndorsementInstrument,
}

impl NestCertificateIndex {
    /// 身份键查证书。
    pub fn get(&self, id: &NestEventIdentity) -> Option<&TypedNestCertificate> {
        self.by_id.get(id)
    }

    /// 索引内证书张数。
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    /// 构建统计快照。
    pub fn stats(&self) -> NestCertificateIndexStats {
        self.stats
    }

    /// #214 测量 sidecar（只读计数；NEST_GATE_FAIL/NEST_GATE_LEVEL 新行消费，
    /// 不参与判定、不影响索引内容）。
    pub fn instrument(&self) -> &NestEndorsementInstrument {
        &self.instrument
    }
}

/// 从确认事件账本构建证书索引。
///
/// `events_by_level[ℓ]` = gate `absorb_exts` 落账的 level-ℓ 确认事件
/// （`divergence_confirmed` 已过滤、身份已 first-wins 去重）。
///
/// #214（spec ID-1C）：终端背书经测量入口 [`terminal_bits_at_event_measured`] 每基例
/// 求值一次——判定结果与既有 `terminal_bits_at_event` 闭包逐 bit 一致（共核），
/// 副产品探针归集进 [`NestEndorsementInstrument`]；既有六字段语义/索引内容/链闭合
/// 一个 bit 不动（ID-6 零干预）。
///
/// ★#218 面 B（spec owner-attribution-fix-20260724 ID-2，主接缝签名扩展）：
/// - `anchor_at`：点判同 oracle（**仅供二类点判同**）——`anchor_at(source_index)` =
///   （极值价, 合并组锚），T1 供给线单一来源（projection.rs:143-145 同一查法，
///   调用链从供给在手处透传，禁第二查法）。
/// - `event_anchor_of`：事件侧两元锚查找（`NestCandidateEventExt` 已带
///   `extreme_price`/`group_anchor`，事件构造点就 seg_c.1 解析，零新增解析）——
///   gate 落账时随事件身份透传；`(None, _)`/`(_, None)` = 事件侧锚供给缺失（锚不可解）。
/// 一/三类判同的事件侧 B 带从 `classification` 账本层 `centers` 查出（`b_center_start`
/// 只当查找键），无需入参。
pub fn build_nest_certificate_index(
    classification: &Classification,
    events_by_level: &[Vec<NestCandidateEvent>],
    caliber: NestIntervalCaliber,
    anchor_at: &dyn Fn(usize) -> Option<(Tick, usize)>,
    event_anchor_of: &dyn Fn(&NestCandidateEvent) -> (Option<Tick>, Option<usize>),
) -> NestCertificateIndex {
    let mut index = NestCertificateIndex::default();
    let top_max = events_by_level.len().saturating_sub(1);
    // #214 按级 × kind 分解矩阵（行 = 级别槽；槽 0 恒零——exec 从 1 起扫，nest 不听 L0）。
    index.instrument.base_by_level_kind = vec![[0; 2]; events_by_level.len()];
    index.instrument.assembled_by_level_kind = vec![[0; 2]; events_by_level.len()];
    index.instrument.indexed_by_level_kind = vec![[0; 2]; events_by_level.len()];
    // exec 从 1 起扫：nest 不听 L0，p105 §3（events_by_level[0] 设计性忽略）。
    for exec in 1..events_by_level.len() {
        for base in &events_by_level[exec] {
            index.stats.base_events += 1;
            // 终端背书唯一落点：CWindow（p117 T1 裁定2）；构建器不加额外过滤。
            // #214：测量入口与生产判定共核，同一次账本遍历产出判定 + 失败分类副产品
            // （ID-2/ID-3 口径）；判定 bits 与既有闭包逐 bit 一致。
            // #218 面 B：判同参照包随事件构造（oracle 借用 + 事件两元锚透传）。
            let ctx = OwnerAnchorCtx {
                anchor_at,
                event_anchor: event_anchor_of(base),
            };
            let (endorsement, probe) =
                terminal_bits_at_event_measured(classification, base, TerminalMatch::CWindow, &ctx);
            index
                .instrument
                .observe_base(exec, base.kind, endorsement.is_some(), &probe);
            let terminal_bits = endorsement.map(|e| e.bits);
            let terminal_of = move |_: &NestCandidateEvent| terminal_bits;
            // 最深可行链：top 自塔顶向下回退，取首个可装配级（单级合法）。
            let cert = (exec..=top_max).rev().find_map(|top| {
                assemble_typed_certificate(events_by_level, base, top, caliber, &terminal_of)
            });
            let Some(cert) = cert else { continue };
            index.stats.assembled += 1;
            let ki = kind_idx(base.kind);
            index.instrument.assembled_by_level_kind[exec][ki] += 1;
            let id = NestEventIdentity::of(base);
            if let std::collections::hash_map::Entry::Vacant(slot) = index.by_id.entry(id) {
                let rungs = cert.judge_at().len().saturating_sub(1);
                match rungs {
                    0 => index.stats.rungs_0 += 1,
                    1 => index.stats.rungs_1 += 1,
                    _ => index.stats.rungs_2_plus += 1,
                }
                slot.insert(cert);
                index.stats.indexed += 1;
                index.instrument.indexed_by_level_kind[exec][ki] += 1;
            }
        }
    }
    index
}
