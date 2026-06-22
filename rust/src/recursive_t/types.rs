//! 统一递归算子 T 的数据类型（standalone 版，泛型，所有级别通用）。
//!
//! 设计依据：`docs/unified_recursive_operator_T.md` §1.2 类型签名、§1.3 四步骤。
//! 核心原则（第65课 `aₙ=f(aₙ₋₁)`）：所有级别用同一套类型，级别差异只体现在
//! `level` 字段与递归深度，不分叉类型。
//!
//! 架构：standalone——自造中枢/走势/背驰逻辑，不依赖 `crate::level`/`crate::moves`/v3
//! 任何现有引擎，验证 T 四步循环可独立于 v3 nucleus 自洽实现（第65课形式不变性）。

/// 方向（向上 / 向下）。
///
/// 走势方向的唯一源头是 H⁰ 形态学构造轴（设计文档 §0.4）。本模块只做结构识别，
/// 不涉及多空仓位，故方向就是几何方向，不是操作极性。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
}

impl Direction {
    /// 反向。
    pub fn flip(self) -> Direction {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
        }
    }
}

/// 走势完美（步骤c 背驰）判定模式。
///
/// 第24课：「用均线或MACD看背驰都是辅助性的……配合上中枢，那是 100% 绝对的。」
/// 三模式实测对照（编排者 2026-06-18 裁决，escalation `2026-06-18-1752-t-stepc-macd-
/// scope-vs-direction.md` §6.7 选项C「数据划线」）：纯结构 / 结构∧MACD / 结构∨MACD
/// 三路**同数据同操作层**，唯一区别 = 步骤c 力度判据，由 L2/L3 回测甄别 MACD 收紧
/// （AND）/放宽（OR）的有效域——OR放宽/AND收紧是有效域读数，非先验（formalization-
/// validity-domain）。
///
/// 三模式共享**几何门 G**（≥2 中枢 + c 创新高/新低）。在 G 内（设计文档 §6.6 定案）：
/// - `Structural`：G ∧ (F∧S)——纯结构（第37课条件2·3·5 + 结构力度衰减），现状。
/// - `And`：G ∧ (F∧S) ∧ M——结构完美**且** MACD 面积衰减（收紧门，信号单调减）。
/// - `Or`：G ∧ [(F∧S) ∨ M]——结构完美**或** MACD 面积衰减（放宽门，M 可绕过 F∧S，
///   但**不可**绕过 G——BSP 价格/bar 锚定 c 段趋势极值，无创新高则无从定位一类买卖点）。
///
/// 其中 F=第37课条件2·3·5（结构滤网，仅 has_nest 时激活），S=结构力度衰减（嵌套
/// 深度/振幅），M=MACD 面积衰减（C 段 < A 段，按方向 Σ 非负面积）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PerfectionMode {
    /// 纯结构 5 条件（默认，§6.6 纯结构优先）。
    #[default]
    Structural,
    /// 结构 ∧ MACD 面积衰减（收紧）。
    And,
    /// 结构 ∨ MACD 面积衰减（放宽）。
    Or,
}

/// a₀（递归底座）来源（谱系 526 / 第65课 065:182「区别仅在 a0」）。
///
/// 第65课原文5：两者在递归形式上一样，`aₙ=f(aₙ₋₁)`，**唯一不同就是预先给出的 a₀**。
/// 引擎的两个 a₀ 载体，二者都处于「a₀ 确认层」（526号），过滤口径不同：
/// - `Segment`：a₀ = 线段序列，`confirmed && kind==Settled`——现状默认，保 bit-exact。
///   Settled 是线段特有的**特征序列递归确认**语义（第67/71课）。
/// - `Stroke`：a₀ = 笔序列，仅 `confirmed`——递归底座下移（级别数 5-6→8-10 的主因）。
///   笔无 Settled 语义（bi.md:151「最后一笔始终 confirmed=False，直到下一笔生成后才结算」），
///   完成口径 = confirmed（525号「完成在本级别图上可观测、不需下钻」第18课:24 +
///   526号「a₀确认层=操作性意义」）。
///
/// 两层各自的级别增量必须**分别测量**（filter-spec 下游推论1）——故 a₀ 来源是可切换
/// 参数（与 bar_spec 对称），能跑 a0=线段(基线) vs a0=笔(新) A/B 对比，非兼容垫片。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum A0Source {
    /// a₀ = 线段序列（`confirmed && Settled` 过滤）——现状默认，保 bit-exact。
    #[default]
    Segment,
    /// a₀ = 笔序列（`confirmed` 过滤）——递归底座下移（526号 / 第65课:182）。
    Stroke,
}

/// 单元（泛型，所有级别通用）。
///
/// 递归恒等式（谱系 540）：级别 k 的一个完整走势类型 `Move(k)` ≡ 级别 k+1 的一根「笔」。
/// 所以 `Unit` 既表示 a₀ 的笔（level=0），也表示任意级别封装后的上级单元。
///
/// `inner_zhongshu_count`：本单元作为「下级走势封装」时其内部的中枢数量。
/// - a₀ 笔 / 线段：`0`——线段以下是类中枢，无真中枢（第64课）；
/// - level≥1 单元：= 被封装走势的中枢数 ≥ 1。
///
/// 这个字段是结构性背驰（步骤c，选项C「嵌套深度」）的必要输入：扁平的 high/low
/// 无法表达嵌套深度，故必须在封装时把下级中枢数携带上来（设计文档 §6.4 候选「中枢
/// 嵌套深度比较」）。它使背驰判据在 level-0 自然退化为类背驰（几何振幅），在
/// level≥1 升级为真背驰（嵌套深度），与第64课「线段以下用类背驰力度比较」吻合。
#[derive(Debug, Clone, PartialEq)]
pub struct Unit {
    pub high: f64,
    pub low: f64,
    pub start_bar: i64,
    pub end_bar: i64,
    pub direction: Direction,
    /// 递归深度（a₀ 笔 = 0）。
    pub level: usize,
    /// 内部中枢数（嵌套深度，a₀ = 0）。
    pub inner_zhongshu_count: usize,
    /// 本单元覆盖区间的 MACD **红柱（正 hist）累积面积**，约定 `≥0`。
    ///
    /// 步骤c MACD 面积背驰判据（`PerfectionMode::And`/`Or`）的输入。a₀ 层由调用方（ffi）
    /// 经 `merged_to_raw` 转 raw bar 后用 `macd::macd_area_for_range` 注入；无 MACD 数据
    /// 时为 `0.0`（退化为纯结构）。封装时随 `inner_zhongshu_count` 一起**求和上传**
    /// （面积可加性：大走势 MACD 面积 = 内含小单元之和），故 level≥1 单元自然聚合。
    pub area_pos: f64,
    /// 本单元覆盖区间的 MACD **绿柱（负 hist）累积面积的绝对值**，约定 `≥0`。
    ///
    /// 与 `area_pos` 同源同步（注入/求和上传）。注入时取 `macd_area_for_range(...).area_neg`
    /// 的绝对值——本模块统一用非负面积，方向由 `leg_macd_force` 按段所属趋势方向选取
    /// （上涨段 Σarea_pos，下跌段 Σarea_neg），保住面积可加性 + 方向一致性。
    pub area_neg: f64,
}

impl Unit {
    /// 构造一根 a₀ 笔（level=0，无内部中枢，无 MACD 面积）。
    ///
    /// `area_pos`/`area_neg` 默认 `0.0`（纯结构路径）。需 MACD 面积背驰判定时由调用方
    /// 直接构造 `Unit { .., area_pos, area_neg }`（见 `ffi::run_recursive_t`）。
    pub fn stroke(low: f64, high: f64, start_bar: i64, end_bar: i64, direction: Direction) -> Unit {
        Unit {
            high,
            low,
            start_bar,
            end_bar,
            direction,
            level: 0,
            inner_zhongshu_count: 0,
            area_pos: 0.0,
            area_neg: 0.0,
        }
    }
}

/// 中枢。
///
/// 第17课走势中枢递归定义：被至少三个连续次级别走势类型所重叠的部分。
/// 设计文档 §1.3 步骤a 口径（编纂版 + 博文双区间均保留）：
/// - 核心区间 `[low, high] = [ZD, ZG]`：前两段定（编纂版口径），`ZG > ZD` 为成立条件；
/// - 外缘区间 `[dd, gg] = [DD, GG]`：全部参与段定（含全部波动）。
#[derive(Debug, Clone, PartialEq)]
pub struct Zhongshu {
    /// ZG 核心上沿 = min(前两段 high)。
    pub high: f64,
    /// ZD 核心下沿 = max(前两段 low)。
    pub low: f64,
    /// GG 外缘上沿 = max(全部参与段 high)。
    pub gg: f64,
    /// DD 外缘下沿 = min(全部参与段 low)。
    pub dd: f64,
    /// 参与单元的索引（相对于所属走势的 `units` 切片，已重基）。
    pub units: Vec<usize>,
    pub level: usize,
}

/// 走势类型分类（第17课）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrendKind {
    /// 盘整：只含一个中枢。
    Consolidation,
    /// 上涨趋势：≥2 个依次同向（向上）中枢。
    UpTrend,
    /// 下跌趋势：≥2 个依次同向（向下）中枢。
    DownTrend,
}

/// 买卖点种类（三类 × 买/卖）。
///
/// 完备性（第37课）：所有买卖点归根结底都是第一类，只是级别不同。本模块据此把
/// type1 作为「迭代不变量」（步骤c 直接涌现），type2/type3 是它在递归塔里的投影。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BSPKind {
    Type1Buy,
    Type1Sell,
    Type2Buy,
    Type2Sell,
    Type3Buy,
    Type3Sell,
}

impl BSPKind {
    /// 是否买点。
    pub fn is_buy(self) -> bool {
        matches!(
            self,
            BSPKind::Type1Buy | BSPKind::Type2Buy | BSPKind::Type3Buy
        )
    }

    /// 类型字符串（FFI / 对照用）。
    pub fn as_str(self) -> &'static str {
        match self {
            BSPKind::Type1Buy => "type1_buy",
            BSPKind::Type1Sell => "type1_sell",
            BSPKind::Type2Buy => "type2_buy",
            BSPKind::Type2Sell => "type2_sell",
            BSPKind::Type3Buy => "type3_buy",
            BSPKind::Type3Sell => "type3_sell",
        }
    }
}

/// 买卖点（T 迭代的伴随不变量，不是独立信号）。
#[derive(Debug, Clone, PartialEq)]
pub struct BSP {
    pub kind: BSPKind,
    pub bar: i64,
    pub price: f64,
    pub level: usize,
}

/// 走势类型实例。
///
/// 双重性约束（谱系 540/537，设计文档 §1.3）：封装不可坍缩。`TrendType` 同时保留
/// - 压缩视图（构造用）：`direction` + 区间（封装为上级单元时用）；
/// - 展开视图（确认用）：`zhongshus` + `units` + `bsp`（内部完整结构）。
#[derive(Debug, Clone, PartialEq)]
pub struct TrendType {
    pub kind: TrendKind,
    pub zhongshus: Vec<Zhongshu>,
    /// 构成本走势的单元（级别 k 单元，已克隆为自包含切片）。
    pub units: Vec<Unit>,
    /// 本走势的级别 = 输入单元的级别。
    pub level: usize,
    pub direction: Direction,
    /// 是否终完美（被背驰确认或被反向走势终结）。
    pub completed: bool,
    /// 走势完美时产生的买卖点（type1）。
    pub bsp: Option<BSP>,
}

/// T 在单个级别迭代一次的输出（纯函数返回值，无副作用）。
#[derive(Debug, Clone, PartialEq)]
pub struct TLevelOutput {
    /// 本次迭代的级别 k（输入单元的级别）。
    pub level: usize,
    /// 步骤a 识别出的中枢。
    pub centers: Vec<Zhongshu>,
    /// 步骤b 识别出的走势类型实例。
    pub trends: Vec<TrendType>,
    /// 本级别涌现的买卖点（type1 来自步骤c，type3 来自步骤a 派生）。
    pub bsps: Vec<BSP>,
    /// 步骤d 封装产出的级别 k+1 单元序列 `S_{k+1}`。
    pub next_units: Vec<Unit>,
}

/// 整个递归塔（Tᵏ 迭代到 r* 的结果）。
#[derive(Debug, Clone, PartialEq)]
pub struct RecursiveTree {
    /// 每个级别的输出，`levels[0]` 是 T 作用于 a₀（笔序列）的结果。
    pub levels: Vec<TLevelOutput>,
}

impl RecursiveTree {
    /// 涌现上界 r*：已形成**完整走势**（终完美 completed）的最高级别（设计文档 §1.5）。
    /// = 最后一个含至少一个 completed 走势的级别索引 + 1；空塔返回 0。
    ///
    /// 用 `completed` 而非「trends 非空」：一个级别可有生长中走势（completed=false）但
    /// 尚未终完美（§1.5 局部不动点：M_k.confirmed=true 才冻结），这种级别不计入 r*。
    pub fn emergent_ceiling(&self) -> usize {
        self.levels
            .iter()
            .rposition(|l| l.trends.iter().any(|t| t.completed))
            .map(|i| i + 1)
            .unwrap_or(0)
    }

    /// 涌现上界单元的 `(level, 方向)`：最高已诞生上级单元的级别 + 该级别**最新单元方向**。
    /// 空塔 / 无 completed 走势 → `None`。
    ///
    /// 自下而上仓位涌现（H¹ 升级归属）的结构信号：`emergent_ceiling` 只给级别，本方法额外
    /// 给方向（= 最高 completed 级别**最后一个** completed 走势封装出的上级单元方向）。操作层
    /// 据此把核心仓 relabel 到对应 ladder（仅当核心仓操作极性与该方向一致时）——**不依赖该
    /// 级别 BSP fire**，故能在高级别买卖点尚未涌现时就让低级别仓位升级归属为高级别核心仓。
    ///
    /// 返回的 `level` = 最高 completed 级别 `i` + 1（封装恒等式 `Move(i)≡Level-(i+1) 笔`），
    /// 与 `emergent_ceiling()` 数值一致；方向取 `levels[i].next_units` 最后一根（= 最新封装）。
    pub fn emergent_top(&self) -> Option<(usize, Direction)> {
        let i = self
            .levels
            .iter()
            .rposition(|l| l.trends.iter().any(|t| t.completed))?;
        let last = self.levels[i].next_units.last()?;
        Some((last.level, last.direction))
    }

    /// 收集全塔所有买卖点（含跨级 type2 投影）。
    pub fn all_bsps(&self) -> Vec<BSP> {
        self.levels.iter().flat_map(|l| l.bsps.iter().cloned()).collect()
    }
}
