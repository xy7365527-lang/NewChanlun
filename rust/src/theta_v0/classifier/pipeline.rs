//! Θ_level + Θ_signal 分类主管线：级别态 / 分类输出 / 单元⇄线段互逆规约 /
//! canonical 中枢扫描 / 逐级递归构造（`classify` · `classify_with_tower` 单一来源）。
//!
//! 契约锚与铁律见 [`super`] 模块头。

use super::sublevel::extract_second_for_level;
use super::*;

/// 单级别分类状态（R6 态 + 走势类型 + 中枢 + 买卖点）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LevelState {
    /// 该级别的走势类型分解 C_ℓ = B₁⊕…⊕B_k（PDF §6，task #143）：maximal 同向趋势块/盘整块
    /// 真序列（替代旧 AllTrend 全链裁决的 ≤1 元素投影）。链尾块 Active = Q8 CurrentMove；
    /// 块携方向（econ_positive 旧注释抱怨的 MoveKind 丢方向由此消解）。
    pub moves: Vec<MoveBlock>,
    /// ★A1（07c/08）：`Rc` 共享——增量塔 `lc.centers` 经 `Rc::clone`（O(1)）投影到 LevelState，
    /// 消除 per-bar per-level 全量 `centers.clone()`（A0 profile 坐实 08 段 1M=1.58s）。前缀不可变，
    /// 尾部经 `Rc::make_mut` 追加（caller 逐 bar drop 上轮 Classification ⟹ strong_count==1 ⟹ 原地
    /// O(tail)；对拍 harness 跨 bar 持有 ⟹ 写时复制退化全拷，仍 bit-exact，生产路径不受影响）。
    pub centers: Rc<Vec<Center>>,
    /// 与 `centers` 1:1 的 `B_p/c_p` 生命周期对象；保存首个 non-extension 离开单元，并随每个
    /// 新同级递归单元从 Pending 单调推进到 Closed。事件确认快照不在这里回填。
    pub cp_ownership: Rc<Vec<CpScanOwnership>>,
    /// 各买卖点条目（非互斥 bit-vector + 结构止损价 single source，BSP.lean + reference:46）。
    ///
    /// 路 B（Lead 接口契约裁定）：每个 `BspPoint` 携带 pivot_low/pivot_high/center——
    /// strategy 直接构造 `StopInput`，**不**从 bars 重算结构 pivot。classifier 是结构
    /// 止损价的唯一来源（识别买卖点时已定位 pivot，避免两处结构逻辑漂移）。
    /// ★A1（07c）：`Rc` 共享——memo 命中经 `Rc::clone`（O(1)）投影到 LevelState，消除 per-bar
    /// per-level 全量 `cached_bsp.clone()`（A0 profile 坐实 07c 段 1M=1.80s）+ miss 路径 `b.clone()`。
    pub bsp: Rc<Vec<BspPoint>>,
    /// ★Q4（task #145）：该级盘整背驰证书（`signal::PanDivCert`，与 bsp 同一 extract 调用产出、
    /// 同 memo 键缓存）。**不是买卖点**（零 six-bit，不冒充 B1/S1）——承接路由在 econ 统计层
    /// （collect_signals 走 Nest/XZD 二通道，两门皆闭诚实丢弃）。
    pub pan_div: Rc<Vec<signal::PanDivCert>>,
    /// #110 投影层（SPEC #109 expand 第一票）。门关（默认）= `None`（零开销，bit-exact 不变）；
    /// 门开 = stamping 路径构造 [`projection::LevelProjectionLayer`]（`bsp` 同 `Rc` O(1) 共享 +
    /// 单趟跨度扫描 + T2 (#171) 单趟三元锚索引构建，T1 供给线同源）。只描述不判定
    /// （禁第二查法）——零判定消费。
    pub level_projection: Option<projection::LevelProjectionLayer>,
}

/// 多级别递归分类输出（L0..Lmax；某层自然终止则该层及以上为空）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Classification {
    /// 索引 = 级别 ℓ（0=L0=1分钟线段账本）。
    pub levels: Vec<LevelState>,
}

/// 把 L0 线段规约为携带方向的走势单元（契约锚 `Origin.ChanlunElements.Segment` + `CenterConstruction.segHigh/segLow`）。
///
/// L0 单元 = parser 线段（**含方向**，完整判据 `DirAlternates` 的输入）；`[lo,hi]` 对齐
/// `Origin.CenterConstruction.segHigh/segLow`（向上段 hi=端价/向下段 lo=端价已规约为区间）。
pub(super) fn segment_to_unit(seg: &Segment) -> UnitRange {
    let (lo, hi) = if seg.start_price <= seg.end_price {
        (seg.start_price, seg.end_price)
    } else {
        (seg.end_price, seg.start_price)
    };
    UnitRange {
        start_index: seg.start_index,
        end_index: seg.end_index,
        direction: seg.direction,
        lo,
        hi,
    }
}

/// 级别-N 输入单元 → `Segment`（`segment_to_unit` 的逆，端点价按 `fold_direction` 取 hi/lo）。
///
/// codex-decide-20260703 裁定 A 的最大实现风险点（end_price 忠实性，须单测）：级别-N 的「线段」
/// 角色由输入单元 [`UnitRange`] 承担——向上单元起点=lo/终点=hi（`seg_end` 取 end_price=hi）；
/// 向下单元起点=hi/终点=lo。与 [`segment_to_unit`] 互逆：L0 段经 `segment_to_unit` → `unit_to_segment`
/// round-trip bit-exact（向上段 start<end ⟹ lo=start/hi=end ⟹ 还原 (start,end)；向下段镜像）。
/// `start_index/end_index` 是 source_index（原始 K 序）——A/C 段 MACD 面积经 close_src 映射用同坐标系。
pub(super) fn unit_to_segment(u: &UnitRange) -> Segment {
    let (start_price, end_price) = match u.direction {
        Direction::Up => (u.lo, u.hi),
        Direction::Down => (u.hi, u.lo),
    };
    Segment {
        direction: u.direction,
        start_index: u.start_index,
        end_index: u.end_index,
        start_price,
        end_price,
    }
}

/// 级别-N 一/三类买卖点提取（codex-decide-20260703 裁定 A：级别-N 直接判定，非 L0 relabel）。
///
/// 把级别-N 输入单元 `units`（承担「线段」角色）还原为 `Segment` 后**复用 L0 的
/// [`signal::extract_signals_with_hist`]**——同一套逻辑，只换输入算子（is_l0 分支消失于领域层）：
/// - **趋势门控**：内部 `decompose(centers)` 局部趋势门（Q1/Q8，task #143）与本级 [`classify_level`]
///   同一 `decompose` 单一来源 ⟹ 「一类只在该级当前趋势块内产」忠实。
/// - **A/C 段力度**：内部 `AbcDivergence`/`locate_trend_seg_a` 复用 divergence.rs 面积原语（与
///   `sublevel_diverges` 同族的 `segment_macd_area`/`is_divergence`）——**禁第二套力度引擎**满足。
/// - **三类**：`judge_third` 在级别-N units（外缘区间）+ centers（几何中枢）的离开/回试关系上判定。
///
/// ★force_state 生产热路由（beta-route #115）：传真 `dif/closes_tick`（与 L0 层同源，L0 唯一可达
/// close 序列，级别-N A/C 段经 source_index 坐标映射同坐标系）⟹ 级别-N 一类趋势背驰候选的
/// `point.force` 亦算得 `Some`（5 proxy），进 selector force_state 第 8 维。二/三类 force=None。
pub(super) fn extract_first_third_for_level(
    centers: &[Center],
    units: &[UnitRange],
    anchors: &[Option<Direction>],
    hist: &[f64],
    dif: &[f64],
    closes_tick: &[Tick],
    close_src: &[usize],
    gauge: divergence::DivergenceGauge,
) -> (Vec<BspPoint>, Vec<signal::PanDivCert>) {
    let segs: Vec<Segment> = units.iter().map(unit_to_segment).collect();
    // ★Q7-#1 裁定C（codex-q7-fallback-20260703，收窄 #121 裁定A）：`anchors[i]` = 产生本级 units
    // 的下级 blocks 的 ownership 方向（`center_own_dir_at` 同一来源，与 `project_to_units` 方向
    // 派生锁步）。None = endpoint fallback 单元——保留为序列/区间/面积成员，不作一/三类方向锚。
    // ★p117（686 窄域授权，终端背书裁定 T2）：第一类方向锚经 p117 降级为单元结构方向
    // （`judge_segment` 内消费 `anchors_self`）；`anchors` 实参仍须传——三类 leave 锚与
    // CandDelta 诊断 provider 仍在消费 provenance 锚。
    signal::extract_signals_with_hist_anchored(
        centers, &segs, Some(anchors), hist, dif, closes_tick, close_src, gauge,
    )
}

/// 从 L0 线段单元序列识别 canonical 中枢序列（**完整判据** seed + 延伸吸收，契约锚
/// `Origin.CenterComplete.CenterConfirmedComplete` + 第20课中心定理一）。
///
/// L0 线段有内在方向 ⟹ seed 用 `center::center_from_segments`（完整判据：方向交替 ∧ 全三段核心
/// 非空，口径 B——第三段贯穿已被全三段核心非空吸收，637号 codex L0 等价）。seed 成立后进入延伸
/// 吸收（中心定理一：后续段区间触及 [ZD,ZG] ⟹ 同一中枢延伸，task #142），仅 non-extension
/// （`d_j>ZG ∨ g_j<ZD`）终止；seed 不成立则前进一段继续找。算法单一来源 = `recursive_tower::
/// detect_centers_windowed_resume`（全量/增量同一扫描，bit-exact 定义性）。
pub(super) fn detect_centers_complete(units: &[UnitRange]) -> Vec<Center> {
    detect_centers_with(units, center::center_from_segments)
}

/// 从上级走势单元序列识别 canonical 中枢序列（**几何路径** seed + 延伸吸收，契约锚
/// `Origin.centerHolds` + 三段共同重叠 + 第20课中心定理一）。
///
/// 上级单元是中枢外缘区间（**无内在缠论方向**，方向由 Move 趋势裁决携带）⟹ seed 用
/// `center::center_from_window`（几何判据：全三段核心非空，口径 B——第三段贯穿已吸收，637号；无方向交替）。
/// 延伸吸收与 L0 同一几何判据（区间触及 [ZD,ZG]，task #142）。上级发展裁决用
/// `Origin.CenterStates.classifyDevelopment`（外缘判据，无方向交替要求）——见 `center.rs`
/// 诚实有效域声明。
pub(super) fn detect_centers_geometric(units: &[UnitRange]) -> Vec<Center> {
    detect_centers_with(units, center::center_from_window)
}

/// canonical 中枢扫描（seed + 延伸吸收 + non-extension 终止）——**单一来源委托**
/// `recursive_tower::detect_centers_windowed_resume`（全量 = `start_i=0`；增量塔走同一函数的
/// resume 路径 ⟹ T^inc == T^full 定义性成立，非对拍性成立）。
pub(super) fn detect_centers_with(
    units: &[UnitRange],
    build: fn(&UnitRange, &UnitRange, &UnitRange) -> Option<Center>,
) -> Vec<Center> {
    recursive_tower::detect_centers_windowed_resume(units, build, 0)
        .0
        .into_iter()
        .map(|(c, _)| c)
        .collect()
}

/// 把一级走势单元序列规约为该级走势类型分解 + 中枢（PDF §6，task #143，替代旧 AllTrend
/// `classifyMove` 全链裁决——吸收锁死谓词，见 decompose.rs 模块头）。
///
/// `is_l0`：L0 用完整判据（方向交替），上级用几何路径（外缘）。返回 `(中枢序列, 分解块序列)`。
pub(super) fn classify_level(units: &[UnitRange], is_l0: bool) -> (Vec<Center>, Vec<MoveBlock>) {
    let centers = if is_l0 {
        detect_centers_complete(units)
    } else {
        detect_centers_geometric(units)
    };
    let blocks = decompose(&centers);
    (centers, blocks)
}

/// Θ_level + Θ_signal 分类内部实现（`classify` / `classify_with_tower` 共享单一来源）。
///
/// `tower_snapshots[i]` = 处理第 i 级时 `compose_level` 前的 `moves_tower` 快照，下标与
/// `Classification.levels` 同构（`tower_snapshots.len() == levels.len()`）：
/// - 索引 0（L0 级）：全 `RMove::Segment`（递归底，`sub_moves` 空）。
/// - 索引 ≥1（L(k) 级）：前一级产出的 `RMove::Compose` 序列（携次级别 subs，depth≥1 真嵌套）。
pub(super) fn classify_impl(l0: &ParseLayer, config: &ThetaConfig) -> (Classification, Vec<Rc<Vec<LeveledMove>>>) {
    let min_parts = config.level.min_parts_per_level as usize;
    let l_max = config.level.l_max as usize;

    // L0 输入单元 = parser 线段账本（reference:29 L0=1分钟线段账本）。
    let mut units: Vec<UnitRange> = l0.segments.iter().map(segment_to_unit).collect();
    // Q7-#1 裁定C：units 的方向锚资格（与 units 同步循环携带；L0 分支不消费，级别-N 在投影点派生）。
    let mut units_anchors: Vec<Option<Direction>> = Vec::new();

    // 空 L0：无可构造级别（自然终止于 L0 之前）。
    if units.is_empty() {
        return (Classification::default(), Vec::new());
    }

    // ★递归塔对象（#53 升级，still-MISSING-塔解除）：L0 走势单元 = 携坐标的 `RMove::Segment`
    // （`LeveledMove`，递归底 level 0）。旧塔把每级走势单元折叠为无 subs 的 `UnitRange`，
    // `extract_second_signals`（消费 `RMove::Compose` 的 descend 取回次级别走势）永产不出 B2/S2。
    // 新塔每级走势单元携次级别走势 subs（`RMove::Compose`）+ source_index 坐标 ⟹ B2/S2 真可产。
    // ★O(n) 重构：moves_tower/snapshots 用 Rc（与增量版同返回类型 `Vec<Rc<Vec<LeveledMove>>>`，
    // bit-exact 测试逐字段比较 *rc）。全量版非 per-bar 热点（O(n) 单趟），Rc 仅为类型对齐。
    let mut moves_tower: Rc<Vec<LeveledMove>> = Rc::new(
        units
            .iter()
            .enumerate()
            .map(|(i, u)| LeveledMove::from_unit(u, ElementId { level: 0, ordinal: i as u64 }))
            .collect(),
    );

    // 第一类背驰 MACD：closes/close_src 在全递归层共享（L0 唯一可达 close 序列；上级走势的
    // 次级别 close 区间由 source_index 坐标定位，见 macd 接入点）。
    let closes: Vec<f64> = l0.merged_bars.iter().map(|b| b.close as f64).collect();
    let close_src: Vec<usize> = l0.merged_bars.iter().map(|b| b.source_index).collect();
    // ★force_state 生产热路由（beta-route #115）：dif（黄白线）+ closes_tick（整数 close）供一类候选
    // A/C 段 5 proxy（DIF 峰/振幅/速度）。hist/dif 同一 compute_macd 单趟产出（无额外 O(n) 扫描）。
    let series = divergence::compute_macd(&closes, &config.macd);
    let hist = series.hist;
    let dif = series.dif;
    let closes_tick: Vec<Tick> = l0.merged_bars.iter().map(|b| b.close).collect();

    let mut levels: Vec<LevelState> = Vec::new();
    let mut tower_snapshots: Vec<Rc<Vec<LeveledMove>>> = Vec::new();

    // 递归级别构造：每级由下级走势单元构造（L0 直接是线段单元，从 L0 开始裁决）。
    for level_idx in 0..=l_max {
        // 自然终止（reference:30）：某层无 ≥min_parts 完成部件 ⟹ 无法产生完整走势，停止。
        if units.len() < min_parts {
            break;
        }

        // 本级输入塔快照（compose_level 前，与 levels[level_idx] 对应——同步 index 不变量）。
        tower_snapshots.push(Rc::clone(&moves_tower));

        // L0（level_idx==0）用完整判据（方向交替，线段有方向）；上级用几何路径（外缘，单元无方向）。
        let is_l0 = level_idx == 0;
        let (centers, moves) = classify_level(&units, is_l0);

        // L(k+1) 走势塔 = 本级窗口化 compose（每中枢的构成三段次级别走势 → 一个上级 `RMove::Compose`，
        // 契约锚 `Origin.RecursiveLevelSystem.composeStep` 窗口封装）。`upper_moves` 是携坐标的上级走势
        // 序列（descend 取回构成它的次级别走势 ⟹ B2/S2 可产），与 `centers` 一一对应。
        let (centers_w, upper_moves, mut cp_ownership) = compose_level(&units, &moves_tower[..], is_l0, level_idx as u32 + 1);
        debug_assert_eq!(centers, centers_w, "compose_level 与 classify_level 中枢序列一致");
        recursive_tower::advance_cp_lifecycles(
            &mut cp_ownership,
            &centers_w,
            &units,
            &moves_tower,
            (!is_l0).then_some(&units_anchors[..]),
            1,
        );

        // BSP 信号提取（reference:34-36）。三层覆盖：
        // - **L0 线段层**（`extract_signals`）：第一类（破中枢几何 L0 ∧ MACD 背驰 L1 真算）+ 第三类
        //   （confirmed 结构几何）。L0 走势单元 = 线段（有方向），第一/三类在线段端点上 bit-exact 判定。
        // - **递归组装层**（`extract_second_signals`，#53 接入）：第二类（B2/S2）由次级别第一类构成
        //   （买卖点定律一 §10.2）。对本级**每个上级走势** `RMove::Compose`，从 descend 取回的次级别
        //   走势序列内识别第二类走势结构（第一类离开 + 回拉不创新低/新高），产 B2/S2。背驰力度由
        //   `divergence_of` 闭包用 `divergence.rs` MACD 真算（次级别走势 close 区间 → 面积比较）。
        let (mut bsp, pan_div): (Vec<BspPoint>, Vec<signal::PanDivCert>) = if is_l0 {
            // ★force_state 生产热路由（beta-route #115）：传真 dif/closes_tick ⟹ 一类候选 point.force
            // = Some（5 proxy），进 selector force_state 第 8 维。结构六 bit 不变（force 不进 class_index/
            // 分桶 key，PartialEq 排除），GOLDEN 因 Debug 含 force 诚实翻转（signal.rs digest guard）。
            signal::extract_signals_with_hist(
                &centers, &l0.segments, &hist, &dif, &closes_tick, &close_src,
                config.divergence_gauge,
            )
        } else {
            // 级别-N 一/三类（codex-decide-20260703 裁定 A）：units 承担线段角色，复用 L0 判据（含 force）。
            extract_first_third_for_level(
                &centers, &units, &units_anchors, &hist, &dif, &closes_tick, &close_src,
                config.divergence_gauge,
            )
        };
        // 递归组装层 B2/S2（#53 接入）：对每个上级走势的次级别走势序列识别第二类结构。
        bsp.extend(extract_second_for_level(&upper_moves, &hist, &close_src));
        bsp.sort_by_key(|p| p.source_index);

        let bsp = Rc::new(bsp);
        // #110 投影层 stamping（机制位关 = None 零开销）。T3 (#172) 并门：本机制位转派生——
        // 层载由链路径是否启用单一驱动（π 入口 `admission::chain_driven_level_projection`
        // 唯一生产写入点，#168 裁定 3）；链活 ⟹ 层必载（含三元锚索引），链死不载。
        // T2 (#171)：三元锚供给（`l0.fractals`/`l0.merged_bars`，ParseLayer `Rc` 共享只读
        // 借用，零拷贝）随门开分支引入——门关分支零新增读。
        let level_projection = if config.level_projection.enabled {
            Some(projection::LevelProjectionLayer::from_level(
                levels.len() as u32,
                &bsp,
                &l0.fractals,
                &l0.merged_bars,
            ))
        } else {
            None
        };
        levels.push(LevelState {
            moves,
            centers: Rc::new(centers.clone()),
            cp_ownership: Rc::new(cp_ownership),
            bsp,
            pan_div: Rc::new(pan_div),
            level_projection,
        });

        // L(k+1) 输入单元 = 上级走势塔的 `UnitRange` 投影（外缘区间 + 坐标 + 外缘趋势方向）。
        // 上级走势携 subs（`RMove::Compose`），投影只为下一级几何中枢检测提供 [lo,hi] 区间——
        // 真递归 subs 在 `moves_tower` 里保留（不丢弃，旧塔丢弃 subs 是 B2 不可产的根因）。
        // Q7（task #145）：方向源 = 本级中枢 ownership 块方向（刚 push 的 LevelState.moves 单一来源）。
        {
            let pb = &levels.last().expect("本级 LevelState 已 push").moves;
            units = project_to_units(&upper_moves, pb);
            // Q7-#1 裁定C：锚资格与投影方向同一 provenance 来源（center_own_dir_at，None=fallback）。
            units_anchors = (0..units.len()).map(|i| decompose::center_own_dir_at(pb, i)).collect();
        }
        moves_tower = Rc::new(upper_moves);

        // 本级无中枢 ⟹ 无上级输入单元，停止递归（自然终止）。
        if units.is_empty() {
            break;
        }
    }

    (Classification { levels }, tower_snapshots)
}

/// Θ_level + Θ_signal 顶层入口（reference-theta-v0.md:27-37）。
///
/// 递归构造 L0..Lmax：L0=parser 线段账本；每级由下级已完成走势单元构造中枢 + 裁决走势，
/// 走势成为上级输入单元。自然终止：某级单元数 < `min_parts_per_level`（无法产生完整走势），
/// 或达 `l_max` 上界。
///
/// ★边界条件：
/// - L0 线段数 < `min_parts_per_level` ⟹ `levels` 仅含 L0（或为空，见下）—— 自然终止。
/// - 任一级走势分解（PDF §6）产出完整块序列进 moves（混合链不再是级别整体退化裁决，
///   task #143——旧 AllTrend 的 HigherCenterCandidate 由多块序列吸收）。
/// - 空 ParseLayer（无线段）⟹ `Classification::default()`（空 levels，无可构造级别）。
pub fn classify(l0: &ParseLayer, config: &ThetaConfig) -> Classification {
    classify_impl(l0, config).0
}

/// 分类 + 逐级塔导出入口（(i) 段导出桥，MEMORY coverage-engine-needs-tower-export-bridge）。
///
/// 返回 `(Classification, Vec<Vec<LeveledMove>>)`：
/// - `Classification`：与 `classify` bit-identical（共用 `classify_impl` 单一来源，原行为不变）。
/// - `Vec<Vec<LeveledMove>>`：逐级走势塔快照（`tower[i]` 对应第 i 级处理的输入塔）：
///   - `tower[0]`：L0 线段层（全 `RMove::Segment`，递归底，`sub_moves` 空）。
///   - `tower[k]`（k≥1）：第 k 级输入塔，含 `RMove::Compose` 携次级别 subs（depth≥1 真嵌套），
///     下游 `descend_leveled` 可遍历次级别走势。
///
/// ★不碰附着映射：本函数只导出塔，不消费 coverage/interp 的附着规则（(ii) 段职责）。
/// ★L0/L1 认识论等级：纯结构导出操作，不依赖经验数据（formalization-validity-domain 231号）。
pub fn classify_with_tower(
    l0: &ParseLayer,
    config: &ThetaConfig,
) -> (Classification, Vec<Rc<Vec<LeveledMove>>>) {
    classify_impl(l0, config)
}
