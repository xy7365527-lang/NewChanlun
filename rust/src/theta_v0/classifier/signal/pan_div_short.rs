//! 盘背短差观测通道消费入口（#1176 B01 判据块自 `signal.rs` 迁出，零行为）。
//!
//! 承载 [`PanDivShortRetraceObservation`] 投影、通道词汇映射（[`t3_in_c_grade_reason_to_pan_div_subtype`]
//! / [`pan_div_subtype_channel_reason`]）与消费入口 [`drain_pan_div_short_retrace_observations`]。
//! 消费面经 `signal` 重导出保持原路径。

use super::*;

// ═══════════════════════════════════════════════════════════════════════════
// 短差档→盘背观测通道消费入口（票 #639；#575 消费方接线 3/3；与上方 #606 S1
// 一类点分级 sidecar **位置同构、非管线同构**——判败/分级事件源紧邻放置，通道词汇
// [`T3InCGrade`]/[`T3InCGradeReason`] 与投影逻辑共处一处，不另起模块；只交消费入口本体
// （本函数），不起 collector/summary/runner 骨架——`RetraceLedger` 尚无驱动源，骨架无
// 血肉即死代码，collector 形态归 #575 驱动票按当时真实驱动需要决定）
// ═══════════════════════════════════════════════════════════════════════════

/// 通道亚型词汇 [`T3InCGradeReason`] → 短差档亚型签 [`PanDivSubtype`] 对拍表（票 #639，固定、
/// **映射点上的一一对应**，**同名非同谓词**）。
///
/// 天然映射只有一点：`T3InCGradeReason::RetestReentered ↔ PanDivSubtype::RetestReentered`——
/// 两侧共用同一英文名描述「回抽重回核心」这一几何直觉，但判定域完全不同，**不是**同一谓词的
/// 两个别名：
/// - 通道侧：判定对象 = `last_center` 右边固定首对 leave/retest **段**（[`Segment`] 坐标系，
///   [`t3_in_c_fixed_first_pair`]），谓词 = 该固定首对 retest 端点是否重回该框核心（#606 D1，
///   037:18 T3-in-c 分级）；
/// - 账本侧：判定对象 = [`RetraceKey`]（中枢四边快照 [`super::super::retrace_ledger::CenterFrame`] +
///   departure），谓词 = leave/retest **候选窗口**回抽是否重回该中枢快照框（027:16 判败四行
///   语义，`RetraceLedger::observe` 三态判案）。
///
/// 短差档只消费竞选失败一族——引擎改口（`NotConstitutedReason::CenterRebased`）走 S2 警报桶
/// 不进短差门户（[`PanDivSubtype`] 文档），故账本侧目前只产一种亚型签；通道侧其余四桶
/// （`MissingLeave`/`MissingRetest`/`SameDirection`/`LeaveNotOutside`）描述的是「固定首对压根
/// 未构成」的结构性缺席——账本侧候选已由适配器注册期挡过残废四桶，短差档见到的必是已配对
/// 成功、只是回抽判败的候选，没有对应判案。本函数对这四桶如实返回 `None`，不造伪对应——**正向
/// 表（本函数）四桶 `None` 是对拍完备性的诚实立档，不是「五桶皆双向对应」**；生产折入通道词汇
/// 走的是反向投影 [`pan_div_subtype_channel_reason`]（恒 `Some`），「一一对应」准确所指是
/// `RetestReentered` 这一个映射点上两侧谓词的对应，不是两个五元枚举整体的双射。
pub(crate) fn t3_in_c_grade_reason_to_pan_div_subtype(
    reason: T3InCGradeReason,
) -> Option<PanDivSubtype> {
    match reason {
        T3InCGradeReason::RetestReentered => Some(PanDivSubtype::RetestReentered),
        T3InCGradeReason::MissingLeave
        | T3InCGradeReason::MissingRetest
        | T3InCGradeReason::SameDirection
        | T3InCGradeReason::LeaveNotOutside => None,
    }
}

/// 反向投影：短差档记录携带的 [`PanDivSubtype`] → 通道词汇 [`T3InCGradeReason`]（供
/// [`PanDivShortRetraceObservation::from_record`] 折入观测记录时打通道标签）。与上一函数互为
/// 部分逆——[`PanDivSubtype`] 目前只有一个变体，故本方向恒 `Some`，无需 `Option`；新增变体是
/// 显式的破坏性改动（同 [`PanDivSubtype`] 文档「固定」纪律），届时本函数不再穷尽会编译失败。
pub(crate) fn pan_div_subtype_channel_reason(subtype: PanDivSubtype) -> T3InCGradeReason {
    match subtype {
        PanDivSubtype::RetestReentered => T3InCGradeReason::RetestReentered,
    }
}

/// 短差档→盘背观测通道投影记录（票 #639）：[`ShortRetraceRecord`] 判败盘背观测折入通道词汇
/// 后的载荷。与 [`FirstClassGradeRecord`] 同构（事件→通道记录的投影形状一致），但源不同——
/// 本记录源自账本判败事件（[`RetraceLedger::short_retrace_records`]），[`FirstClassGradeRecord`]
/// 源自 `judge_segment` 热路径 T3-in-c 分级；两条产线不合并，见
/// [`t3_in_c_grade_reason_to_pan_div_subtype`] 文档「同名非同谓词」。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PanDivShortRetraceObservation {
    pub identity: RetraceKey,
    pub side: RetraceSide,
    pub subtype: PanDivSubtype,
    /// 对拍后的通道词汇标签（[`pan_div_subtype_channel_reason`]）。
    pub channel_reason: T3InCGradeReason,
    pub retest_end: RetracePoint,
    pub judged_as_of: usize,
}

impl PanDivShortRetraceObservation {
    fn from_record(record: &ShortRetraceRecord) -> Self {
        Self {
            identity: record.identity,
            side: record.side,
            subtype: record.subtype,
            channel_reason: pan_div_subtype_channel_reason(record.subtype),
            retest_end: record.retest_end,
            judged_as_of: record.judged_as_of,
        }
    }
}

/// 盘背短差观测通道消费入口（票 #639，#575 消费方接线 3/3）：把 [`RetraceLedger`] 的判败盘背
/// 记录经 [`ShortRetracePortal`] 三锁增量消费后折入通道词汇。与仓内既有 pan_div 生产主链
/// （`locate_pan_div_structure` → `PanDivCert` → `strategy::oscillation::PanDivTrigger`——
/// #292 后已降格为可选辅助，`backtest::fill::step_center_oscillation` 不再消费它，见
/// `fill.rs:761`；中枢震荡交易语义改由次级别买卖点驱动，产出
/// `strategy::center_oscillation_trade::CenterOscillationTrigger`，024:36/46 上沿减/下沿补语义
/// 落在该链末端）**零数据/控制流连接**——两条产线并行，互不知情。
///
/// **驱动源裁定**（编排者 2026-07-29 裁定 B，与 #637 裁定 3A 同口径）：`RetraceLedger` 的生产
/// 驱动源（喂真实 replay 判败事件）归 #575 后续票；票面「生产调用点 ≥1（grep 可证）」的达标
/// 口径 = 类型 `ShortRetraceRecord`/`ShortRetracePortal` 出现于生产代码即达标（裁定 B / #637
/// 裁定 3A 同口径），本函数是该类型出现的生产代码位置之一（非测试专用），不是「本函数已被
/// 生产调用方驱动」意义上的达标——全窗驱动挂点（接一个流经 replay 循环的 `RetraceLedger`
/// 实例）留给后续票，本函数不因此改为占位。
///
/// **消费顺序（三锁在接线面复验）**：先 `portal.sync(ledger)`（锁二/三：账本判败集合↔门户
/// 一一对应、账平）补齐/订正门户内容，再逐身份 `portal.consume`（锁四：幂等消费）——已消费
/// 身份被 `filter_map` 自然滤除。故本函数天然幂等：同一账本/门户对上重复调用，观测集合只在
/// 账本新增判败身份时增长，不会对同一身份重复产出观测。账平（锁三）由
/// [`ShortRetracePortal::balances_with`] 在门户内部把关，本函数只走 `sync`/`consume` 两条既有
/// 公开入口，不重造锁。**键唯一（锁一，[`ShortRetracePortal::admit`]）在本函数路径上结构性
/// 空转**：`sync` 只做 `BTreeMap` 身份键 `insert` 订正（同身份覆盖，非冲突），本函数从不调用
/// `admit`，故这条路径上不可能触发 `DuplicateIdentity`——「键冲突拒绝在接线面复验」由旁证测试
/// （另起 `admit` 手工调用）验证锁本身未被破坏，不是「本函数会拒绝键冲突」。
///
/// **复杂度**：每次调用 = `sync`（全量遍历 `ledger.short_retrace_records()`，O(账本判败条目数)）
/// + 对该全量集合逐条 `consume`——是**全量重扫**，不是增量扫描。账本判败条目数不会随调用次数
/// 缩减，若后续票把本函数接进逐帧 replay 循环，总复杂度是 O(账本条目数 × 调用次数)；接入前应
/// 评估是否需要改为增量读（只 `sync` 新增身份）。
///
/// **与失败处置通知面的判重耦合**（缺口登记，非本函数职责但接线者须知）：本函数的
/// `portal.consume` 与 [`ShortRetracePortal::disposal_notices`] 共享同一 `consumed` 判重集——
/// 若通知面未来接到与本函数**同一个 `ShortRetracePortal` 实例**上，本函数每 drain 一条，
/// `disposal_notices` 就少一条；本函数先跑则通知面恒空。当前无实害（通知面无消费方），但
/// 接通知面时须给两个面各自独立的门户实例，或给 `disposal_notices` 另设判重。
pub(crate) fn drain_pan_div_short_retrace_observations(
    ledger: &RetraceLedger,
    portal: &mut ShortRetracePortal,
) -> Vec<PanDivShortRetraceObservation> {
    portal.sync(ledger);
    ledger
        .short_retrace_records()
        .iter()
        .filter_map(|record| portal.consume(&record.identity))
        .map(|record| PanDivShortRetraceObservation::from_record(&record))
        .collect()
}
