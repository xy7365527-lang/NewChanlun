//! 共享夹具（#648 T1 自 classifier/mod.rs 内联 `mod tests` 纯移动抽离，零行为变更）。
//! 可见性 pub(super) = classifier::tests 子树内共享。

use super::super::super::parser::ParseLayer;
use super::super::super::types::Direction;
use super::super::*;

pub(super) fn seg(dir: Direction, si: usize, ei: usize, sp: i64, ep: i64) -> Segment {
    Segment {
        direction: dir,
        start_index: si,
        end_index: ei,
        start_price: sp,
        end_price: ep,
    }
}

/// 构造 merged_bars：source_index 连续 0..n，close = vals（MACD 背驰真算用）。
pub(super) fn bars_from_closes(vals: &[i64]) -> Vec<super::super::super::types::Bar> {
    vals.iter()
        .enumerate()
        .map(|(i, &v)| super::super::super::types::Bar {
            source_index: i,
            timestamp: i as i64,
            open: v,
            high: v,
            low: v,
            close: v,
            volume: 1.0, // #919：f64
            untradable: false,
        })
        .collect()
}

pub(super) fn cache_candidate(c_start: usize, end: usize) -> cand_event::CandidateObservation {
    let key = cand_event::CandidateKey {
        rule_version: cand_event::CANDIDATE_RULE_VERSION,
        level: 0,
        kind: cand_event::CandidateKind::Trend,
        side: Side::Long,
        previous_center_start: Some(10),
        parent: cand_event::ParentFingerprint {
            center_start: 20,
            zd: 100,
            zg: 110,
        },
        seg_a: (11, 19),
        c_start,
    };
    cand_event::CandidateObservation {
        key,
        kind: cand_event::CandidateKind::Trend,
        center_ids: Some((10, 20)),
        candidate_group_id: c_start as u64,
        pair_id: (c_start as u64) + 1,
        structural_predicates: cand_event::StructuralPredicates {
            direction: true,
            comparable: true,
            extreme: true,
        },
        extreme_proof: key.seg_a,
        third_class_proof: None,
        interval: (c_start, end),
        state: cand_event::ObservedState::Provisional,
        first_provable_at: Some(end),
        confirmed_at: None,
    }
}

/// [`l0_units_stays_in_sync_with_tower0_on_segment_ledger_shrink`] 的两 bar 夹具。
///
/// - 第一 bar：6 段账本（confirmed 前缀 5，末段未确认——古怪线段可重划）；
/// - 第二 bar：段账本**回缩**到 4 段（末两段被重划吞并）⟹ 走 `cache.clear()` 分支。
pub(super) fn segment_ledger_shrink_fixture() -> (ParseLayer, ParseLayer) {
    let long_segments = vec![
        seg(Direction::Up, 0, 4, 100, 150),
        seg(Direction::Down, 4, 8, 150, 120),
        seg(Direction::Up, 8, 12, 120, 148),
        seg(Direction::Down, 12, 16, 148, 110),
        seg(Direction::Up, 16, 20, 110, 145),
        seg(Direction::Down, 20, 24, 145, 115),
    ];
    let closes: Vec<i64> = (0..28)
        .map(|i| 100 + if i % 2 == 0 { 20 } else { -20 })
        .collect();
    let long_layer = ParseLayer {
        segments: Rc::new(long_segments.clone()),
        segments_confirmed_len: 5,
        merged_bars: Rc::new(bars_from_closes(&closes)),
        ..Default::default()
    };
    let short_layer = ParseLayer {
        segments: Rc::new(long_segments[..4].to_vec()),
        segments_confirmed_len: 3,
        merged_bars: Rc::new(bars_from_closes(&closes)),
        ..Default::default()
    };
    (long_layer, short_layer)
}

/// #881 S3 集成夹具：12 段在 [0,10] 带内来回震荡（方向交替、任意三连段核心严格非空——
/// 所有段 [lo,hi] 覆盖 [3,8]）⟹ 主干塔 L0 延伸吸收成少量中枢、口径 S 每 3 段一个中枢，
/// 中枢两两外缘重叠（全 LevelExpansion）；L0 产 ≥3 个上级走势 ⟹ L1 可挂载。
pub(super) fn operation_oscillating_layer() -> ParseLayer {
    let specs: [(Direction, i64, i64); 12] = [
        (Direction::Up, 0, 10),
        (Direction::Down, 10, 2),
        (Direction::Up, 2, 9),
        (Direction::Down, 9, 3),
        (Direction::Up, 3, 8),
        (Direction::Down, 8, 2),
        (Direction::Up, 2, 10),
        (Direction::Down, 10, 3),
        (Direction::Up, 3, 9),
        (Direction::Down, 9, 2),
        (Direction::Up, 2, 8),
        (Direction::Down, 8, 3),
    ];
    ParseLayer {
        segments: Rc::new(
            specs
                .iter()
                .enumerate()
                .map(|(i, &(d, sp, ep))| seg(d, i * 2, i * 2 + 2, sp, ep))
                .collect(),
        ),
        ..Default::default()
    }
}

// ──────────────────────────────────────────────────────────────────────
//  增量塔 API bit-exact（task #93：incremental == 全量，逐 bar 断言）
// ──────────────────────────────────────────────────────────────────────

/// 构造逐段追加的合成段序列（方向交替 + 价格震荡，产足够中枢触发多级塔）。
pub(super) fn synthetic_segments(count: usize) -> Vec<Segment> {
    (0..count)
        .map(|i| {
            let dir = if i % 2 == 0 {
                Direction::Up
            } else {
                Direction::Down
            };
            let base = 100i64 + (i as i64) * 3;
            let swing = if i % 2 == 0 { 50 } else { -50 };
            let sp = base;
            let ep = base + swing;
            seg(dir, i * 4, i * 4 + 4, sp, ep)
        })
        .collect()
}

pub(super) fn candidate_rich_segments(count: usize) -> Vec<Segment> {
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
            seg(dir, i * 4, i * 4 + 4, start, price)
        })
        .collect()
}

/// 每 key 最新 revision（因果簿终态投影的机器口径，裁定(i)）。
pub(super) fn latest_by_key(
    streams: &cand_event::CandidateStreams,
) -> std::collections::BTreeMap<cand_event::CandidateKey, cand_event::CandidateEvent> {
    let mut latest = std::collections::BTreeMap::new();
    for stream in streams.iter() {
        for event in stream.iter() {
            latest.insert(event.key, event.clone());
        }
    }
    latest
}

pub(super) fn candidate_rich_layer() -> ParseLayer {
    let segments = candidate_rich_segments(120);
    let closes: Vec<i64> = (0..=segments.last().unwrap().end_index)
        .map(|i| 100 + if i % 8 < 4 { 35 } else { -35 } + (i as i64 / 16))
        .collect();
    ParseLayer {
        segments: Rc::new(segments),
        merged_bars: Rc::new(bars_from_closes(&closes)),
        ..Default::default()
    }
}

/// [`chain_book_over_prefixes`] 的**唯一变量**：`TowerCache` 是否跨前缀复用。
///
/// #676-3（尾部 LOW-4 / Fowler #2）：原先两个 18 行逐字重复的 helper
/// （`chain_book_over_prefixes` / `chain_book_over_prefixes_fresh_cache`）只差 cache 建在
/// 循环外还是循环内。合成一个函数 + 本枚举后，「两侧唯一差别就是 cache 复用与否」这件事
/// 由类型自证，不再靠读者逐行对比两份代码。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PrefixCacheReuse {
    /// 全程共享一个 cache（**因果簿**驱动：事件流 append-only，曾出现的候选身份永存）。
    SharedAcrossPrefixes,
    /// 每个前缀新建 cache（**终态窗口投影**驱动：fresh 无记忆，只投影当步终态）。
    FreshPerPrefix,
}

/// #641（N3）逐段前缀推进的链簿夹具（与 #550 主缝②同一段序列口径，规模收小以夹住 O(n²)）。
///
/// 每一步把该前缀喂给候选事件通道，再把**当步的事件流**喂给链簿推进一次——链簿因此走的是
/// 真实的多 `as_of` 生命史（覆盖边重算、可扩展性翻转、证伪跨越、终态封口），不是单点快照。
///
/// `reuse` 选事件流的驱动语义（见 [`PrefixCacheReuse`]）；除它之外两种驱动逐字同路：同一段
/// 前缀序列、同一 `ParseLayer` 构造、同一 `as_of`（`end`）、同一推进次数与顺序。
pub(super) fn chain_book_over_prefixes(
    segments: &[Segment],
    closes: &[i64],
    cfg: &ThetaConfig,
    reuse: PrefixCacheReuse,
) -> chain_cert::ChainCertificateBook {
    let mut book = chain_cert::ChainCertificateBook::default();
    let mut cache = TowerCache::new();
    for n in 1..=segments.len() {
        // #691 LOW-4：`match` 穷尽两档而非 `==` 单项判断——扩展第三档时编译红，不会静默落入
        // `SharedAcrossPrefixes` 分支（旧 `==` 写法对新增档零抵抗，见 commit message 反事实负控）。
        match reuse {
            PrefixCacheReuse::FreshPerPrefix => {
                // 唯一的差别就这一行：丢弃上一前缀的记忆（等价于原 `_fresh_cache` 版本在循环
                // **内**建 cache），其余一切逐字同路。
                cache = TowerCache::new();
            }
            PrefixCacheReuse::SharedAcrossPrefixes => {}
        }
        let end = segments[n - 1].end_index.min(closes.len() - 1);
        let layer = ParseLayer {
            segments: Rc::new(segments[..n].to_vec()),
            merged_bars: Rc::new(bars_from_closes(&closes[..=end])),
            ..Default::default()
        };
        let streams = classify_incremental(&layer, cfg, &mut cache, &[]).candidate_streams;
        book.advance(&streams, end);
    }
    book
}

pub(super) fn chain_fixture(count: usize) -> (Vec<Segment>, Vec<i64>) {
    let segments = candidate_rich_segments(count);
    let closes: Vec<i64> = (0..=segments.last().expect("非空").end_index)
        .map(|i| 100 + if i % 8 < 4 { 35 } else { -35 } + (i as i64 / 16))
        .collect();
    (segments, closes)
}

/// [`causal_and_terminal_projection_drives_agree_on_common_chain_keys`] 的比对口径：
/// 逐字段相同，唯一豁免 `edges[].skipped_levels[].alive_at_level` / `.inside_parent`——
/// 这两个数字是「该级别在**候选全集**里存活多少候选」的当场快照，候选全集本就由驱动
/// 决定（因果簿的候选全集是 append-only 累积，终态投影每步 fresh），两驱动在此项上
/// 天然不同不代表链身份/生命史分叉。除这两个数字外，`ChainEdge` 的其余字段
/// （`parent`/`child`/`kind`/`skipped_levels[].level`/`crossed_nodes`/`predicate_holds`/
/// `breach`）与证书的其余字段（`key`/`extends`/`nodes`/`status`/`revision_at` 等）逐一
/// 参与比对，不豁免。
/// 三层全部解构开头（不用 `..`）：新增字段落进任一层的字面量都会编译红在**本函数**，把
/// 「不豁免」的承诺从手写等式合取链的君子协定变成编译期事实（MED-2，同 diff 的
/// `issue550_event_battery::print_chain_readout` 已用同招）。`SkippedLevel` 的
/// `alive_at_level` / `inside_parent` 两字段仍在解构里显式列出、显式 `_` 排除——豁免理由
/// 见本函数上方 doc（候选全集计数由驱动决定，天然不同不代表链身份分叉）。
pub(super) fn certificates_agree_ignoring_alive_candidate_universe_counts(
    causal: &chain_cert::TowerChainCertificate,
    terminal: &chain_cert::TowerChainCertificate,
) -> bool {
    fn skipped_level_key(level: &chain_cert::SkippedLevel) -> u32 {
        let chain_cert::SkippedLevel {
            level,
            // 豁免：候选全集计数，见外层函数 doc。
            alive_at_level: _,
            inside_parent: _,
        } = level;
        *level
    }

    fn edges_agree(a: &chain_cert::ChainEdge, b: &chain_cert::ChainEdge) -> bool {
        let chain_cert::ChainEdge {
            parent: a_parent,
            child: a_child,
            kind: a_kind,
            skipped_levels: a_skipped_levels,
            crossed_nodes: a_crossed_nodes,
            predicate_holds: a_predicate_holds,
            breach: a_breach,
        } = a;
        let chain_cert::ChainEdge {
            parent: b_parent,
            child: b_child,
            kind: b_kind,
            skipped_levels: b_skipped_levels,
            crossed_nodes: b_crossed_nodes,
            predicate_holds: b_predicate_holds,
            breach: b_breach,
        } = b;
        a_parent == b_parent
            && a_child == b_child
            && a_kind == b_kind
            && a_crossed_nodes == b_crossed_nodes
            && a_predicate_holds == b_predicate_holds
            && a_breach == b_breach
            && a_skipped_levels.len() == b_skipped_levels.len()
            && a_skipped_levels
                .iter()
                .zip(b_skipped_levels.iter())
                .all(|(sa, sb)| skipped_level_key(sa) == skipped_level_key(sb))
    }

    let chain_cert::TowerChainCertificate {
        key: causal_key,
        extends: causal_extends,
        root_level: causal_root_level,
        leaf_level: causal_leaf_level,
        nodes: causal_nodes,
        edges: causal_edges,
        extendable: causal_extendable,
        status: causal_status,
        observed_at: causal_observed_at,
        closed_at: causal_closed_at,
        invalidated_at: causal_invalidated_at,
        invalidation_cause: causal_invalidation_cause,
        revision: causal_revision,
        supersedes_revision: causal_supersedes_revision,
        revision_at: causal_revision_at,
    } = causal;
    let chain_cert::TowerChainCertificate {
        key: terminal_key,
        extends: terminal_extends,
        root_level: terminal_root_level,
        leaf_level: terminal_leaf_level,
        nodes: terminal_nodes,
        edges: terminal_edges,
        extendable: terminal_extendable,
        status: terminal_status,
        observed_at: terminal_observed_at,
        closed_at: terminal_closed_at,
        invalidated_at: terminal_invalidated_at,
        invalidation_cause: terminal_invalidation_cause,
        revision: terminal_revision,
        supersedes_revision: terminal_supersedes_revision,
        revision_at: terminal_revision_at,
    } = terminal;

    causal_edges.len() == terminal_edges.len()
        && causal_edges
            .iter()
            .zip(terminal_edges.iter())
            .all(|(a, b)| edges_agree(a, b))
        && causal_key == terminal_key
        && causal_extends == terminal_extends
        && causal_root_level == terminal_root_level
        && causal_leaf_level == terminal_leaf_level
        && causal_nodes == terminal_nodes
        && causal_extendable == terminal_extendable
        && causal_status == terminal_status
        && causal_observed_at == terminal_observed_at
        && causal_closed_at == terminal_closed_at
        && causal_invalidated_at == terminal_invalidated_at
        && causal_invalidation_cause == terminal_invalidation_cause
        && causal_revision == terminal_revision
        && causal_supersedes_revision == terminal_supersedes_revision
        && causal_revision_at == terminal_revision_at
}

/// #551 生命史全谱合成夹具（几何逐点标注）。
///
/// 中枢1 `[1000,1200]` → 大跌 → 中枢2 `[400,600]` → 过渡走势（`398→260→100`，两段同向
/// 使三段重叠为空 ⟹ 不成中枢，走势低点 100 压到中枢3 下沿之下）→ 中枢3 `[150,250]`
/// （`dd=100 < zd=150`，故「破核心」与「破 b 包络极值」可分离）→ C 腿1 `140→130`（破
/// `zd=150`、未破 `b_lo=100` ⟹ 未决）→ 同 episode 反弹 `130→138`（不回中枢）→ C 腿2
/// `138→80`（破 `b_lo` ⟹ 可证）。
///
/// 中枢2 与中枢3 的 `dd/gg` 不相交 ⟹ `classify_relation` 判 `DownContinuation` ⟹ 趋势门
/// 对中枢3 开启，C 腿才进第一类候选域。段方向不要求严格交替（parser 允许同向相邻段）。
pub(super) fn lifecycle_rich_layer() -> ParseLayer {
    let pts: [i64; 21] = [
        1000, 1200, 1000, 1200, 350, 600, 400, 600, 390, 398, 260, 100, 250, 150, 250, 150, 250,
        140, 130, 138, 80,
    ];
    let segments: Vec<Segment> = pts
        .windows(2)
        .enumerate()
        .map(|(i, w)| {
            let dir = if w[1] > w[0] {
                Direction::Up
            } else {
                Direction::Down
            };
            seg(dir, i * 4, i * 4 + 4, w[0], w[1])
        })
        .collect();
    let closes: Vec<i64> = (0..=segments.last().expect("非空").end_index)
        .map(|i| 100 + (i as i64 % 7) * 3)
        .collect();
    ParseLayer {
        segments: Rc::new(segments),
        merged_bars: Rc::new(bars_from_closes(&closes)),
        ..Default::default()
    }
}

/// 逐段前缀推进出的因果簿（真实增量宿主，保留全部 revision）。
pub(super) fn causal_book_over_prefixes(layer: &ParseLayer, cfg: &ThetaConfig) -> TowerCache {
    let mut cache = TowerCache::new();
    for n in 1..=layer.segments.len() {
        let prefix = ParseLayer {
            segments: Rc::new(layer.segments[..n].to_vec()),
            merged_bars: Rc::clone(&layer.merged_bars),
            ..Default::default()
        };
        classify_incremental(&prefix, cfg, &mut cache, &[]);
    }
    cache
}

/// 业务载荷投影相等 —— 口径与字段表的唯一来源是
/// [`cand_event::CandidateProjection`]（钟与 revision 计数属生命史，不入等价比较）。
pub(super) fn payload_eq(a: &cand_event::CandidateEvent, b: &cand_event::CandidateEvent) -> bool {
    a.projection() == b.projection()
}
