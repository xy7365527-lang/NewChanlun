//! p118 关④：小转大显式分类分支——旁挂联合分类 `NestTurnClass`（四类 partition，纯只读派生）。
//!
//! 施工图：`chanlun/review-results/p118-xiaozhuanda-branch-design-20260718.md`（形态 D）。
//! 断链点 = 装配环 `nest.rs` rung 级 `divergence_confirmed` 丢弃（已由 sidecar 恢复，
//! 见 [`TypedNestCertificate::confirmed`]）+ 分类词汇无「小转大」值（本文件补齐）。
//! 本层对账本（centers/bsp/证书）**只读**：不过滤、不增产、不改任何既有对象语义。
//!
//! ## 形态语义（教义锚，全部直读主仓 `docs/chanlun/text/blog/` 逐条核对）
//!
//! 小转大 = 「背驰级别小于当下走势级别」的转折（043:32、044:14）：
//! **父级无背驰段**（043:34「明确显示没有出现30分钟的背驰，也就是背驰段最终不成立」）+
//! **低级别有背驰**（044:16「如果c是一个1分钟级别的背驰，最终引发下跌拉回B里」）。
//! 链行为：父级无证书，链顶 = 小级别背驰证书，中间各级无需背驰证书（跳级是原文合法形态）。
//!
//! ## 044:30 纪律（防误标）
//!
//! 「关于这种情况，只有必要条件，而没有充分条件」（044:30）——`XiaozhuandaCandidate`
//! 的语义 = 「**通过必要条件过滤的候选**」，永远不是「小转大确认」。类型层封锁：
//! [`XzdEvidence`] 不含 `BspBits` 成员、不提供 `confirm_side` 方法；候选不能作终端背书、
//! 不能置任何 six-bit、不进 strategy 触发链（编译期构造保证，非注释承诺，测试 T9 钉死）。
//! 四个反例封锁位（全是必要条件的否定式，无一充分断言）：c′ 缺失 ⟹ 非候选；
//! c′ 正常震荡无三类点（044:20）⟹ 非候选；三类点方向反（= 平台下破，066:198）⟹ 非候选；
//! 三类点因果序反（先于基例背驰点）⟹ 非候选。
//!
//! ## c′ 必要条件过滤（044:22-26 的生产对象映射；级别算术 = 实装验证点 W1）
//!
//! 设链顶 rung nest 级 ℓ（其中枢链 = `levels[ℓ-1]`，p117 T1 裁定事实链
//! `nest::event_bsp_book_level` 同源）：
//!
//! 1. **c′ 定位**：`levels[ℓ-2].centers` 中 `[start,end] ⊆ 链顶事件 interval_b`（c 段区间）
//!    且 `end ≤ 基例.turn_source` 的**最后一个**中枢（044:20「c`是c中最后一个5分钟的中枢」
//!    「顶背驰只能出现在c`之后」——因果序 c′.end ≤ 基例.turn）。ℓ<2 ⟹ c′ 落背驰递归
//!    地板之下（065:94 线段以下无背驰、066:198 类背驰域）⟹ `ExecEvidenceOnly`。
//! 2. **三类点命中**：`levels[ℓ-2].bsp` 存在点 p：`p.center == Some(c′)`（3 类点携中枢
//!    契约 `signal::make_third_point` + `bsp.rs` 不变量）、`p.source_index > 基例.turn_source`
//!    （回拉入 c′ 后出三类，044:20）、方向匹配——顶转（side=Short）须 `sell3`、底转
//!    （side=Long）须 `buy3`（044:24/26）。
//! 3. **父级二类点补充**（053:28「在小级别转大级别的情况下，第二类买卖点就是最佳的」，
//!    **不作门**）：`levels[ℓ-1].bsp` `[基例.turn, 三类点 src]` 窗内首个 buy2/sell2 记入
//!    `XzdEvidence.second_class`，供下游操作层消费；有无不影响候选成立。
//!
//! ## p92 `TURN_CLASS` dump 行契约（侧信道只写不判，#98/#99 纪律；CERT/CKPT/门行零改）
//!
//! - 证书行（与 CERT 行同主键配对）：
//!   `TURN_CLASS caliber=<A|B> exec=<e> top=<t> as_of=<bar> ids=<同 CERT 主键：身份向量
//!   高→低含基例，`level:turn_source:start-end` 以 `|` 连接> class=<NestedConfirmed|
//!   XiaozhuandaCandidate|ExecEvidenceOnly> confirmed_vec=<01 串，高→低含基例>
//!   [evidence=c_prime=(zg,zd,start,end);third=<src>;second=<src|->]`（evidence 仅 Candidate）。
//! - 孤儿行（039:34 defer 域，c 破极值未确认 Trend 事件未被任何链消费）：
//!   `TURN_CLASS ids=<单事件身份 level:turn_source:start-end> class=DeferOrphan confirmed_vec=0`。
//! - 每证 / 每个 c 破极值未确认 Trend 事件**恰一行**（partition）；分类是旁挂派生，
//!   证书集合三栏对照（存续=全/失证=0/新增=0）不受本信道影响。

use super::super::types::{Center, Side};
use super::level_view::{NestCandidateEvent, NestDivergenceKind};
use super::nest::{NestEventIdentity, TypedNestCertificate};
use super::Classification;

/// 小转大候选证据（044:22-26 必要条件的命中坐标）——**纯结构坐标**：无 `BspBits` 成员、
/// 无确认语义（044:30 只有必要条件；本结构不能作终端背书、不能置 six-bit、不进触发链）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XzdEvidence {
    /// c′ = 链顶 c 段内最后一个次级别中枢（044:20「c`是c中最后一个5分钟的中枢」）。
    pub c_prime: Center,
    /// c′ 三类点 source_index（044:24/26；严格晚于基例背驰点 = 因果序正）。
    pub third_src: usize,
    /// 父级二类点补充证据（053:28）——`levels[ℓ-1].bsp` `[基例.turn, third_src]` 窗内
    /// 首个二类点 source_index；**不作门**（有无不影响候选成立）。
    pub second_class: Option<usize>,
}

/// nest 链顶背书形态分类（跨级链属性；与六态 r / 信号位 b 正交——r 答「在哪」、b 答
/// 「有什么信号」、本类答「链顶背书形态」）。每证 / 每孤儿事件**恰一类**（partition，
/// 由 [`classify_nest_turns`] 构造保证）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NestTurnClass {
    /// 情况一（043:28/30）：背驰级别 = 走势级别——链顶 rung confirmed=true。
    NestedConfirmed,
    /// 情况二候选（043:32/34 + 044:22-26 必要条件过滤通过）：链顶 confirmed=false ∧
    /// 基例 confirmed=true ∧ c′ 三类点必要条件成立。★044:30：只有必要条件——本值
    /// 语义 = 「通过必要条件过滤的候选」，永远不是「小转大确认」。
    XiaozhuandaCandidate { evidence: XzdEvidence },
    /// 链顶 confirmed=false 且必要条件不成立（含 c′ 正常震荡 044:20 域、方向反、因果序反）、
    /// 或 ℓ<2（c′ 落背驰地板之下，065:94/066:198 类背驰域）——背书证据仅及基例本级，
    /// 诚实判负，不是误杀。
    ExecEvidenceOnly,
    /// 039:34 defer 域：c 破极值的未确认 Trend 事件未被任何链消费（孤儿）——等三段成
    /// 中枢后再比较，挂起观察而非丢弃（defer 时序重判状态机列遗留项，本关只交付分类口径）。
    DeferOrphan { identity: NestEventIdentity },
}

/// 分类账本条目主键：证书 = 身份向量（高→低，含基例；与 p92 CERT 行 `ids` 同构）；
/// 孤儿事件 = 单元素身份向量。
pub type CertKey = Vec<NestEventIdentity>;

/// 单证书分类（纯，只读）：按 sidecar `confirmed[0]`（链顶）分流——true ⟹
/// `NestedConfirmed`（043:28/30 情况一）；false ⟹ c′ 必要条件过滤，过 ⟹
/// `XiaozhuandaCandidate`、不过 ⟹ `ExecEvidenceOnly`。
pub fn classify_certificate_turn(
    certificate: &TypedNestCertificate,
    classification: &Classification,
) -> NestTurnClass {
    let confirmed = certificate.confirmed();
    let identities = certificate.identities();
    // 构造保证（`assemble_typed_certificate` 播种基例 + 逐级同位 push）：confirmed /
    // identities 非空且等长。防御性诚实判负，只读层不 panic。
    let (Some(&top_confirmed), Some(top), Some(base)) =
        (confirmed.first(), identities.first(), identities.last())
    else {
        return NestTurnClass::ExecEvidenceOnly;
    };
    if top_confirmed {
        return NestTurnClass::NestedConfirmed;
    }
    match xzd_evidence(
        classification,
        certificate.certificate().side(),
        top.level,
        top.interval_b,
        base.turn_source,
    ) {
        Some(evidence) => NestTurnClass::XiaozhuandaCandidate { evidence },
        None => NestTurnClass::ExecEvidenceOnly,
    }
}

/// c′ 必要条件过滤（044:22-26；三个要件全是必要条件的肯定式核验，任一不成立即判负）。
/// 命中 ⟹ `Some(evidence)`；级别算术 `levels[ℓ-2]` = 实装验证点 W1（测试钉死）。
fn xzd_evidence(
    classification: &Classification,
    side: Side,
    top_level: u32,
    top_interval: (usize, usize),
    base_turn: usize,
) -> Option<XzdEvidence> {
    // ℓ<2 ⟹ c′ 落背驰递归地板之下（065:94 线段以下无背驰、066:198 类背驰域）。
    let book_level = (top_level as usize).checked_sub(2)?;
    let book = classification.levels.get(book_level)?;
    // 要件 1：c′ 定位——[start,end] ⊆ 链顶 interval_b（c 段）∧ end ≤ 基例.turn 的最后一个
    // 中枢（044:20「c`是c中最后一个5分钟的中枢」「顶背驰只能出现在c`之后」）。
    // `max_by_key` 平局取迭代序末个（确定性）；「最后」按 end_index 最大。
    let c_prime = *book
        .centers
        .iter()
        .filter(|center| {
            center.start_index >= top_interval.0
                && center.end_index <= top_interval.1
                && center.end_index <= base_turn
        })
        .max_by_key(|center| center.end_index)?;
    // 要件 2：三类点命中——p.center == Some(c′)（3 类点携中枢契约）∧ src > 基例.turn
    //（回拉入 c′ 后出三类，044:20）∧ 方向匹配（顶转须 sell3、底转须 buy3，044:24/26；
    // 方向反 = 平台下破 066:198 ⟹ 非候选）。最早命中（`min_by_key` 平局取迭代序首个）。
    let third = book
        .bsp
        .iter()
        .filter(|point| {
            // #218 面 A 载体形态机械适配：三类点恒 Center 载体（语义不变，全字段等式照旧）。
            point.center == Some(super::bsp::OwnerRef::Center(c_prime))
                && point.source_index > base_turn
                && match side {
                    Side::Short => point.bits.sell3,
                    Side::Long => point.bits.buy3,
                }
        })
        .min_by_key(|point| point.source_index)?;
    // 要件 3（补充证据，不作门）：父级二类点（053:28）——`levels[ℓ-1].bsp`
    // `[基例.turn, 三类点 src]` 窗内首个 buy2/sell2。
    let second_class = classification
        .levels
        .get(top_level as usize - 1)
        .and_then(|parent_book| {
            parent_book
                .bsp
                .iter()
                .filter(|point| {
                    base_turn <= point.source_index
                        && point.source_index <= third.source_index
                        && match side {
                            Side::Short => point.bits.sell2,
                            Side::Long => point.bits.buy2,
                        }
                })
                .min_by_key(|point| point.source_index)
                .map(|point| point.source_index)
        });
    Some(XzdEvidence {
        c_prime,
        third_src: third.source_index,
        second_class,
    })
}

/// 039:34 defer 孤儿谓词：c 破极值的未确认 Trend 事件且未被任何证书身份向量覆盖。
///
/// 「c 破极值」由 provider 门 extreme 预滤（`level_view.rs` Trend 分支）保证——未破极值的
/// 形态在事件层已被击杀，属 037:20 否则条款域（盘整/二类点通道），**不进入**本分支；
/// 故事件层在册的未确认 Trend 事件即「c 破极值未确认」（防误标第一防线，施工图 §1.4/§2.5）。
pub fn is_defer_orphan_event(event: &NestCandidateEvent, covered: bool) -> bool {
    event.kind == NestDivergenceKind::Trend && !event.divergence_confirmed && !covered
}

/// 批量联合分类（纯，只读）：每张证书按 `confirmed[0]` 分流恰一类；事件层未被任何证书
/// 身份向量覆盖的 c 破极值未确认 Trend 事件 ⟹ `DeferOrphan`——每证 / 每孤儿事件恰一类，
/// 互斥穷尽（partition 由构造保证，测试 T8 钉死）。
pub fn classify_nest_turns(
    events_by_level: &[Vec<NestCandidateEvent>],
    certificates: &[TypedNestCertificate],
    classification: &Classification,
) -> Vec<(CertKey, NestTurnClass)> {
    let mut out = Vec::with_capacity(certificates.len());
    let mut covered: Vec<NestEventIdentity> = Vec::new();
    for certificate in certificates {
        covered.extend_from_slice(certificate.identities());
        out.push((
            certificate.identities().to_vec(),
            classify_certificate_turn(certificate, classification),
        ));
    }
    for events in events_by_level {
        for event in events {
            let identity = NestEventIdentity::of(event);
            if is_defer_orphan_event(event, covered.contains(&identity)) {
                out.push((vec![identity], NestTurnClass::DeferOrphan { identity }));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::bsp::BspPoint;
    use super::super::nest::{assemble_typed_certificate, NestIntervalCaliber};
    use super::super::LevelState;
    use super::*;
    use crate::theta_v0::types::{BspBits, Tick};
    use std::rc::Rc;

    // ───────── 夹具（沿用 nest.rs 测试模式：手工事件 + 假账本，坐标全部 source_index 同系）─────────

    fn typed_event(
        level: u32,
        side: Side,
        kind: NestDivergenceKind,
        interval_b: (usize, usize),
        interval_a: (usize, usize),
        turn_source: usize,
        judge_at: usize,
        divergence_confirmed: bool,
    ) -> NestCandidateEvent {
        NestCandidateEvent {
            level,
            side,
            kind,
            seg_a: interval_b,
            interval_b,
            interval_a,
            divergence_confirmed,
            turn_source,
            judge_at,
            provider_window: interval_a,
            intake_fallback: false,
            // 关③ P3 新字段：本模块测试的 terminal_of 均为自带闭包，不读事件 B 身份快照。
            b_center_start: 0,
        }
    }

    fn sell1_bits() -> BspBits {
        let mut t = BspBits::default();
        t.sell1 = true;
        t
    }

    fn buy1_bits() -> BspBits {
        let mut t = BspBits::default();
        t.buy1 = true;
        t
    }

    fn sell2_bits() -> BspBits {
        let mut t = BspBits::default();
        t.sell2 = true;
        t
    }

    fn sell3_bits() -> BspBits {
        let mut t = BspBits::default();
        t.sell3 = true;
        t
    }

    fn buy3_bits() -> BspBits {
        let mut t = BspBits::default();
        t.buy3 = true;
        t
    }

    fn center(
        zd: Tick,
        zg: Tick,
        dd: Tick,
        gg: Tick,
        start_index: usize,
        end_index: usize,
    ) -> Center {
        Center { zd, zg, dd, gg, start_index, end_index }
    }

    /// `BspPoint` 夹具：`center` 按 `make_third_point` 契约填（3 类点必 `Some`，bsp.rs 不变量；
    /// #218 面 A 载体形态：Center 变体包装）。
    fn pt(source_index: usize, bits: BspBits, center: Option<Center>) -> BspPoint {
        BspPoint { source_index,
            bits,
            pivot_low: 0,
            pivot_high: 0,
            center: center.map(crate::theta_v0::classifier::bsp::OwnerRef::Center),
            struct_break_dir: None,
            force: None,
        }
    }

    fn level(centers: Vec<Center>, bsp: Vec<BspPoint>) -> LevelState {
        LevelState {
            centers: Rc::new(centers),
            bsp: Rc::new(bsp),
            ..Default::default()
        }
    }

    fn ledger(levels: Vec<LevelState>) -> Classification {
        Classification { levels }
    }

    // ───────── 044:16 形态几何公共件（施工图 §5）─────────
    //
    // 父事件（level=2，Short，divergence_confirmed=false，interval_b=(33,50)，turn=50）；
    // 基例（level=1，Short，confirmed=true，interval_b=(44,48)，turn=48）；
    // levels[1].centers = A[100,200](0-12)、B[150,250](20-32)；
    // levels[0].centers 含 c′[420,440](40-46) ⊂ (33,50)，end=46 ≤ 基例.turn=48（因果序）。
    // 链 (top=2, exec=1) 成证，confirmed 向量 = [false, true]（高→低）。

    fn xzd_base_event() -> NestCandidateEvent {
        typed_event(1, Side::Short, NestDivergenceKind::Trend, (44, 48), (36, 48), 48, 48, true)
    }

    fn xzd_parent_event(confirmed: bool) -> NestCandidateEvent {
        typed_event(2, Side::Short, NestDivergenceKind::Trend, (33, 50), (25, 50), 50, 50, confirmed)
    }

    fn c_prime() -> Center {
        center(420, 440, 410, 450, 40, 46)
    }

    /// levels[1]：父级（30 分）中枢链 A/B（044:16 的 a+A+b+B+c 之 A、B；c′ 查法不读本层）。
    fn parent_book_centers() -> Vec<Center> {
        vec![
            center(100, 200, 90, 210, 0, 12),
            center(150, 250, 140, 260, 20, 32),
        ]
    }

    fn xzd_events() -> Vec<Vec<NestCandidateEvent>> {
        vec![vec![], vec![xzd_base_event()], vec![xzd_parent_event(false)]]
    }

    fn xzd_certificate(events: &[Vec<NestCandidateEvent>]) -> TypedNestCertificate {
        assemble_typed_certificate(
            events,
            &events[1][0],
            2,
            NestIntervalCaliber::B,
            &|_| Some(sell1_bits()),
        )
        .expect("044:16 形态几何：基例 C=(44,48) ⊆ 父 C=(33,50)，链 (top=2,exec=1) 成证")
    }

    // ───────── T1：情况一（043:28/30）─────────

    #[test]
    fn turn_class_nested_confirmed_when_top_confirmed() {
        // 链顶 confirmed=true ⟹ NestedConfirmed（c′ 账本留空也不影响）。
        let events = vec![vec![], vec![xzd_base_event()], vec![xzd_parent_event(true)]];
        let cert = xzd_certificate(&events);
        assert_eq!(cert.confirmed(), &[true, true], "sidecar 高→低含基例");
        let empty_books = ledger(vec![LevelState::default(), LevelState::default()]);
        assert_eq!(
            classify_certificate_turn(&cert, &empty_books),
            NestTurnClass::NestedConfirmed
        );
    }

    // ───────── T2：044:16 正例 ─────────

    #[test]
    fn turn_class_xiaozhuanda_candidate_044_16_positive() {
        let events = xzd_events();
        let cert = xzd_certificate(&events);
        // sidecar（断链点修复核验）：confirmed = [false, true]（链顶未确认 + 基例确认）。
        assert_eq!(cert.confirmed(), &[false, true]);
        // levels[0].bsp 置 sell3@52（center=c′）⟹ XiaozhuandaCandidate（044:16/044:24）。
        let classification = ledger(vec![
            level(vec![c_prime()], vec![pt(52, sell3_bits(), Some(c_prime()))]),
            level(parent_book_centers(), vec![]),
        ]);
        let NestTurnClass::XiaozhuandaCandidate { evidence } =
            classify_certificate_turn(&cert, &classification)
        else {
            panic!("044:16 正例应为 XiaozhuandaCandidate");
        };
        assert_eq!(evidence.c_prime, c_prime(), "c′=[420,440]/(40,46)");
        assert_eq!(evidence.third_src, 52);
        assert!(
            evidence.c_prime.end_index <= 48 && 48 < evidence.third_src,
            "因果序：c′.end=46 ≤ 基例.turn=48 < 三类点=52（044:20）"
        );
        assert_eq!(
            evidence.second_class, None,
            "levels[1].bsp 无二类点 ⟹ None 且候选仍成立（053:28 证据不作门）"
        );
        // 辅助断言：levels[1].bsp 窗内置 sell2@49 ⟹ 记入 second_class（053:28 补充证据）。
        let with_second = ledger(vec![
            level(vec![c_prime()], vec![pt(52, sell3_bits(), Some(c_prime()))]),
            level(parent_book_centers(), vec![pt(49, sell2_bits(), None)]),
        ]);
        let NestTurnClass::XiaozhuandaCandidate { evidence } =
            classify_certificate_turn(&cert, &with_second)
        else {
            panic!("置二类点后候选仍成立（不作门）");
        };
        assert_eq!(evidence.second_class, Some(49));
    }

    // ───────── T3：c′ 缺失 ⟹ 非候选（044:18）─────────

    #[test]
    fn turn_class_not_candidate_when_c_prime_missing() {
        let events = xzd_events();
        let cert = xzd_certificate(&events);
        // levels[0].centers 无包含中枢 ⟹ ExecEvidenceOnly（044:18：c 至少含一个次级别中枢）。
        let no_center = ledger(vec![LevelState::default(), level(parent_book_centers(), vec![])]);
        assert_eq!(
            classify_certificate_turn(&cert, &no_center),
            NestTurnClass::ExecEvidenceOnly
        );
        // 中枢存在但不被链顶 interval_b 包含（(10,20) ⊄ (33,50)）⟹ 同判负。
        let stray = center(300, 320, 290, 330, 10, 20);
        let outside = ledger(vec![
            level(vec![stray], vec![pt(52, sell3_bits(), Some(stray))]),
            level(parent_book_centers(), vec![]),
        ]);
        assert_eq!(
            classify_certificate_turn(&cert, &outside),
            NestTurnClass::ExecEvidenceOnly
        );
        // 中枢被包含但 end=49 > 基例.turn=48（破因果序 c′.end ≤ turn）⟹ 同判负。
        let late = center(420, 440, 410, 450, 40, 49);
        let late_book = ledger(vec![
            level(vec![late], vec![pt(52, sell3_bits(), Some(late))]),
            level(parent_book_centers(), vec![]),
        ]);
        assert_eq!(
            classify_certificate_turn(&cert, &late_book),
            NestTurnClass::ExecEvidenceOnly
        );
    }

    // ───────── T4：c′ 正常震荡 ⟹ 非候选（044:20，防误标主负例）─────────

    #[test]
    fn turn_class_not_candidate_when_c_prime_normal_oscillation() {
        let events = xzd_events();
        let cert = xzd_certificate(&events);
        // c′ 在但 levels[0].bsp 无三类点 ⟹ ExecEvidenceOnly
        //（044:20「在最后一个次级别中枢正常震荡的，都不可能转化成大级别的转折」）。
        let no_bsp = ledger(vec![level(vec![c_prime()], vec![]), level(parent_book_centers(), vec![])]);
        assert_eq!(
            classify_certificate_turn(&cert, &no_bsp),
            NestTurnClass::ExecEvidenceOnly
        );
        // 有 c′ 关联点但非三类（sell1@52）⟹ 同判负。
        let first_only = ledger(vec![
            level(vec![c_prime()], vec![pt(52, sell1_bits(), Some(c_prime()))]),
            level(parent_book_centers(), vec![]),
        ]);
        assert_eq!(
            classify_certificate_turn(&cert, &first_only),
            NestTurnClass::ExecEvidenceOnly
        );
        // 三类点存在但不携 c′（携他中枢）⟹ 同判负（3 类点携中枢契约 bsp.rs 不变量）。
        let other = center(500, 520, 490, 530, 40, 46);
        let wrong_center = ledger(vec![
            level(vec![c_prime()], vec![pt(52, sell3_bits(), Some(other))]),
            level(parent_book_centers(), vec![]),
        ]);
        assert_eq!(
            classify_certificate_turn(&cert, &wrong_center),
            NestTurnClass::ExecEvidenceOnly
        );
    }

    // ───────── T5：066:198 平台下破负例 + 044:26 镜像正例 ─────────

    #[test]
    fn turn_class_platform_breakdown_negative_066_198() {
        // 底转镜像件（side=Long）：c′ 处只有 sell3（平台下破 = 方向反，066:198
        //「小转大的平台，是可以往下突破的」）⟹ 非候选；buy3 版 ⟹ 候选（044:26）。
        let base =
            typed_event(1, Side::Long, NestDivergenceKind::Trend, (44, 48), (36, 48), 48, 48, true);
        let parent =
            typed_event(2, Side::Long, NestDivergenceKind::Trend, (33, 50), (25, 50), 50, 50, false);
        let events = vec![vec![], vec![base], vec![parent]];
        let cert = assemble_typed_certificate(
            &events,
            &events[1][0],
            2,
            NestIntervalCaliber::B,
            &|_| Some(buy1_bits()),
        )
        .expect("Long 镜像链成证");
        assert_eq!(cert.confirmed(), &[false, true]);
        let sell_side = ledger(vec![
            level(vec![c_prime()], vec![pt(52, sell3_bits(), Some(c_prime()))]),
            level(parent_book_centers(), vec![]),
        ]);
        assert_eq!(
            classify_certificate_turn(&cert, &sell_side),
            NestTurnClass::ExecEvidenceOnly,
            "066:198 平台下破：底转候选遇 sell3 = 方向反 ⟹ 非候选"
        );
        let buy_side = ledger(vec![
            level(vec![c_prime()], vec![pt(52, buy3_bits(), Some(c_prime()))]),
            level(parent_book_centers(), vec![]),
        ]);
        let NestTurnClass::XiaozhuandaCandidate { evidence } =
            classify_certificate_turn(&cert, &buy_side)
        else {
            panic!("044:26 底转镜像：c′ 三类买点 ⟹ 候选");
        };
        assert_eq!(evidence.third_src, 52);
    }

    // ───────── T6：三类点因果序反 ⟹ 非候选（044:20）─────────

    #[test]
    fn turn_class_not_candidate_when_third_precedes_base() {
        // sell3@45 先于基例 turn=48 ⟹ 因果序否决（044:20「顶背驰只能出现在c`之后」）。
        let events = xzd_events();
        let cert = xzd_certificate(&events);
        let early = ledger(vec![
            level(vec![c_prime()], vec![pt(45, sell3_bits(), Some(c_prime()))]),
            level(parent_book_centers(), vec![]),
        ]);
        assert_eq!(
            classify_certificate_turn(&cert, &early),
            NestTurnClass::ExecEvidenceOnly
        );
    }

    // ───────── T7：039:34 defer 孤儿 ─────────

    #[test]
    fn turn_class_defer_orphan_event_039_34() {
        // 父事件不被任何链消费（方向不合）⟹ 账本含 DeferOrphan{identity} 且不带候选语义；
        // 孤儿事件 100% 有类。
        let base =
            typed_event(1, Side::Long, NestDivergenceKind::Trend, (44, 48), (36, 48), 48, 48, true);
        let orphan =
            typed_event(2, Side::Short, NestDivergenceKind::Trend, (33, 50), (25, 50), 50, 50, false);
        let events = vec![vec![], vec![base], vec![orphan.clone()]];
        let cert = assemble_typed_certificate(
            &events,
            &events[1][0],
            1,
            NestIntervalCaliber::B,
            &|_| Some(buy1_bits()),
        )
        .expect("单级链成证");
        assert!(
            assemble_typed_certificate(
                &events,
                &events[1][0],
                2,
                NestIntervalCaliber::B,
                &|_| Some(buy1_bits()),
            )
            .is_none(),
            "方向不合 ⟹ (exec=1,top=2) 装配失败，父事件孤儿化"
        );
        let books = ledger(vec![
            LevelState::default(),
            LevelState::default(),
            LevelState::default(),
        ]);
        let out = classify_nest_turns(&events, &[cert], &books);
        assert_eq!(out.len(), 2, "一证 + 一孤儿，每证/每事件恰一类");
        let orphan_identity = NestEventIdentity::of(&orphan);
        let defer_entries: Vec<_> = out
            .iter()
            .filter(|(_, class)| matches!(class, NestTurnClass::DeferOrphan { .. }))
            .collect();
        assert_eq!(defer_entries.len(), 1, "孤儿事件 100% 有类（039:34 defer 域）");
        let NestTurnClass::DeferOrphan { identity } = defer_entries[0].1 else {
            unreachable!();
        };
        assert_eq!(identity, orphan_identity);
        assert_eq!(defer_entries[0].0, vec![orphan_identity], "孤儿主键 = 单元素身份向量");
        // 孤儿不带候选语义：账本无任何 XiaozhuandaCandidate 条目。
        assert!(out
            .iter()
            .all(|(_, class)| !matches!(class, NestTurnClass::XiaozhuandaCandidate { .. })));
    }

    // ───────── T8：partition 穷尽互斥锁 + sidecar 中性 ─────────

    #[test]
    fn turn_class_partition_unique_and_sidecar_neutral() {
        // 枚举合成链（depth 1..3 × rung confirmed 全组合 × c′ 状态）⟹ 每证恰一类；
        // sidecar 中性：同输入重装配逐字段相等（含 confirmed），证书真值 n_delta 不动。
        let base = || {
            typed_event(1, Side::Short, NestDivergenceKind::Trend, (44, 48), (36, 48), 48, 48, true)
        };
        let mid = |confirmed| {
            typed_event(
                2,
                Side::Short,
                NestDivergenceKind::Trend,
                (33, 50),
                (25, 50),
                50,
                50,
                confirmed,
            )
        };
        let top3 = |confirmed| {
            typed_event(
                3,
                Side::Short,
                NestDivergenceKind::Trend,
                (20, 60),
                (10, 60),
                60,
                60,
                confirmed,
            )
        };
        let sell1 = &|_: &NestCandidateEvent| Some(sell1_bits());
        let mut cases: Vec<(Vec<Vec<NestCandidateEvent>>, usize)> = vec![(vec![vec![], vec![base()]], 1)];
        for &mc in &[true, false] {
            cases.push((vec![vec![], vec![base()], vec![mid(mc)]], 2));
        }
        for &mc in &[true, false] {
            for &tc in &[true, false] {
                cases.push((vec![vec![], vec![base()], vec![mid(mc)], vec![top3(tc)]], 3));
            }
        }
        for (events, top) in cases {
            for with_c_prime in [false, true] {
                let cert = assemble_typed_certificate(
                    &events,
                    &events[1][0],
                    top,
                    NestIntervalCaliber::B,
                    sell1,
                )
                .expect("合成链成证");
                let mut levels = vec![LevelState::default(); top + 1];
                if with_c_prime && top >= 2 {
                    // c′ 落 levels[ℓ-2]（W1 算术）+ 对应 sell3@52。
                    levels[top - 2] =
                        level(vec![c_prime()], vec![pt(52, sell3_bits(), Some(c_prime()))]);
                }
                let classification = ledger(levels);
                // sidecar 中性：同输入重装配相等；向量与身份对齐；真值不动。
                let reassembled = assemble_typed_certificate(
                    &events,
                    &events[1][0],
                    top,
                    NestIntervalCaliber::B,
                    sell1,
                )
                .unwrap();
                assert_eq!(cert, reassembled, "同输入重装配逐字段相等（含 sidecar）");
                assert_eq!(cert.confirmed().len(), cert.identities().len());
                assert_eq!(cert.confirmed().last(), Some(&true), "基例门恒 confirmed=true");
                assert!(cert.certificate().n_delta(), "sidecar 不进证书真值");
                // partition：每证恰一类。
                let out = classify_nest_turns(&events, std::slice::from_ref(&cert), &classification);
                let key = cert.identities().to_vec();
                let cert_entries: Vec<_> = out.iter().filter(|(k, _)| *k == key).collect();
                assert_eq!(cert_entries.len(), 1, "每证恰一类（depth={top}）");
                let class = cert_entries[0].1;
                if cert.confirmed()[0] {
                    assert_eq!(class, NestTurnClass::NestedConfirmed);
                } else {
                    assert!(matches!(
                        class,
                        NestTurnClass::XiaozhuandaCandidate { .. } | NestTurnClass::ExecEvidenceOnly
                    ));
                }
                // 链消费掉的全部事件不作孤儿：本用例无任何 DeferOrphan 条目。
                assert!(out
                    .iter()
                    .all(|(_, class)| !matches!(class, NestTurnClass::DeferOrphan { .. })));
            }
        }
    }

    // ───────── T9：044:30 类型层封锁（候选无 confirm 语义）─────────

    #[test]
    fn turn_class_candidate_carries_no_confirm_semantics() {
        // 编译期构造保证的文档化断言（非注释承诺）：XzdEvidence 完全解构只有
        // (Center, usize, Option<usize>) 三坐标字段——无 BspBits 成员、无 confirm_side
        // 方法；字段面变化即编译失败，强制复议。
        let evidence = XzdEvidence { c_prime: c_prime(), third_src: 52, second_class: Some(49) };
        let XzdEvidence { c_prime, third_src, second_class } = evidence;
        let coords: (Center, usize, Option<usize>) = (c_prime, third_src, second_class);
        assert_eq!((coords.0.zd, coords.0.zg, coords.1, coords.2), (420, 440, 52, Some(49)));
        // NestTurnClass 四构造子穷尽匹配（无通配臂）：新增变体即编译失败——partition
        // 类型面锁定；候选臂只携坐标，不进任何 six-bit 置位路径、不作终端背书。
        let identity = NestEventIdentity::of(&xzd_parent_event(false));
        let classes = [
            NestTurnClass::NestedConfirmed,
            NestTurnClass::XiaozhuandaCandidate { evidence },
            NestTurnClass::ExecEvidenceOnly,
            NestTurnClass::DeferOrphan { identity },
        ];
        let mut names = Vec::new();
        for class in classes {
            names.push(match class {
                NestTurnClass::NestedConfirmed => "NestedConfirmed",
                NestTurnClass::XiaozhuandaCandidate { evidence } => {
                    let _only_coords: (Center, usize, Option<usize>) =
                        (evidence.c_prime, evidence.third_src, evidence.second_class);
                    "XiaozhuandaCandidate"
                }
                NestTurnClass::ExecEvidenceOnly => "ExecEvidenceOnly",
                NestTurnClass::DeferOrphan { identity: id } => {
                    assert_eq!(id, identity);
                    "DeferOrphan"
                }
            });
        }
        assert_eq!(
            names,
            ["NestedConfirmed", "XiaozhuandaCandidate", "ExecEvidenceOnly", "DeferOrphan"]
        );
    }

    // ───────── W1：c′ 级别算术钉死（施工图 §8 实装验证点）─────────

    #[test]
    fn turn_class_c_prime_book_level_arithmetic_w1() {
        // 链顶 nest 级 ℓ ⟹ c′ 账本 = levels[ℓ-2]（ℓ≥2），父级二类点账本 = levels[ℓ-1]。
        // ℓ=3 链验证：c′ 只在 levels[1] ⟹ 候选；同一 c′ 只在 levels[0] / levels[2] ⟹
        // ExecEvidenceOnly（不错读邻级账本——与 p117 T1 终端背书移位 ℓ→ℓ-1 同源事实链）。
        let base =
            typed_event(1, Side::Short, NestDivergenceKind::Trend, (44, 48), (36, 48), 48, 48, true);
        let mid =
            typed_event(2, Side::Short, NestDivergenceKind::Trend, (33, 50), (25, 50), 50, 50, true);
        let top =
            typed_event(3, Side::Short, NestDivergenceKind::Trend, (20, 60), (10, 60), 60, 60, false);
        let events = vec![vec![], vec![base], vec![mid], vec![top]];
        let cert = assemble_typed_certificate(
            &events,
            &events[1][0],
            3,
            NestIntervalCaliber::B,
            &|_| Some(sell1_bits()),
        )
        .expect("ℓ=3 链成证");
        assert_eq!(cert.confirmed(), &[false, true, true], "sidecar 高→低含基例");
        let c_prime = c_prime(); // (40,46) ⊆ 链顶 interval_b=(20,60)，end=46 ≤ 基例.turn=48。
        let third = pt(52, sell3_bits(), Some(c_prime));
        // 正例：c′ ∈ levels[1]（= levels[ℓ-2]，ℓ=3）。
        let book_at_l1 = ledger(vec![
            LevelState::default(),
            level(vec![c_prime], vec![third]),
            LevelState::default(),
        ]);
        let NestTurnClass::XiaozhuandaCandidate { evidence } =
            classify_certificate_turn(&cert, &book_at_l1)
        else {
            panic!("W1：c′ 账本 = levels[ℓ-2]，levels[1] 命中应为候选");
        };
        assert_eq!(evidence.third_src, 52);
        // 负例 1：c′ 只在 levels[0]（低一级账本）⟹ 非候选。
        let book_at_l0 = ledger(vec![
            level(vec![c_prime], vec![third]),
            LevelState::default(),
            LevelState::default(),
        ]);
        assert_eq!(
            classify_certificate_turn(&cert, &book_at_l0),
            NestTurnClass::ExecEvidenceOnly,
            "W1：c′ 落 levels[0] ≠ levels[ℓ-2]=levels[1]（ℓ=3）⟹ 非候选"
        );
        // 负例 2：c′ 只在 levels[2]（高一级账本）⟹ 非候选。
        let book_at_l2 = ledger(vec![
            LevelState::default(),
            LevelState::default(),
            level(vec![c_prime], vec![third]),
        ]);
        assert_eq!(
            classify_certificate_turn(&cert, &book_at_l2),
            NestTurnClass::ExecEvidenceOnly,
            "W1：c′ 落 levels[2] ≠ levels[ℓ-2]=levels[1]（ℓ=3）⟹ 非候选"
        );
    }
}
