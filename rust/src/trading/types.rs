//! 交易层类型基座 — 有机赋格 v2（`analysis/organic_fugue_v2_design.md`）。
//!
//! 设计来源：v2 §4 类型系统汇总 + C6（BspClass 六变体穷举）+ K10（v1R 类型基座）。
//! 引擎枚举（BspKind/Side/DivKind/Direction）直接复用，零重复定义。
//!
//! ## 非法状态不可表示清单（v2 §4 表）
//! - 六类买卖点漏分支：`BspClass` 六变体 match 穷举（C6）
//! - 中枢三态漏分支：`CenterEvent` 三变体 match 穷举（C6）
//! - 第三种守恒律静默引入：`ConservationLaw` 二变体 match 穷举（K4）
//! - earning 阶段改写成本：`LedgerPhase::EarningShares` 无 cost_basis 字段（K4）
//! - 槽键算术混淆：`SlotKey { ladder, leg }` 替代 ladder+100/+200 整数算术
//!
//! ## Python parity 注记
//! 事件元组逐字段对应 `fugue_version_i.BarSignalI` 的事件流：
//!   bsp: (kind, side, seg_idx, confirmed, center_seg_start, center_zd, center_zg, price)
//!   div: (kind, direction, side, seg_idx, force_a, force_c, price) —— side 不存储，
//!        是 direction 的纯函数（up→Sell / down→Buy，v1R §2.1 裁决）。
//! cs/zd/zg 各自独立 Option：Python 中 center_gate=False 时主腿锚可携带 None cs
//! 且 `e[4] == anchor[0]` 的 None==None 语义必须保真（Option 等值精确对应）。

use crate::buysellpoint::{BspKind, Side};
use crate::divergence::DivKind;
use crate::stroke::Direction;

/// 中枢承载层数（= Python MAX_LEVELS(8) + 3：0=bar 1=bi 2=segment 3=move(L1) 4..=recL2..）。
pub const MAX_LADDER: usize = 11;
/// 首个中枢承载层（segment 级笔中枢，525号）。
pub const FIRST_BSP_LADDER: usize = 2;
/// ARM 初始级别（Python arm_ladder = LADDER_MOVE）。
pub const LADDER_MOVE: usize = 3;

/// 初始资金（与 `fugue_alpha_diagnosis.INITIAL_CAPITAL` 逐字）。
pub const INITIAL_CAPITAL: f64 = 100_000.0;
/// 核心止损比例（stop_mode A/B）。
pub const STOP_FRAC: f64 = 0.02;
/// ARM 后次级别确认过期 bar 数（`fugue_alpha_diagnosis.SUB_EXPIRY`）。
pub const SUB_EXPIRY: i64 = 60;

/// ladder → 人类可读级别名（`fugue_version_i.ladder_name` 逐字；exit_reason 字符串成分）。
pub fn ladder_name(ladder: usize) -> String {
    match ladder {
        0 => "bar".to_string(),
        1 => "bi".to_string(),
        2 => "segment".to_string(),
        3 => "move(L1)".to_string(),
        k => format!("recL{}", k - 2),
    }
}

/// 11 层布尔行的位掩码（buy1/sell1/sell_any/buy_any/up_move_settled 各一行）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LadderMask(pub u16);

impl LadderMask {
    #[inline]
    pub fn get(self, k: usize) -> bool {
        (self.0 >> k) & 1 == 1
    }
}

/// REV 路径键容量（递归赋格深度上限 = MAX_REV_DEPTH − 1）。
/// 数据界：depth = k − FIRST_BSP_LADDER ≤ 2（设计报告 §4.3），4 留一档余量。
pub const MAX_REV_DEPTH: usize = 4;

/// REV 槽的路径键（递归赋格任务，2026-06-11）。定长数组保 Copy/Hash。
///
/// `segs[0]` = tranche 确认级别（与旧 `Rev(usize)` 同义——深度 0 时本类型
/// 是旧编码的同构替换，py_key/leg_kind 投影逐位不变）；
/// `segs[1..len]` = 嵌套子 LOU 的级别坐标（深度 n 的子腿，符号交替塔
/// (−1)^n：奇数深度 = 反弹腿先买后卖，偶数深度 = 短差先卖后买）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RevPath {
    segs: [u8; MAX_REV_DEPTH],
    len: u8,
}

impl RevPath {
    /// 单段路径（= 旧 `Rev(level)` 语义）。
    pub fn single(level: usize) -> Self {
        debug_assert!(level < 256);
        let mut segs = [0u8; MAX_REV_DEPTH];
        segs[0] = level as u8;
        RevPath { segs, len: 1 }
    }

    /// 追加一级（子 LOU 腿）。容量满时 panic——调用方（runner capability
    /// guard）保证 rev_sub_depth + 1 ≤ MAX_REV_DEPTH，到达即 bug。
    pub fn child(self, sub_level: usize) -> Self {
        assert!(
            (self.len as usize) < MAX_REV_DEPTH,
            "RevPath 容量耗尽：guard 必须拒绝 rev_sub_depth ≥ MAX_REV_DEPTH"
        );
        let mut segs = self.segs;
        segs[self.len as usize] = sub_level as u8;
        RevPath { segs, len: self.len + 1 }
    }

    /// 递归深度（0 = 普通 REV 腿；n = 第 n 层子腿）。
    pub fn depth(self) -> usize {
        self.len as usize - 1
    }
}

/// 腿类别。Python 槽键算术（ladder / +100 / +200）的类型化替代。
/// `Rev(RevPath)` 携带 tranche 确认级别 + 嵌套子腿路径（递归赋格任务：
/// C4 级别隔离延伸到 tranche 级 + 任意深度子 LOU 坐标）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LegClass {
    Main,
    Osc,
    Rev(RevPath),
}

/// 账本槽键。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlotKey {
    pub ladder: usize,
    pub leg: LegClass,
}

impl SlotKey {
    pub fn main(ladder: usize) -> Self {
        SlotKey { ladder, leg: LegClass::Main }
    }
    pub fn osc(ladder: usize) -> Self {
        SlotKey { ladder, leg: LegClass::Osc }
    }
    pub fn rev(home: usize, level: usize) -> Self {
        SlotKey { ladder: home, leg: LegClass::Rev(RevPath::single(level)) }
    }

    /// 路径键构造（递归子腿）。
    pub fn rev_path(home: usize, path: RevPath) -> Self {
        SlotKey { ladder: home, leg: LegClass::Rev(path) }
    }

    /// Python 整数槽键投影（trace/归因报告兼容：main=ladder / osc=+100 / rev=+200）。
    /// Rev tranche 的 level 不进键投影（Python v1 单 rev 槽无对应物；level 由
    /// `leg_name` 单列报告）。递归子腿（depth ≥ 1）无 Python oracle——投影
    /// 显式扩区为 +200+100×depth（depth1=+300），与在册 +200 区不混表
    /// （Python parity 边界重划，设计报告 §5.1）。
    pub fn py_key(self) -> i64 {
        match self.leg {
            LegClass::Main => self.ladder as i64,
            LegClass::Osc => self.ladder as i64 + 100,
            LegClass::Rev(p) => self.ladder as i64 + 200 + 100 * p.depth() as i64,
        }
    }

    /// 腿类型名（trace `leg` 字段：与 Python `leg_kind` 逐字；递归子腿新词）。
    pub fn leg_kind(self) -> &'static str {
        match self.leg {
            LegClass::Main => "main",
            LegClass::Osc => "osc",
            LegClass::Rev(p) if p.depth() == 0 => "rev",
            LegClass::Rev(_) => "rev_sub",
        }
    }
}

/// 六种买卖点的规范形（C6）。(BspKind × Side) 积类型的和类型展开——
/// 消费点 match 此枚举时，漏任何一类编译不过。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BspClass {
    /// 下跌趋势背驰点（17课）。
    Buy1,
    /// 1买后次级别上涨结束再下跌不创新低的结束点（17课/定律一）。
    Buy2,
    /// 次级别离开中枢后第一次回抽不破 ZG（21/33课）。
    Buy3,
    /// 上涨趋势顶背驰点。
    Sell1,
    /// 1卖后次级别下跌结束再上涨不创新高的结束点。
    Sell2,
    /// 次级别离开中枢后第一次回抽不破 ZD（33课"必须走"）。
    Sell3,
}

impl BspClass {
    pub fn from_parts(kind: BspKind, side: Side) -> Self {
        match (kind, side) {
            (BspKind::Type1, Side::Buy) => BspClass::Buy1,
            (BspKind::Type2, Side::Buy) => BspClass::Buy2,
            (BspKind::Type3, Side::Buy) => BspClass::Buy3,
            (BspKind::Type1, Side::Sell) => BspClass::Sell1,
            (BspKind::Type2, Side::Sell) => BspClass::Sell2,
            (BspKind::Type3, Side::Sell) => BspClass::Sell3,
        }
    }

    pub fn kind(self) -> BspKind {
        match self {
            BspClass::Buy1 | BspClass::Sell1 => BspKind::Type1,
            BspClass::Buy2 | BspClass::Sell2 => BspKind::Type2,
            BspClass::Buy3 | BspClass::Sell3 => BspKind::Type3,
        }
    }

    pub fn side(self) -> Side {
        match self {
            BspClass::Buy1 | BspClass::Buy2 | BspClass::Buy3 => Side::Buy,
            BspClass::Sell1 | BspClass::Sell2 | BspClass::Sell3 => Side::Sell,
        }
    }
}

/// REV 开腿信号类型（配对修正轴 `rev_paired`，2026-06-10 编排者任务）。
///
/// 诊断根因：v1/V1f 的 REV 腿与域腿 type1卖→type3买（67.9% 卖飞）同根因——
/// 信号层 kind 折叠 + 盲配对（seg_end 三触发折叠为一个无差别触发，闭腿端
/// buy_any 无差别回补）。修正 = kind 展开：
/// - **震荡型**：confirmed Sell1 / 盘整背驰卖，且所在中枢存活——反向走势在
///   存活中枢语境内，ZD 触线回补几何有效（域腿 77% 胜率的同一机制）。
/// - **逃逸型**：confirmed Sell3（中枢向下终结）——价格已离开中枢，无 ZD
///   触线目标，闭腿走趋势配对（confirmed Buy1）或 Buy3 回补位。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevOpenKind {
    Oscillation,
    Escape,
}

/// BSP 事件（BarSignalI.bsp_events 单元素的类型化）。
/// cs/zd/zg 独立 Option（Python 元组语义保真，见模块 docstring）。
#[derive(Debug, Clone, Copy)]
pub struct BspEvent {
    pub class: BspClass,
    /// 事件锚段索引（Python 去重键成分；交易层当前无消费者——磁带词汇完整性
    /// 传输位，C6 完整词汇 + 归因报告的字段基础）。
    #[allow(dead_code)]
    pub seg_idx: i64,
    pub confirmed: bool,
    /// center_seg_start（type1 可无锚）。
    pub cs: Option<i64>,
    pub zd: Option<f64>,
    pub zg: Option<f64>,
    /// 事件端点价（成交价恒用 close——P5 口径；传输位同 seg_idx）。
    #[allow(dead_code)]
    pub price: f64,
}

/// 背驰事件。side 不存储——direction 的纯函数（存两个字段允许不一致状态）。
#[derive(Debug, Clone, Copy)]
pub struct DivEvent {
    pub kind: DivKind,
    /// 背驰所在 move 方向（Python "up"/"down"）。
    pub direction: Direction,
    /// 背驰段锚（div.seg_c_end；传输位，同 BspEvent.seg_idx）。
    #[allow(dead_code)]
    pub seg_idx: i64,
    /// 力度字段：G-2 力度收敛门控的预注册数据基础（v2 §C2/G-2："以 div_events
    /// 的 force_a/force_c 字段为数据基础（已透传，K3）"）——列为 v2 后续工位。
    #[allow(dead_code)]
    pub force_a: f64,
    #[allow(dead_code)]
    pub force_c: f64,
    #[allow(dead_code)]
    pub price: f64,
}

impl DivEvent {
    pub fn side(self) -> Side {
        match self.direction {
            Direction::Up => Side::Sell,
            Direction::Down => Side::Buy,
        }
    }
}

/// 中枢三态事件（C6 词汇表的另一半；33课新生/延伸/终结）。
///
/// 数据源声明（090号，与 v2 §7 的差异显式落盘）：v2 设计把 center_events 列为
/// 磁带行（D2，信号层 zhongshus diff 产出）；当前磁带（organic_signals.py）无此行，
/// 本实现由 `CenterBook::ingest` 在唯一 diff 点从 BSP 事件锚派生（单一真相源，
/// compute-once 同精神）。两个后果：
///   (a) 只有产出过 BSP 事件锚的中枢可见（CenterBook 本就是"最后已知中枢"账本）；
///   (b) `Extended` 不携带 seg_count（事件锚不含段计数；§5.3 矩阵中 Extended
///       全行 no-op，字段无消费者，故删除而非填 0——声明=能力）。
/// M2 信号层下沉时升级为磁带行，本枚举形状不变。
/// 事件身份载荷（seg_start/zd/zg）当前消费者只 match 变体类别（§5.3 矩阵：
/// 门清空/加码按"是否 Formed/Terminated{方向}"判定）；载荷为词汇完整性传输位。
#[derive(Debug, Clone, Copy)]
pub enum CenterEvent {
    /// 新中枢（last_center 的 cs 变化）。
    Formed {
        #[allow(dead_code)]
        seg_start: i64,
        #[allow(dead_code)]
        zd: f64,
        #[allow(dead_code)]
        zg: f64,
    },
    /// 同一中枢边界更新（cs 不变，zd/zg 变化）。
    Extended {
        #[allow(dead_code)]
        seg_start: i64,
    },
    /// 中枢死亡（confirmed type3；direction：Buy3→Up 离开 / Sell3→Down 离开）。
    Terminated {
        #[allow(dead_code)]
        seg_start: i64,
        direction: Direction,
    },
}

/// 锚来源标记（Python osc 锚第三元字符串 "osc" 与 BSP kind 字符串共用槽位的类型化）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorKind {
    Bsp(BspKind),
    Osc,
}

/// 腿锚点。rev 腿"无中枢锚"是独立变体——"带中枢锚的 rev 腿"不可表示（E5 编码）。
#[derive(Debug, Clone, Copy)]
pub enum LegAnchor {
    /// osc/main 腿：锚定中枢快照。boundary = 触线回补价（osc=ZD）或开腿锚（main=ZG）。
    /// main 腿在 center_gate=False 下 cs 可为 None（Python None==None 等值保真）。
    /// kind = 锚来源（trace 归因的字段基础，v1R §2.7 注；当前 trace 未输出）。
    Center {
        cs: Option<i64>,
        boundary: Option<f64>,
        #[allow(dead_code)]
        kind: AnchorKind,
    },
    /// rev 腿：段尺度，无中枢锚（§4b：价格已离开中枢）。
    SegmentScale,
}

/// 两阶段守恒律（31/43课）。腿 open 时刻按账本阶段冻结进腿记录，
/// close 时 match 穷举——第三种守恒律不可静默引入。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConservationLaw {
    /// 降成本阶段开的腿：股数守恒，价差 profit 降共享 cost_basis。
    ShareConserving,
    /// 挣股数阶段开的腿：金额守恒（卖 V 买 V），total_shares 净增，cost_basis 锁 0。
    AmountConserving,
}

/// 账本阶段（31课两阶段的类型化）。EarningShares 变体没有 cost_basis 字段——
/// 挣股数阶段"成本"概念不存在（锁 0 不是值是性质）。单向相变：只有
/// CostReduction → EarningShares 的代码路径，逆向不可达（INV-2）。
#[derive(Debug, Clone, Copy)]
pub enum LedgerPhase {
    /// 不变量：cost_basis > 0（≤0 即刻相变）。
    CostReduction { cost_basis: f64 },
    EarningShares,
}

impl LedgerPhase {
    /// 报告/trace 用读数（earning ⇒ 0.0）。只读投影，无 setter。
    pub fn cost_basis(self) -> f64 {
        match self {
            LedgerPhase::CostReduction { cost_basis } => cost_basis,
            LedgerPhase::EarningShares => 0.0,
        }
    }

    pub fn is_earning(self) -> bool {
        matches!(self, LedgerPhase::EarningShares)
    }
}

/// 计数器：Python 字符串 dict 的结构体化（拼错键名 = 编译错误）。
/// 前 18 个字段与 Python `run_organic` counters 逐键对应；其后为 v2 新增可观测面。
#[derive(Debug, Default, Clone)]
pub struct Counters {
    // ── Python v1 逐键对应 ──
    pub n_close_normal: u64,
    pub n_close_pre_type3: u64,
    pub n_close_hard_type3: u64,
    pub n_open_gate_rejects: u64,
    pub n_osc_open: u64,
    pub n_osc_zd_close: u64,
    /// 49课严格形式（osc_sell3_no_recover）：锚中枢死亡后不回补的持有 bar 数。
    pub n_osc_dead_holds: u64,
    /// 41课门（域腿，osc_l41_gate）：父级别上行无衰竭被拒的 osc 开腿尝试数。
    pub n_osc_l41_rejects: u64,
    /// osc 操作域（osc_domain=ConsolidationOnly）：锚中枢所在走势 kind==Trend
    /// 时域外不开的 osc 触发点数（对象域定义而非点态门——计数的是"趋势走势
    /// 中本不存在的操作对象"被在册定义误触发的次数）。
    pub n_osc_domain_rejects: u64,
    pub n_rev_attempts: u64,
    pub n_rev_gate_rejects: u64,
    pub n_rev_frozen_rejects: u64,
    pub n_rev_open: u64,
    pub n_rev_close_t5: u64,
    pub n_rev_close_t6: u64,
    pub n_rev_close_t7: u64,
    pub n_master_rev_open: u64,
    pub n_master_rev_close: u64,
    pub n_earning_reached: u64,
    pub n_exit_upgraded: u64,
    pub n_open_rejects_zero: u64,
    // ── v2 新增（C3 拒因分计数 / C4 tranche / C5 T5b / C7 门开率 / T5 疑点）──
    pub n_rev_sub_anchor_rejects: u64,
    pub n_rev_budget_rejects: u64,
    pub n_rev_tranche_adds: u64,
    pub n_rev_tranche_closes: u64,
    pub n_rev_struct_close: u64,
    pub n_t5_shareconserving_after_earning: u64,
    pub fatigue_open_bars_by_ladder: [u64; MAX_LADDER],
    // ── rev_paired 配对修正可观测面（2026-06-10 编排者任务）──
    /// 震荡型开腿（confirmed Sell1/盘背 × 存活中枢 × 深度门过）。
    pub n_rev_open_osc: u64,
    /// 逃逸型开腿（confirmed Sell3 × 深度门过）。
    pub n_rev_open_esc: u64,
    /// 开腿拒：震荡型无存活中枢 / 逃逸型事件缺 zd/zg / 边界非有限。
    pub n_rev_nocenter_rejects: u64,
    /// 开腿拒：中枢振幅 (ZG−ZD)/price < θ_depth，或震荡型 c ≤ ZD（零利润空间）。
    pub n_rev_depth_rejects: u64,
    /// ZD 触线闭腿（震荡型专属，域腿同机制）。
    pub n_rev_zd_close: u64,
    /// kind/锚不匹配持仓 bar 数：legacy T5 谓词（Buy2/盘背买/异锚Buy1）会闭、
    /// 配对谓词拒——"不接受 kind 不匹配的买点"的反事实计数。
    pub n_rev_mismatch_holds: u64,
    /// 按开腿类型分解的闭合对统计（win = profit > 0）。
    pub rev_osc_pairs: u64,
    pub rev_osc_wins: u64,
    pub rev_esc_pairs: u64,
    pub rev_esc_wins: u64,
    // ── 盘背递归正则化消融可观测面（2026-06-11 任务，R1/R2/R3）──
    /// R1 拒：盘背触发存在但次级别 Sell 证据不在 C 段窗口内（且无其他触发源）。
    pub n_rev_r1_rejects: u64,
    /// R2 兑现：ZG 触线闭腿（reason 9，原文保证域兑现）。
    pub n_rev_zg_close: u64,
    /// R3 持有：candidate Buy3 出现但次级别"回跌不重回"证据缺失，t6 被抑制的 bar 数。
    pub n_rev_r3_holds: u64,
    /// θ 自适应 warm-up 回退：参照集样本 < min_obs ⇒ 使用 cfg.theta_depth
    /// 固定值的开腿尝试数（仅 theta_mode=AdaptiveQuantile 计数）。
    pub n_rev_theta_fallbacks: u64,
    /// θ 成本门下界触发：分位数 < k×friction_rt，θ_eff 被抬到下界的取值次数
    /// （仅 AdaptiveQuantile 计数——下界激活 = 该层参照振幅不足覆盖 k 倍往返成本）。
    pub n_rev_theta_cost_floor: u64,
    /// 41课门（REV 主腿，rev_l41_gate）：父级别上行无衰竭被拒的开腿尝试数。
    pub n_rev_l41_rejects: u64,
    // ── 次级别确认完整递归可观测面（2026-06-11 任务，SC 六位）──
    /// SCe：超时 bar 确认仍缺、legacy 会 fallback 入场而 strict 拒绝（每 ARMED 期一次）。
    pub n_sc_entry_expire_skips: u64,
    /// SCm：sell1 行真但次级别卖确认缺，master 出场被延迟的 bar 数。
    pub n_sc_master_holds: u64,
    /// SCo：合格卖事件存在但次级别卖确认缺，main 开腿被拒次数。
    pub n_sc_main_open_rejects: u64,
    /// SCc：同锚 confirmed Buy1 出现但次级别买确认缺，main 闭腿被延迟的 bar 数。
    pub n_sc_main_close_holds: u64,
    /// SCr：REV 开腿触发存在但次级别卖确认缺，开腿被拒次数。
    pub n_sc_rev_open_rejects: u64,
    /// SC7：confirmed Buy3 出现但次级别买确认缺，回补被延迟的 bar 数。
    pub n_sc_t7_holds: u64,
    // ── type2 消融可观测面（2026-06-11 任务，T2o/T2c）──
    /// T2o：trigger 掩码含 bit3（confirmed Sell2 参与触发）的开腿数。
    pub n_rev_sell2_open: u64,
    /// T2c：confirmed Buy2 配对闭腿数（reason 10）。
    pub n_rev_buy2_close: u64,
    // ── 38课位置分支可观测面（2026-06-11 任务，rev_seq_nobreak）──
    /// Seq38n：不跌破第一段低点 ∧ 次级别结构确认 → 买回（reason 11；
    /// 38课:36 分岔1 + 答疑:296 的主 REV 腿形式）。
    pub n_rev_seq_nobreak_close: u64,
    // ── 递归赋格子腿可观测面（2026-06-11 任务，rev_sub_depth ≥ 1）──
    /// 子腿开腿（奇数深度反弹腿先买 / 偶数深度短差先卖）。
    pub n_sub_open: u64,
    /// 子腿信号闭腿（配对镜像谓词；不含级联强闭）。
    pub n_sub_close: u64,
    /// 子腿级联强闭（父腿闭合时未决子树腿的强制平仓）。
    pub n_sub_forced_close: u64,
    /// 开腿拒：子级别中枢振幅 < 2×sub_friction_rt（经济终止条件）。
    pub n_sub_amp_rejects: u64,
    /// 开腿拒：子级别无存活中枢（反弹腿的锚域不存在）。
    pub n_sub_nocenter_rejects: u64,
    /// 开腿拒：账本处于 EarningShares——金额守恒对先买后卖循环未定义
    /// （声明=能力：显式拒绝并计数，不静默降级为另一守恒律）。
    pub n_sub_earning_rejects: u64,
    // ── P1 双门可观测面（2026-06-11 任务，Fractal 子腿）──
    /// 成本门拒：该层典型中枢振幅 θ_q < sub_cost_k × sub_friction_rt
    /// （35课：波幅不够覆盖往返成本的级别自动关闭）。
    pub n_sub_cost_rejects: u64,
    /// 成本门拒（参照不可定义）：θ_q 无参照集——bi 级无中枢事件流 /
    /// warm-up 样本 < SUB_COST_MIN_OBS，保守拒绝（不静默放行先例）。
    pub n_sub_cost_noref_rejects: u64,
    /// 41课门拒：父级别（子腿 ladder+1）向下走势无衰竭迹象
    /// （相邻同向段创新低 ∧ 无盘整背驰 = 趋势未完 = 不做反向）。
    pub n_sub_l41_rejects: u64,
    // ── Sequence38 闭腿三岔分解（2026-06-11 任务，sub_mode=Sequence38；
    //    归因优先序 盘背买 > 不跌破 > 新下跌背驰——同 bar 共现取最强证据）──
    /// 段间盘整背驰买点买回（38课:36 分岔2 + 第二段完成的事件证据）。
    pub n_sub_seq_consbuy_close: u64,
    /// 不跌破第一段低点买回（38课:36 分岔1；低点未破 ∧ 次级别结构确认）。
    pub n_sub_seq_nobreak_close: u64,
    /// 观望出口买回（新的下跌背驰：Trend×Buy ∨ confirmed Buy1）。
    pub n_sub_seq_newdiv_close: u64,
    /// 子腿闭合对统计（win = profit > 0；含级联强闭）。
    pub sub_pairs: u64,
    pub sub_wins: u64,
    // ── 38课循环 voice 可观测面（2026-06-11 任务，rev_cycle=Cycle38）──
    /// 进入循环：UpLeg ∧ 宿主（ladder+1）趋势态成立（kind==Trend ∧ dir==Up）。
    pub n_c38_enter: u64,
    /// 循环终止：宿主趋势态翻落（38课"不创新高或盘整背驰"的趋势态行读数）。
    pub n_c38_exit: u64,
    /// 循环内短差开（本级别卖点：confirmed Sell1 ∨ 盘背卖）。
    pub n_c38_open: u64,
    /// 循环内短差闭（次级别 k−1 买点）。
    pub n_c38_close: u64,
    /// 循环终止 bar 未决腿强闭（卖了必须买回——38课程序内置追价买回形态）。
    pub n_c38_forced_close: u64,
    /// 成本门拒：该层典型中枢振幅 θ_q < theta_cost_k × friction_rt（35课）。
    pub n_c38_cost_rejects: u64,
    /// 成本门拒（参照不可定义）：warm-up 样本不足，保守拒绝（不静默放行）。
    pub n_c38_cost_noref_rejects: u64,
    /// 开腿拒：本级别冻结（hard type3 已落——G3a 同语义）。
    pub n_c38_frozen_rejects: u64,
    /// 开腿拒（rev_cycle_close ≠ SubAny 专属）：买回判据的锚中枢不可定义
    /// （无存活中枢/边界非有限）——同锚 Buy1/ZD 触线没有比较基准，保守拒绝
    /// （不静默放行先例）。SubAny 恒 0（无锚依赖，开腿路径零接触）。
    pub n_c38_nocenter_rejects: u64,
    // ── 38课循环买回判据消融分解（2026-06-11 任务；reason 编码与
    //    RevCloseLog 同义：7=confirmed Buy3 / 6=T6 candidate Buy3 /
    //    5=同锚Buy1 / 8=ZD 触线 / 10=Buy2（BspAny 臂）/ 11=任意锚Buy1
    //    （BspAny 臂；与 5 同锚区分）。SubAny 闭腿不入分解位（n_c38_close
    //    在册语义不变）──
    pub n_c38_close_t7: u64,
    pub n_c38_close_t6: u64,
    pub n_c38_close_buy1: u64,
    pub n_c38_close_zd: u64,
    pub n_c38_close_buy2: u64,
    pub n_c38_close_buy1any: u64,
    // ── 反事实可观测（编排者递归因果纠正 2026-06-11：一卖→回落→二买/
    //    三买涌现即买回机会）：持腿期本级别 confirmed 买点出现但不在当前
    //    闭腿集内（本 bar 未闭）的 bar 数——量化"被漏掉的买回机会"──
    pub n_c38_buy1_holds: u64,
    pub n_c38_buy2_holds: u64,
    pub n_c38_buy3_holds: u64,
    /// 循环短差闭合对统计（win = profit > 0；含强闭）。
    pub c38_pairs: u64,
    pub c38_wins: u64,
    // ── master 入场侧递归建仓可观测面（2026-06-11 任务，entry_mode=Recursive）──
    /// 入场后更高级别 confirmed buy1 触发的追加 tranche 数（不含初始入场）。
    pub n_rec_entry_fills: u64,
    /// 达到满仓（undeployed_cash 流干）的持仓数。
    pub n_rec_entry_full: u64,
    /// 未满仓即出场的持仓数（close 时 undeployed_cash > 0）。
    pub n_rec_entry_partial_exits: u64,
    /// 追加被拒：账本处于 EarningShares——成本概念已不存在，加权均价算术
    /// 未定义（显式拒绝并计数，open_sub 同先例）。
    pub n_rec_entry_earning_rejects: u64,
    // ── master 出场状态驱动可观测面（2026-06-11 任务，exit_mode=HoldTrend）──
    /// HoldTrend：entry 级 sell1 触发成立但父级别（entry_ladder+1）上行趋势
    /// 未衰竭（up_unexhausted），master 出场被拦截持仓的 bar 数。
    pub n_exit_trend_holds: u64,
    /// Emergent：持仓中走势级别归属高于 entry 级的 bar 数（mx_lad >
    /// entry_ladder 的 LONG bar 计数——归属活性读数；归属到多高的分布
    /// 由 exit reason 中的 ladder 名读出）。
    pub n_exit_emergent_bars: u64,
    /// 净现金按类型分解（f64，lib.rs 单独 marshal——py_items 仅 u64）。
    pub rev_osc_cash: f64,
    pub rev_esc_cash: f64,
    /// 子腿聚合净现金（O_sub1 判据的直接读数；f64 同上单独 marshal）。
    pub sub_cash: f64,
    /// 循环短差聚合净现金（cycle38 总贡献判据；f64 同上单独 marshal）。
    pub c38_cash: f64,
    /// 循环短差逐腿闭合日志 (reason, profit)：reason 编码同上分解位 +
    /// 0=SubAny 次级别买点 / 1=循环终止强闭——按买点类型的 payoff 分布
    /// 读数（编排者递归因果诊断的数据基础）。lib.rs 单独 marshal（list）。
    pub c38_close_profits: Vec<(u8, f64)>,
    // ── 账本 voice 可观测面（2026-06-11 并发赋格最小实验，voice_mode=Ledger）──
    /// 账本开腿：confirmed Sell@k 且槽空 → 卖出 frac_k（清 slice）。
    pub n_ledger_opens: u64,
    /// 账本闭腿：confirmed Buy@k 且槽开 → 买回（填 slice）。
    pub n_ledger_closes: u64,
    /// 槽已开的 Sell 事件 no-op（"清至 0 幂等"的饱和算术读数）。
    pub n_ledger_sell_noops: u64,
    /// 槽空的 Buy 事件 no-op（"满仓者二买无事可做"读数，26课恒仓推论）。
    pub n_ledger_buy_noops: u64,
    /// 41课域腿门拒开逐事件日志 (ladder, bar)：regime 分段统计的数据基础
    /// （拦截率/盈亏按牛熊震荡阶段分解——2026-06-11 追加质询）。仅
    /// osc_l41_gate 变体非空（门关恒空表）。lib.rs 单独 marshal（list）。
    pub osc_l41_reject_log: Vec<(u8, i64)>,
    /// osc 操作域域外不开逐事件日志 (ladder, bar)：regime 分段统计数据基础
    /// （l41 日志同构）。仅 osc_domain=ConsolidationOnly 变体非空。
    /// lib.rs 单独 marshal（list）。
    pub osc_domain_reject_log: Vec<(u8, i64)>,
    // ── REV 腿逐腿日志（trade_behavior 行为分解，2026-06-11 任务）──
    // 仅 rev_paired 路径产出（legacy 腿无 kind/锚概念——声明=能力）。
    // PyO3 不可见：py_items 与 lib.rs marshal 均不含 → 在册对账面零侵入。
    pub rev_open_log: Vec<RevOpenLog>,
    pub rev_close_log: Vec<RevCloseLog>,
}

/// REV 腿开腿日志行（rev_paired_open 开腿成功点记录）。
#[derive(Debug, Clone)]
pub struct RevOpenLog {
    pub ladder: u8,
    pub bar: i64,
    /// 0 = 震荡型（Oscillation），1 = 逃逸型（Escape）。
    pub kind: u8,
    /// 开腿触发位掩码：bit0 = confirmed Sell1（type1 卖），
    /// bit1 = 盘整背驰卖（DivKind::Consolidation × Up），bit2 = confirmed Sell3，
    /// bit3 = confirmed Sell2（T2o 轴，sell2_open）。
    pub trigger: u8,
    /// 锚中枢 seg_start（震荡型=存活中枢快照；逃逸型=被终结中枢）。
    pub anchor_cs: Option<i64>,
    /// 锚中枢边界（震荡型恒 Some；逃逸型 RevLeg.zd=None 不触线，此处仍记录
    /// 事件自带边界供振幅分析）。
    pub zd: Option<f64>,
    pub zg: Option<f64>,
    pub price: f64,
}

/// REV 腿闭腿日志行。reason: 5=T5 kind配对 Buy1、6=T6 candidate Buy3 预回补、
/// 7=T7 confirmed Buy3 回补、8=ZD 触线（R2 延伸档触 ZD 同码）、
/// 9=ZG 兑现（R2 原文保证域锚）、10=T2c confirmed Buy2 配对（buy2_close 轴）、
/// 11=Seq38n 不跌破第一段低点×次级别确认（rev_seq_nobreak 轴）。
/// 未出现在本日志的开腿 = 强平
/// （master 出场/eod，按 (ladder, open_bar) 与开腿日志 join 可识别）。
#[derive(Debug, Clone)]
pub struct RevCloseLog {
    pub ladder: u8,
    pub open_bar: i64,
    pub bar: i64,
    pub reason: u8,
    pub price: f64,
}

impl Counters {
    /// Python counters dict 的逐键导出（对账面：键名与 run_organic 逐字一致）。
    pub fn py_items(&self) -> Vec<(&'static str, u64)> {
        vec![
            ("n_close_normal", self.n_close_normal),
            ("n_close_pre_type3", self.n_close_pre_type3),
            ("n_close_hard_type3", self.n_close_hard_type3),
            ("n_open_gate_rejects", self.n_open_gate_rejects),
            ("n_osc_open", self.n_osc_open),
            ("n_osc_zd_close", self.n_osc_zd_close),
            ("n_osc_dead_holds", self.n_osc_dead_holds),
            ("n_osc_l41_rejects", self.n_osc_l41_rejects),
            ("n_rev_attempts", self.n_rev_attempts),
            ("n_rev_gate_rejects", self.n_rev_gate_rejects),
            ("n_rev_frozen_rejects", self.n_rev_frozen_rejects),
            ("n_rev_open", self.n_rev_open),
            ("n_rev_close_t5", self.n_rev_close_t5),
            ("n_rev_close_t6", self.n_rev_close_t6),
            ("n_rev_close_t7", self.n_rev_close_t7),
            ("n_master_rev_open", self.n_master_rev_open),
            ("n_master_rev_close", self.n_master_rev_close),
            ("n_earning_reached", self.n_earning_reached),
            ("n_exit_upgraded", self.n_exit_upgraded),
            ("n_open_rejects_zero", self.n_open_rejects_zero),
            ("n_rev_sub_anchor_rejects", self.n_rev_sub_anchor_rejects),
            ("n_rev_budget_rejects", self.n_rev_budget_rejects),
            ("n_rev_tranche_adds", self.n_rev_tranche_adds),
            ("n_rev_tranche_closes", self.n_rev_tranche_closes),
            ("n_rev_struct_close", self.n_rev_struct_close),
            ("n_t5_shareconserving_after_earning", self.n_t5_shareconserving_after_earning),
            ("n_rev_open_osc", self.n_rev_open_osc),
            ("n_rev_open_esc", self.n_rev_open_esc),
            ("n_rev_nocenter_rejects", self.n_rev_nocenter_rejects),
            ("n_rev_depth_rejects", self.n_rev_depth_rejects),
            ("n_rev_zd_close", self.n_rev_zd_close),
            ("n_rev_mismatch_holds", self.n_rev_mismatch_holds),
            ("rev_osc_pairs", self.rev_osc_pairs),
            ("rev_osc_wins", self.rev_osc_wins),
            ("rev_esc_pairs", self.rev_esc_pairs),
            ("rev_esc_wins", self.rev_esc_wins),
            ("n_rev_r1_rejects", self.n_rev_r1_rejects),
            ("n_rev_zg_close", self.n_rev_zg_close),
            ("n_rev_r3_holds", self.n_rev_r3_holds),
            ("n_rev_theta_fallbacks", self.n_rev_theta_fallbacks),
            ("n_rev_theta_cost_floor", self.n_rev_theta_cost_floor),
            ("n_rev_l41_rejects", self.n_rev_l41_rejects),
            ("n_sc_entry_expire_skips", self.n_sc_entry_expire_skips),
            ("n_sc_master_holds", self.n_sc_master_holds),
            ("n_sc_main_open_rejects", self.n_sc_main_open_rejects),
            ("n_sc_main_close_holds", self.n_sc_main_close_holds),
            ("n_sc_rev_open_rejects", self.n_sc_rev_open_rejects),
            ("n_sc_t7_holds", self.n_sc_t7_holds),
            ("n_rev_sell2_open", self.n_rev_sell2_open),
            ("n_rev_buy2_close", self.n_rev_buy2_close),
            ("n_rev_seq_nobreak_close", self.n_rev_seq_nobreak_close),
            ("n_sub_open", self.n_sub_open),
            ("n_sub_close", self.n_sub_close),
            ("n_sub_forced_close", self.n_sub_forced_close),
            ("n_sub_amp_rejects", self.n_sub_amp_rejects),
            ("n_sub_nocenter_rejects", self.n_sub_nocenter_rejects),
            ("n_sub_earning_rejects", self.n_sub_earning_rejects),
            ("n_sub_cost_rejects", self.n_sub_cost_rejects),
            ("n_sub_cost_noref_rejects", self.n_sub_cost_noref_rejects),
            ("n_sub_l41_rejects", self.n_sub_l41_rejects),
            ("n_sub_seq_consbuy_close", self.n_sub_seq_consbuy_close),
            ("n_sub_seq_nobreak_close", self.n_sub_seq_nobreak_close),
            ("n_sub_seq_newdiv_close", self.n_sub_seq_newdiv_close),
            ("sub_pairs", self.sub_pairs),
            ("sub_wins", self.sub_wins),
            ("n_c38_enter", self.n_c38_enter),
            ("n_c38_exit", self.n_c38_exit),
            ("n_c38_open", self.n_c38_open),
            ("n_c38_close", self.n_c38_close),
            ("n_c38_forced_close", self.n_c38_forced_close),
            ("n_c38_cost_rejects", self.n_c38_cost_rejects),
            ("n_c38_cost_noref_rejects", self.n_c38_cost_noref_rejects),
            ("n_c38_frozen_rejects", self.n_c38_frozen_rejects),
            ("n_c38_nocenter_rejects", self.n_c38_nocenter_rejects),
            ("n_c38_close_t7", self.n_c38_close_t7),
            ("n_c38_close_t6", self.n_c38_close_t6),
            ("n_c38_close_buy1", self.n_c38_close_buy1),
            ("n_c38_close_zd", self.n_c38_close_zd),
            ("n_c38_close_buy2", self.n_c38_close_buy2),
            ("n_c38_close_buy1any", self.n_c38_close_buy1any),
            ("n_c38_buy1_holds", self.n_c38_buy1_holds),
            ("n_c38_buy2_holds", self.n_c38_buy2_holds),
            ("n_c38_buy3_holds", self.n_c38_buy3_holds),
            ("c38_pairs", self.c38_pairs),
            ("c38_wins", self.c38_wins),
            ("n_rec_entry_fills", self.n_rec_entry_fills),
            ("n_rec_entry_full", self.n_rec_entry_full),
            ("n_rec_entry_partial_exits", self.n_rec_entry_partial_exits),
            ("n_rec_entry_earning_rejects", self.n_rec_entry_earning_rejects),
            ("n_exit_trend_holds", self.n_exit_trend_holds),
            ("n_exit_emergent_bars", self.n_exit_emergent_bars),
            ("n_ledger_opens", self.n_ledger_opens),
            ("n_ledger_closes", self.n_ledger_closes),
            ("n_ledger_sell_noops", self.n_ledger_sell_noops),
            ("n_ledger_buy_noops", self.n_ledger_buy_noops),
            ("n_osc_domain_rejects", self.n_osc_domain_rejects),
        ]
    }
}

/// 复刻 CPython `round(x, ndigits)`（正确舍入 + round-half-to-even）。
/// 既有先例：macd.rs `round6`（bit-exact 已验证）；Rust `{:.N}` 格式化使用
/// 正确舍入（round-half-to-even），与 CPython 一致。
pub fn py_round(x: f64, ndigits: usize) -> f64 {
    format!("{:.*}", ndigits, x).parse::<f64>().expect("py_round: 格式化往返必然可解析")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bsp_class_roundtrip() {
        // 六类全枚举往返（穷举义务的测试面）
        for class in [
            BspClass::Buy1,
            BspClass::Buy2,
            BspClass::Buy3,
            BspClass::Sell1,
            BspClass::Sell2,
            BspClass::Sell3,
        ] {
            assert_eq!(BspClass::from_parts(class.kind(), class.side()), class);
        }
    }

    #[test]
    fn py_round_banker() {
        // Python: round(0.123456789, 4) = 0.1235; round(2.675, 2)=2.67（二进制表示低于 .675）
        assert_eq!(py_round(0.123456789, 4), 0.1235);
        assert_eq!(py_round(-0.00000001, 6), 0.0); // -0.0 == 0.0
        assert_eq!(py_round(1.00005, 4), 1.0001); // 二进制表示 1.0000500…38，与 CPython 一致
    }

    #[test]
    fn ladder_names() {
        assert_eq!(ladder_name(2), "segment");
        assert_eq!(ladder_name(3), "move(L1)");
        assert_eq!(ladder_name(4), "recL2");
    }

    #[test]
    fn slot_py_keys() {
        assert_eq!(SlotKey::main(2).py_key(), 2);
        assert_eq!(SlotKey::osc(3).py_key(), 103);
        assert_eq!(SlotKey::rev(2, 4).py_key(), 202);
    }

    #[test]
    fn rev_path_keys_and_depth() {
        // 深度 0 = 旧编码逐位投影（O0≡P5 守卫面：py_key/leg_kind 不变）
        let p0 = RevPath::single(3);
        assert_eq!(p0.depth(), 0);
        assert_eq!(SlotKey::rev_path(3, p0), SlotKey::rev(3, 3));
        assert_eq!(SlotKey::rev_path(3, p0).leg_kind(), "rev");
        // 深度 1 子腿：py_key 扩区 +300，leg_kind 新词
        let p1 = p0.child(2);
        assert_eq!(p1.depth(), 1);
        assert_eq!(SlotKey::rev_path(3, p1).py_key(), 303);
        assert_eq!(SlotKey::rev_path(3, p1).leg_kind(), "rev_sub");
        // 路径键互异（同 home 不同路径不混槽）
        assert_ne!(SlotKey::rev_path(3, p1), SlotKey::rev_path(3, p0));
        assert_ne!(SlotKey::rev_path(3, p1), SlotKey::rev_path(3, p0.child(1)));
        // 深度 2：符号交替塔的下一层
        let p2 = p1.child(1);
        assert_eq!(p2.depth(), 2);
        assert_eq!(SlotKey::rev_path(3, p2).py_key(), 403);
    }
}
