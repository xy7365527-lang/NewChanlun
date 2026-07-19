//! 区间套有限递归证书 χ（reference-theta-v0.md；契约重锚 `Origin.SubLevelDescent`）。
//!
//! ## 契约重锚（legacy Strict/Nest → `Origin.SubLevelDescent`）
//!
//! - 下钻递归 ↔ `Origin.SubLevelDescent.descend : RMove → List RMove`（compose 的逆，segment → []，
//!   `descend_level_decreases` 证下钻级别严格降 well-founded 终止）。
//! - 区间套关系 ↔ `Sub`：`J'.startTime>=J.startTime ∧ J'.endTime<=J.endTime`（J' 套在 J 内）。
//! - 子级别破中枢 ↔ `Origin.SubLevelDescent.{SubBrokeBelow,SubBrokeAbove,subLevelHasBrokenCenter}`。
//! - `Sel_Θ` 选择器 ↔ `selKey`：`(endTime, startTime, idx)` 字典序（canonical tie-break）。
//! - 终端确认 ↔ `Confirm`：`Λ≠∅`（至少一类买卖点成立，**不要求 |Λ|=1**——2/3 类可共存）。
//! - 方向化区间套证书 ↔ `N^δ_{ℓ↓e}`（spec P5 §6 line 1168）：χ 的**方向化 + 候选化精化**——基例
//!   用方向化 `Conf^δ_e`（[`BspBits::confirm_side`]），递归步追加候选谓词 `Cand^δ_ℓ`（[需人工确认]
//!   定义式，spec 疑点2）+ 闭包含（旧链为 J 子⊆父；严格装配为 `I(A_child)⊆D_parent`，
//!   复用 [`is_sub`]）。见 [`NestCertificate`]。
//!
//! ## 核心定理（契约锚 `Origin.SubLevelDescent.descend_level_decreases`）
//!
//! 在 `Sel_Θ` 固定下，区间套递归证书的定位见证（各级 chosen 键序列）唯一；下钻级别严格降
//! （`descend_level_decreases`）⟹ 有限递归终止。唯一性依赖 `Sel_Θ`——缠论结构公理单独给不出
//! 唯一定位（需选择器固定）。
//!
//! ## 认识论（formalization-validity-domain）
//!
//! 全部 L0：选择器是确定字典序，区间套是确定整数比较，确认是 bit-vector 非空判定。
//! 唯一性是纯结构归纳（同义反复，不冒充 L1+）。`Sel_Θ` 规则由 Θ_signal 给出（reference:24
//! canonical 分解 tie-break：最早确认时间 → 最低递归层 → 最早原始 index）。

use super::super::types::{BspBits, Side};
use super::bsp::BspPoint;
use super::level_view::{NestCandidateEvent, NestDivergenceKind};
use super::recursive_tower::{cp_terminal_certificate, CandDeltaEvent, CpScanOwnership};
use super::Classification;

/// 候选定位区间（契约锚 `Origin.SubLevelDescent` 下钻区间）——携带 `Sel_Θ` 排序三键。
///
/// 几何上下界由下游 `Sub` 契约（区间套缩小），本结构暴露排序所需三键，使「选择器固定
/// candidate」可机器检查。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NestInterval {
    /// 结束时间（第一排序键，最新者优先 ⟹ 数值最大优先）。
    pub end_time: u64,
    /// 开始时间（第二排序键，最新者优先 ⟹ 数值最大优先）。
    pub start_time: u64,
    /// 编号（第三排序键，最小者优先 ⟹ 数值最小优先）。
    pub idx: u64,
}

impl NestInterval {
    /// `Sel_Θ` 字典序键（reference:对齐 `selKey`）：`(end_time, start_time, idx)`。
    pub fn sel_key(&self) -> (u64, u64, u64) {
        (self.end_time, self.start_time, self.idx)
    }
}

/// `Sel_Θ` 严格序（reference:对齐 `selOrder`）：a 严格优于 b。
///
/// 逐字对齐 Lean：endTime 大优先；endTime 相等则 startTime 大优先；前两相等则 idx 小优先。
pub fn sel_order(a: &NestInterval, b: &NestInterval) -> bool {
    a.end_time > b.end_time
        || (a.end_time == b.end_time && a.start_time > b.start_time)
        || (a.end_time == b.end_time && a.start_time == b.start_time && a.idx < b.idx)
}

/// 区间套关系 `Sub`（reference:对齐 `Sub`）：`J'` 套在 `J` 之内（逐级缩小定位）。
///
/// `inner.start_time>=outer.start_time ∧ inner.end_time<=outer.end_time`。
pub fn is_sub(inner: &NestInterval, outer: &NestInterval) -> bool {
    inner.start_time >= outer.start_time && inner.end_time <= outer.end_time
}

/// 从候选集合按 `Sel_Θ` 选出唯一最优候选（reference:对齐 `IsSelected`）。
///
/// 返回在 `selOrder` 下严格优于所有其他候选（或键相同）的那一个。`cands` 为空 ⟹ `None`
/// （无候选无定位）。bit-exact：用 `sel_order` 全序遍历选最优——每对比较确定。
///
/// 边界条件（Θ-参数化前件）：若 `cands` 含两个键不同但互不严格优的候选，`sel_order` 是
/// 严格全序（三键字典序），不会出现该情况——选择唯一（`selected_key_unique`）。
pub fn select_best(cands: &[NestInterval]) -> Option<NestInterval> {
    let mut best: Option<NestInterval> = None;
    for &cand in cands {
        match best {
            None => best = Some(cand),
            Some(b) => {
                // cand 严格优于当前 best ⟹ 替换（键相同则保留——sel_order 严格序无平局歧义）。
                if sel_order(&cand, &b) {
                    best = Some(cand);
                }
            }
        }
    }
    best
}

/// 区间套链一级节点（reference:对齐 `LevelNode`）。
#[derive(Debug, Clone, PartialEq)]
pub struct LevelNode {
    /// 该级别候选区间集合。
    pub cands: Vec<NestInterval>,
    /// `Sel_Θ` 从 `cands` 选出的候选。
    pub chosen: NestInterval,
}

/// 终端确认（reference:对齐 `Confirm`）：bit-vector 至少一类买卖点成立（`Λ≠∅`）。
///
/// **不要求 |Λ|=1**（Nest.lean `confirm_of_two_three`/`confirm_of_all_three`）——2/3 类
/// 共存同样确认。唯一被排除的是全零 bit-vector（无任何买卖点成立的终端）。
///
/// ★方向无关 = `Conf^+ ∨ Conf^-`（spec P5 §6）：`Λ≠∅` 恰为「买侧确认 [`BspBits::conf_plus`] 或
/// 卖侧确认 [`BspBits::conf_minus`]」的析取。委托两方向谓词（单一来源，消除重复析取）；区间套证书
/// `N^δ_{ℓ↓e}` 的**方向化**基例用 [`BspBits::confirm_side`]，本函数是其双向并集。
pub fn confirm(terminal: &BspBits) -> bool {
    terminal.conf_plus() || terminal.conf_minus()
}

/// 最终确认证书 χ（reference:对齐 `Chi`）——区间套递归证书 + 终端确认。
///
/// `chain` 操作级→执行级的级别链（ℓ₀ 表头，ℓ_k 表尾=执行级）；`terminal` 执行级 bit-vector。
#[derive(Debug, Clone, PartialEq)]
pub struct Chi {
    pub chain: Vec<LevelNode>,
    pub terminal: BspBits,
}

impl Chi {
    /// 区间套递归证书良构（reference:对齐 `NestCertificate`）：逐级相邻区间套 ⊆。
    ///
    /// 每对相邻级别 `chosen` 满足 `is_sub(下级, 上级)`（区间套逐级缩小）。空链/单级平凡成立。
    pub fn nest_valid(&self) -> bool {
        self.chain
            .windows(2)
            .all(|w| is_sub(&w[1].chosen, &w[0].chosen))
    }

    /// χ 整体确认（reference:对齐 `Chi`）：递归证书良构 ∧ 终端确认。
    pub fn is_confirmed(&self) -> bool {
        self.nest_valid() && confirm(&self.terminal)
    }

    /// 定位见证键序列（reference:对齐 `locatorKeys`）：各级 `chosen` 的 `selKey`。
    pub fn locator_keys(&self) -> Vec<(u64, u64, u64)> {
        self.chain.iter().map(|n| n.chosen.sel_key()).collect()
    }
}

/// 方向化区间套证书 `N^δ_{ℓ↓e}` 的递归梯级（spec P5 §6 递归步 ℓ>e 的单级数据）。
///
/// 对应 spec 递归步 `Cand^δ_ℓ(x) ∧ [J^δ_{ℓ-1}(x)⊆J^δ_ℓ(x)] ∧ N^δ_{ℓ-1↓e}` 中本级 ℓ 的两个分量：
/// - `interval` = 本级定位区间 `J^δ_ℓ`（与**子级** `J^δ_{ℓ-1}` 做闭口径 ⊆ 比较，复用 [`is_sub`]）。
/// - `cand` = 候选谓词 `Cand^δ_ℓ(x) ∈ {0,1}` 的**取值**（不是定义式）。
///
/// [需人工确认]（spec 疑点2，line 1257）：`Cand^δ_ℓ(x)` 在 PDF 中仅作符号出现，**无独立定义式**。
/// 本结构按 spec 把 `Cand^δ_ℓ(x)` 当作 0/1 谓词**取值**消费（与 `b_ℓ∈{0,1}^6` 同样是判定结果输入），
/// 其计算规则属上游（候选判据定义）——`N^δ` 只做合取组装，**不臆造** Cand 的判据。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NestRung {
    /// [新缠论] 本级事件的算法确认时点（#37 P0 局部改判）。与 `interval.end_time` 独立；
    /// 二者可以数值相等，但不互相派生。旧证书链没有逐级确认见证时为 `None`，不得伪造。
    confirm_src: Option<usize>,
    /// 严格 D_parent 装配边的子级 `I(A)`；旧数据载体路径没有边见证时为 `None`。
    child_interval: Option<NestInterval>,
    /// 本级父区间：严格装配为 `D_parent`；旧数据载体路径为 `J^δ_ℓ`。
    interval: NestInterval,
    /// `Cand^δ_ℓ(x) ∈ {0,1}` 取值。[需人工确认] 定义式（spec 疑点2）。
    cand: bool,
}

impl NestRung {
    /// 梯级只允许 crate 内受信生产器构造；外部调用方只能读取。
    /// `cand` 仍是 0/1 谓词**取值**输入（裁决① `Cand^δ` 定义式在 recursive_tower/cand_predicate
    /// 上游），本构造器不重判、不臆造判据。
    pub(crate) fn new(interval: NestInterval, cand: bool) -> Self {
        Self { confirm_src: None, child_interval: None, interval, cand }
    }
    /// 严格装配路径构造：保留独立确认时点及边的 `I(A_child) ⊆ D_parent` 见证。
    fn assembled(
        confirm_src: usize,
        child_interval: NestInterval,
        interval: NestInterval,
        cand: bool,
    ) -> Self {
        Self {
            confirm_src: Some(confirm_src),
            child_interval: Some(child_interval),
            interval,
            cand,
        }
    }
    /// 本级事件的算法确认时点；旧数据载体路径未携带见证时为 `None`。
    pub fn confirm_src(&self) -> Option<usize> {
        self.confirm_src
    }
    /// 严格装配边的子级 `I(A)`；旧数据载体路径为 `None`。
    pub fn child_interval(&self) -> Option<NestInterval> {
        self.child_interval
    }
    /// 本级父区间：严格装配为 `D_parent`；旧数据载体路径为 `J^δ_ℓ`。
    pub fn interval(&self) -> NestInterval {
        self.interval
    }
    /// `Cand^δ_ℓ(x) ∈ {0,1}` 取值。
    pub fn cand(&self) -> bool {
        self.cand
    }
}

/// 方向化区间套证书 `N^δ_{ℓ↓e}(x) ∈ {0,1}`（spec P5 §6 line 1168 / 结果包 line 308）。
///
/// `Chi`（χ）的**方向化 + 候选化精化**：χ 用方向无关 [`confirm`] 且无 `Cand`；本证书 (a) 基例改用
/// 方向化 [`BspBits::confirm_side`]（按 δ 选 `conf_plus`/`conf_minus`），(b) 每个递归级追加
/// `Cand^δ_ℓ` 合取。区间套关系仍复用契约锚 [`is_sub`]（不重造区间逻辑）。
///
/// 外部调用方不能用字段字面量或旁路构造器伪造证书；唯一公开生产路径是
/// [`assemble_certificate`] / [`assemble_certificates`]。
///
/// ```compile_fail
/// use newchan_rust::theta_v0::classifier::nest::{NestCertificate, NestInterval, NestRung};
/// use newchan_rust::theta_v0::types::{BspBits, Side};
/// let iv = NestInterval { start_time: 0, end_time: 1, idx: 0 };
/// let _ = NestCertificate {
///     side: Side::Long,
///     terminal: BspBits::default(),
///     base_interval: iv,
///     rungs: vec![NestRung { interval: iv, cand: true }],
/// };
/// ```
///
/// ```compile_fail
/// use newchan_rust::theta_v0::classifier::nest::{NestCertificate, NestInterval};
/// use newchan_rust::theta_v0::types::{BspBits, Side};
/// let iv = NestInterval { start_time: 0, end_time: 1, idx: 0 };
/// let _ = NestCertificate::from_parts(Side::Long, BspBits::default(), iv, vec![]);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct NestCertificate {
    /// 方向 δ（`Long`=+1 / `Short`=-1），固定于整条证书。
    side: Side,
    /// 执行级 e 终端 bit-vector（基例 `Conf^δ_e` 的输入）。
    terminal: BspBits,
    /// [新缠论] 执行级 e 事件的算法确认时点（#37 P0 局部改判）。严格装配时直接取事件字段，
    /// 不从定位区间右端回填；不承载该见证的旧证书链为 `None`。
    base_confirm_src: Option<usize>,
    /// 最低递归级的子包含区间；严格装配为 `I(A_e)`，旧数据载体路径为 `J^δ_e`。
    base_interval: NestInterval,
    /// 递归级 `(e, ℓ]` 梯级，**从高到低**排列：`rungs[0]`=级 ℓ，`rungs[last]`=级 e+1。
    /// 空 ⟹ ℓ=e（纯基例）。结构上不可表达 e>ℓ（无负梯级），天然满足前置约束 e≤ℓ。
    rungs: Vec<NestRung>,
}

impl NestCertificate {
    /// crate 内数据载体入口；外部不可见，最终统一经过私有 builder 收口。
    ///
    /// 本构造器是**数据载体**入口（π 结构路径 `econ_positive::build_nest_certificate`、测试用）：
    /// - **不**宣称装配前置成立（方向逐级来源、级别连续性、rung 与
    ///   `CandDeltaEvent` 身份对应）——那些是 [`assemble_certificate`]
    ///   三门 DFS 的生产层职责，证书本体无字段可复验（诚实边界，见 [`Self::n_delta`] 文档）。
    /// - `n_delta()` 仍是 0/1 谓词**取值**：构造成功 ≠ `n_delta=true`（π 路径合法携带
    ///   `cand=false` 梯级供 `effective_nest_depth` 截断消费）。
    /// - debug 断言结构上可复验的一门：定位区间链逐级相套 `J_e⊆…⊆J_ℓ`（闭口径 [`is_sub`]，
    ///   与 econ_positive #100 看守同款；release 编译掉，零行为改动）。
    pub(crate) fn from_parts(
        side: Side,
        terminal: BspBits,
        base_interval: NestInterval,
        rungs: Vec<NestRung>,
    ) -> Self {
        NestCertificateBuilder { side, terminal, base_confirm_src: None, base_interval, rungs }
            .finish()
    }

    /// 方向 δ（固定于整条证书）。
    pub fn side(&self) -> Side {
        self.side
    }
    /// 执行级 e 终端 bit-vector。
    pub fn terminal(&self) -> &BspBits {
        &self.terminal
    }
    /// 执行级 e 事件的算法确认时点；旧数据载体路径未携带见证时为 `None`。
    pub fn base_confirm_src(&self) -> Option<usize> {
        self.base_confirm_src
    }
    /// 最低递归级的子包含区间（严格装配为 `I(A_e)`）。
    pub fn base_interval(&self) -> NestInterval {
        self.base_interval
    }
    /// 递归级梯级（从高到低）。
    pub fn rungs(&self) -> &[NestRung] {
        &self.rungs
    }

    /// ★诚实边界（cert F-01）：`n_delta` 只能复验证书**自身字段**（逐级 `cand` 取值 ∧ 相邻 ⊆ ∧
    /// 基例 `Conf^δ_e`）——装配层前置（方向逐级来源、级别连续性、rung 与
    /// `CandDeltaEvent` 的身份对应）证书无字段可复验，`n_delta=true` **不**蕴含它们成立；它们由
    /// [`assemble_certificate`] 三门 DFS 在生产时保证（数据载体旁路见 [`Self::from_parts`]）。
    ///
    /// 区间套证书 `N^δ_{ℓ↓e}(x) ∈ {0,1}`：按 ℓ 从高到低逐级校验。
    ///
    /// ## 结果包（六要素）
    /// - **结论**：返回 0/1（bool）值的级别递归谓词。基例 ℓ=e（`rungs` 空）取方向化确认
    ///   `Conf^δ_e(x)`=[`BspBits::confirm_side`]；递归步合取 `Cand^δ_ℓ(x)` 与本级闭包含边。
    ///   严格装配边取 `I(A_child)⊆D_parent`；旧数据载体边保持 `J_child⊆J_parent`。
    /// - **定义依据**：spec `2026-06-28-recursive-complete-classification-bsp-pdf-extract.md`
    ///   P5 §6 分段方框（line 277-285）+ ∃! 方框（line 295）+ 结果包（line 308-312）。`side` 选 δ；
    ///   `terminal` 满足基例 `Conf^δ_e=⋁ B_{i,e}`/`⋁ S_{i,e}`；`rungs[k].cand` 提供 `Cand^δ`；
    ///   相邻 `interval` 满足 ⊆。
    /// - **边界条件**：(1) ⊆ 方向是**子⊆父** `J^δ_{ℓ-1}⊆J^δ_ℓ`（[`is_sub`]：child.lo≥parent.lo
    ///   ∧ child.hi≤parent.hi，闭口径；lo=`start_time`/hi=`end_time`），方向反则证书失效。
    ///   (2) 任一级 `Cand^δ_ℓ=false`、或任一相邻 ⊆ 不成立、或基例 `Conf^δ_e=false` ⟹ 整体翻转为 0。
    ///   (3) 方向 δ 翻转（Long↔Short）则基例改判 `conf_plus`↔`conf_minus`，结论可翻转。
    ///   (4) 前置 e≤ℓ 由结构保证（rungs 非负长度），e>ℓ 不可表达（spec：e>ℓ 递归未定义）。
    /// - **下游推论**：N^δ 是 Γ 候选集→确认信号提升（环2→环3）的方向化确认层；返回 bool ⟹ ∃!
    ///   ∈{0,1}（2 值确定函数，无需 Finset）；级别每步严格下降（rungs 缩短）⟹ 有限终止。
    ///   Lean 端对应 (ℓ-e):Nat 结构递归。
    /// - **谱系引用**：第三类边界谱系（MEMORY: theta-v0-type3-boundary）——基例只消费
    ///   [`BspBits::confirm_side`]（委托 `conf_plus`/`conf_minus` → buy3/sell3 bit），B3 边界已在
    ///   `bsp::endpoint_to_bsp`（rust `>=`）结算，本层**不重判边界**。`Cand^δ_ℓ` 定义式缺失见
    ///   spec 疑点2（[需人工确认]）。
    /// - **影响声明**：在 nest.rs 新增方向化区间套证书；不改 `Chi`/`confirm`/`is_sub`（契约锚保留），
    ///   不动 classifier/mod.rs。L0 操作语义结构（非 L2 alpha）。
    pub fn n_delta(&self) -> bool {
        Self::n_delta_rec(self.side, &self.terminal, &self.base_interval, &self.rungs)
    }

    /// `N^δ` 的级别递归核（mirror Lean (ℓ-e):Nat 结构递归）。`rungs` 从高(ℓ)到低(e+1)。
    fn n_delta_rec(side: Side, terminal: &BspBits, base: &NestInterval, rungs: &[NestRung]) -> bool {
        match rungs.split_first() {
            // 基例 ℓ=e：N^δ_{e↓e} = Conf^δ_e。
            None => terminal.confirm_side(side),
            // 递归步 ℓ>e：Cand^δ_ℓ ∧ [J^δ_{ℓ-1} ⊆ J^δ_ℓ] ∧ N^δ_{ℓ-1↓e}。
            Some((top, rest)) => {
                // 子级 J^δ_{ℓ-1}：下一梯级区间；rest 空 ⟹ 子级是执行级 J^δ_e。
                let child = top
                    .child_interval
                    .as_ref()
                    .unwrap_or_else(|| rest.first().map_or(base, |r| &r.interval));
                top.cand
                    && is_sub(child, &top.interval)
                    && Self::n_delta_rec(side, terminal, base, rest)
            }
        }
    }
}

/// 私有最终 builder：所有 crate 内旁路数据载体构造也只能在此处落成；外部 API 无法命名或调用。
struct NestCertificateBuilder {
    side: Side,
    terminal: BspBits,
    base_confirm_src: Option<usize>,
    base_interval: NestInterval,
    rungs: Vec<NestRung>,
}

impl NestCertificateBuilder {
    fn finish(self) -> NestCertificate {
        debug_assert!(
            self.rungs
                .iter()
                .enumerate()
                .all(|(i, rung)| {
                    let child = rung.child_interval.as_ref().unwrap_or_else(|| {
                        self.rungs
                            .get(i + 1)
                            .map_or(&self.base_interval, |next| &next.interval)
                    });
                    is_sub(child, &rung.interval)
                }),
            "cert F-01 builder：嵌套边破裂 I(A_child)⊆D_parent：base={:?} rungs={:?}",
            self.base_interval,
            self.rungs
        );
        NestCertificate {
            side: self.side,
            terminal: self.terminal,
            base_confirm_src: self.base_confirm_src,
            base_interval: self.base_interval,
            rungs: self.rungs,
        }
    }
}

/// #92 A/B 区间口径。生产路径固定用 B；A 只允许并行诊断与迁移前基线对账。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NestIntervalCaliber {
    /// leave→retest 全跨度（诊断口径）。
    A,
    /// 完整背驰段 C 的结构区间（裁定生产口径）。
    B,
}

/// #97 身份标签：链上每级实际选用的 provider 事件身份，按高→低排列并包含基例，
/// 与 [`TypedNestCertificate::kinds`] 一一对齐。只作归因/对账 sidecar，不参与证书真值。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NestEventIdentity {
    pub level: u32,
    pub turn_source: usize,
    pub interval_b: (usize, usize),
}

impl NestEventIdentity {
    /// p118 关④：`turn_class` 派生层需由事件构造身份（DeferOrphan 主键），提升可见性。
    pub fn of(event: &NestCandidateEvent) -> Self {
        Self {
            level: event.level,
            turn_source: event.turn_source,
            interval_b: event.interval_b,
        }
    }
}

/// #92 typed 证书：证书真值仍由 [`NestCertificate`] 单一来源复验，类型与首次可证钟
/// 作为不可改判的 sidecar 随链保存。`kinds`/`judge_at`/`identities`/`confirmed` 均按
/// 高→低排列并包含基例。
#[derive(Debug, Clone, PartialEq)]
pub struct TypedNestCertificate {
    certificate: NestCertificate,
    kinds: Vec<NestDivergenceKind>,
    judge_at: Vec<usize>,
    identities: Vec<NestEventIdentity>,
    /// p118 关④ sidecar：链上每级 `divergence_confirmed` 真值（高→低，含基例），与
    /// [`Self::identities`] 一一对齐。装配环在断链点（`extend_typed_upward` rung push 同位）
    /// 恢复的 rung 级力度真值——只供 `turn_class` 纯读派生小转大分类，不进证书真值
    ///（`NestCertificate`/`n_delta` 不动），不进 p92 去重键（`certificate_key` 不读本字段）。
    confirmed: Vec<bool>,
    caliber: NestIntervalCaliber,
}

impl TypedNestCertificate {
    pub fn certificate(&self) -> &NestCertificate {
        &self.certificate
    }

    pub fn kinds(&self) -> &[NestDivergenceKind] {
        &self.kinds
    }

    pub fn judge_at(&self) -> &[usize] {
        &self.judge_at
    }

    /// #97 身份标签（高→低，含基例），与 [`Self::kinds`] 对齐。
    pub fn identities(&self) -> &[NestEventIdentity] {
        &self.identities
    }

    /// p118 关④ sidecar：链上每级 `divergence_confirmed`（高→低，含基例），与
    /// [`Self::identities`] 对齐。`confirmed()[0]` = 链顶 rung 力度真值（小转大判定的
    /// 断链恢复点，施工图 D-1）；`confirmed().last()` = 基例真值（基例门恒 true）。
    pub fn confirmed(&self) -> &[bool] {
        &self.confirmed
    }

    pub fn caliber(&self) -> NestIntervalCaliber {
        self.caliber
    }

    /// D3 sidecar：父级首次可证钟应不晚于子级。这里只计数，不参与证书真值。
    pub fn d3_descent_stats(&self) -> (usize, usize) {
        let edges = self.judge_at.len().saturating_sub(1);
        let violations = self.judge_at.windows(2).filter(|pair| pair[0] > pair[1]).count();
        (edges, violations)
    }
}

fn typed_interval(event: &NestCandidateEvent, caliber: NestIntervalCaliber) -> NestInterval {
    let (start, end) = match caliber {
        NestIntervalCaliber::A => event.interval_a,
        NestIntervalCaliber::B => event.interval_b,
    };
    NestInterval {
        start_time: start as u64,
        end_time: end as u64,
        idx: event.turn_source as u64,
    }
}

// ═════════ p117 T1 终端背书级别对齐（bsp-terminal-endorsement-ruling-20260718）═════════

/// nest level-ℓ 事件 → 终端背书 BSP 账本级别（p117 T1 裁定1：级别移位 ℓ→ℓ-1）。
///
/// 事件原子 = tower[ℓ] 窗携带核 = `levels[ℓ-1]` 中枢（S1a 图 §1.4 事实链），其趋势背驰
/// 级别 = `levels[ℓ-1]` 中枢链级别（037:16 趋势级别=中枢级别）⟹ 合法背书账本 =
/// `levels[ℓ-1].bsp`（024:18 背驰必制造其所在级别的买卖点；043:26 查找只能向下）。
/// nest 桶号 ℓ 与 BSP 级别 ℓ 的恒等贴标判定为级别混配装配工件，不是谓词语义
/// （fiat 反读已驳回，裁定 T1.3）。
///
/// ℓ≥1 ⟹ Some(ℓ-1)；ℓ=0 ⟹ None（nest 不听 L0，p105 §3）。exec≥2 同构适用（裁定 T1.5）。
pub fn event_bsp_book_level(event_level: u32) -> Option<usize> {
    match event_level {
        0 => None,
        level => Some(level as usize - 1),
    }
}

/// 终端背书坐标口径（p117 T1 裁定2：生产 = C-b；C-a 保留为敏感性对照，不实装为默认）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalMatch {
    /// C-a 精确坐标：`source_index == turn_source`（旧位格等式平移一级）。敏感性对照专用，
    /// 不作生产默认（预测差 = 5 延迟案，验收报告须落 C-a/C-b 对照读数）。关③ P3.1：本臂
    /// 维持诊断常数，不收紧、不入生产默认（原裁定 T1.2 口径不变）。
    Exact,
    /// C-b 窗口最早**合法**点：`[c_start, t*]`（= `interval_b.0..=turn_source`）内、满足
    /// 事件族点类限定的最早 confirm_side 点。点类分域（关③ P1/P2，
    /// pan-terminal-endorsement-ruling-20260718）：`Trend` = 破 B 一类点（buy1|sell1 ∧
    /// owner=B——判定式 `point.center.start_index == b_center_start`，start_index 判同，
    /// 禁全字段等式，p117 §7）；`Consolidation` = 同向一/二/三类全合法（`confirm_side`
    /// 即精确语义，零改动）。本注释取代旧的「窗口内首个破核点的判决中枢必 = B」结构身份
    /// 保证——该保证已被 T1 复议证伪作废（bsp-terminal-endorsement-t1-review-20260718:92，
    /// frontier 吸收 37/37），不得复引。方向由 `confirm_side` 锁死。
    CWindow,
}

/// 终端背书判定结构（关③ P3.2：`terminal_bits_at_event` 返回形状由 `Option<BspBits>`
/// 扩为携带点身份——bits + 命中点 owner 中枢 start_index；属同函数语义内扩展，
/// 消费方同步，不构成第二查法）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalEndorsement {
    /// 命中点的买卖点 bit 向量（方向已由 `confirm_side` 按事件 side 锁死）。
    pub bits: BspBits,
    /// 命中点 owner 中枢的 `start_index`（`BspPoint.center.start_index`；`None` = 点无
    /// center——★owner 载体补齐（关③ 补记 2026-07-18 ②，路径 (a)）后，生产一/二/三类点
    /// center 均恒 `Some`（构造时填入判定中枢，signal.rs `make_first_point`/
    /// `make_second_point`/`make_third_point` 不变量）；`None` 仅剩非生产合成形状）。
    /// Trend 域合法命中按构造恒 `Some(b_center_start)`（owner=B 是合法合取，判定只比 start_index）。
    pub owner_center_start: Option<usize>,
}

/// 单本账上的终端背书查法核（单一来源；账本只读，不改 `BspPoint` 任何字段/排序/去重）。
///
/// 窗口为 `c_start..=turn_source` 闭区间。`CWindow` 取窗口内**合法**点中 `source_index`
/// 最小者（`min_by_key` 平局取迭代序首个，确定性）——**合法性过滤先于最早性取点**
///（关③ P3.1：最早性只是确定性规则，二/三类点的在场不影响其后一类点的合法性，
/// 044:30 延迟常态；「最早合法」≠「最早」）。合法谓词按事件族 `kind` 分域（关③
/// P1/P2/P3，收紧唯一落点，调用点零限定、禁 fork）：
/// - `Trend`：`confirm_side(side) ∧ (buy1|sell1) ∧ owner=B`，owner 判定式 =
///   `point.center.start_index == b_center_start`（start_index 判同——延伸只改写
///   end/dd/gg，start_index 与 zd/zg 稳定，p117 §7 实测 37/37；**禁全字段 `Center`
///   等式**，已证结构性必败）。`b_center_start = None`（旧事件路径 B 身份缺失）⟹
///   无合法点（身份合取不可证，诚实判负）。
/// - `Consolidation`：`confirm_side(side)` 即精确语义（同向一/二/三类全合法，一类非
///   必需，P2 裁决 5 零改动）；`b_center_start` 不入判（pan 同写仅归因用途，不作门）。
///
/// `Exact` 为位格等式 + confirm_side 的首个迭代序命中（旧查法语义平移；诊断常数不收紧）。
/// 窗口内无合法点 ⟹ None——037:18 否则条款域的诚实判负，非放宽、非误杀。
#[allow(clippy::too_many_arguments)]
pub fn terminal_bits_in_book(
    bsp: &[BspPoint],
    c_start: usize,
    turn_source: usize,
    side: Side,
    kind: NestDivergenceKind,
    b_center_start: Option<usize>,
    m: TerminalMatch,
) -> Option<TerminalEndorsement> {
    let endorsement = |point: &BspPoint| TerminalEndorsement {
        bits: point.bits,
        owner_center_start: point.center.map(|c| c.start_index),
    };
    match m {
        TerminalMatch::Exact => bsp
            .iter()
            .find(|point| point.source_index == turn_source && point.bits.confirm_side(side))
            .map(endorsement),
        TerminalMatch::CWindow => bsp
            .iter()
            .filter(|point| {
                c_start <= point.source_index
                    && point.source_index <= turn_source
                    && point.bits.confirm_side(side)
                    && match kind {
                        NestDivergenceKind::Consolidation => true,
                        NestDivergenceKind::Trend => {
                            (point.bits.buy1 || point.bits.sell1)
                                && b_center_start.is_some_and(|b| {
                                    point.center.is_some_and(|c| c.start_index == b)
                                })
                        }
                    }
            })
            .min_by_key(|point| point.source_index)
            .map(endorsement),
    }
}

/// nest 事件的终端背书（级别对齐查法，p117 T1 = S0+S1a 合并项的生产单一来源；关③ P3 =
/// confirm_side 收紧唯一落点——kind 分域点类限定在本函数语义层，调用点零限定，禁 fork）。
///
/// 账本 = `levels[event_bsp_book_level(e.level)?].bsp`；窗口左端 = `e.interval_b.0`
///（c_start = `departure_move_c_start` 当前 episode 首腿，失败离开→回中枢→重新离开不
/// 桥接）；右端 = `e.turn_source`（R1 后 = 确认点 t*，因果序正，无前视入证）。
/// 点类限定按 `e.kind` 分派（Trend = 破 B 一类点 ∧ owner=B——B 身份由事件自带
/// `e.b_center_start` 快照供给，判定式 = `point.center.start_index == e.b_center_start`；
/// Consolidation = 同向一/二/三类全合法）。账本级别缺失/越界 ⟹ None。
///
/// S0 的 A1 桥键（c 责任单元 `terminal_bits_bridged`）被级别移位 + C-b 同构吸收，
/// 不并列实装（裁定 T1.4，单一来源纪律：旧查法删除，不得保留为 fallback）。
pub fn terminal_bits_at_event(
    c: &Classification,
    e: &NestCandidateEvent,
    m: TerminalMatch,
) -> Option<TerminalEndorsement> {
    let book = &c.levels.get(event_bsp_book_level(e.level)?)?.bsp;
    terminal_bits_in_book(
        book,
        e.interval_b.0,
        e.turn_source,
        e.side,
        e.kind,
        Some(e.b_center_start),
        m,
    )
}

/// #92 单基例 typed 装配。
///
/// Cand 由 [`NestCandidateEvent`] 的 provider 构造即真；力度在
/// `divergence_confirmed` 独立合取。Trend/Pan 共用几何递归但保留 typed sidecar。
/// `judge_at` 只登记 D3 违反率，绝不作硬门。
///
/// ## 终端契约（p117 T1，bsp-terminal-endorsement-ruling-20260718；关③ P3 收紧）
///
/// `terminal_of` 必须返回**事件结构级别**账本（`levels[exec-1].bsp`，见
/// [`event_bsp_book_level`]）的合法终端背书 bits——点类分域 = 事件 kind（Trend = 破 B
/// 一类 ∧ owner=B；Consolidation = 同向一/二/三类，pan-terminal-endorsement-ruling-
/// 20260718 P1/P2）；调用方须用 [`terminal_bits_at_event`]（收紧唯一落点，禁调用点
/// 自行加限定 fork）。`levels[exec]` 位格等式查法是级别混配装配工件，已裁定退役，
/// 不得作为生产查法恢复（裁定不回滚条款）。
pub fn assemble_typed_certificate<F>(
    events_by_level: &[Vec<NestCandidateEvent>],
    base: &NestCandidateEvent,
    top_level: usize,
    caliber: NestIntervalCaliber,
    terminal_of: &F,
) -> Option<TypedNestCertificate>
where
    F: Fn(&NestCandidateEvent) -> Option<BspBits>,
{
    let exec_level = base.level as usize;
    if top_level < exec_level || !base.divergence_confirmed {
        return None;
    }
    let terminal = terminal_of(base)?;
    if !terminal.confirm_side(base.side) {
        return None;
    }
    let base_interval = typed_interval(base, caliber);
    let mut rungs_low_to_high = Vec::with_capacity(top_level - exec_level);
    let mut kinds_low_to_high = vec![base.kind];
    let mut clocks_low_to_high = vec![base.judge_at];
    let mut ids_low_to_high = vec![NestEventIdentity::of(base)];
    // p118 关④：confirmed sidecar 以基例力度真值播种（基例门 :564 已断言 true）。
    let mut confirmed_low_to_high = vec![base.divergence_confirmed];
    if !extend_typed_upward(
        events_by_level,
        base.side,
        exec_level + 1,
        top_level,
        caliber,
        &base_interval,
        &mut rungs_low_to_high,
        &mut kinds_low_to_high,
        &mut clocks_low_to_high,
        &mut ids_low_to_high,
        &mut confirmed_low_to_high,
    ) {
        return None;
    }
    rungs_low_to_high.reverse();
    kinds_low_to_high.reverse();
    clocks_low_to_high.reverse();
    ids_low_to_high.reverse();
    confirmed_low_to_high.reverse();
    let certificate = NestCertificateBuilder {
        side: base.side,
        terminal,
        base_confirm_src: Some(base.judge_at),
        base_interval,
        rungs: rungs_low_to_high,
    }
    .finish();
    debug_assert!(certificate.n_delta());
    Some(TypedNestCertificate {
        certificate,
        kinds: kinds_low_to_high,
        judge_at: clocks_low_to_high,
        identities: ids_low_to_high,
        confirmed: confirmed_low_to_high,
        caliber,
    })
}

#[allow(clippy::too_many_arguments)]
fn extend_typed_upward(
    events_by_level: &[Vec<NestCandidateEvent>],
    side: Side,
    level: usize,
    top_level: usize,
    caliber: NestIntervalCaliber,
    child: &NestInterval,
    rungs: &mut Vec<NestRung>,
    kinds: &mut Vec<NestDivergenceKind>,
    clocks: &mut Vec<usize>,
    ids: &mut Vec<NestEventIdentity>,
    confirmed: &mut Vec<bool>,
) -> bool {
    if level > top_level {
        return true;
    }
    let Some(events) = events_by_level.get(level) else {
        return false;
    };
    let mut order: Vec<_> = (0..events.len()).collect();
    order.sort_by_key(|&index| {
        let event = &events[index];
        (typed_interval(event, caliber).sel_key(), event.turn_source, index)
    });
    for index in order {
        let event = &events[index];
        // #97 初筛只看结构（D1 裁定：Cand=纯结构宽候选，力度留在②基例生产门）：
        // rung 级不再要求 divergence_confirmed；力度真值仍在事件字段上独立可查。
        if event.side != side {
            continue;
        }
        let parent = typed_interval(event, caliber);
        if !is_sub(child, &parent) {
            continue;
        }
        rungs.push(NestRung::assembled(event.judge_at, *child, parent, true));
        kinds.push(event.kind);
        clocks.push(event.judge_at);
        ids.push(NestEventIdentity::of(event));
        // p118 关④断链点修复：rung 级 `divergence_confirmed` 真值不再丢弃——恢复进
        // sidecar 向量（施工图 §1.2：信息在装配时存在、输出时消失）；回溯时同步 pop。
        confirmed.push(event.divergence_confirmed);
        if extend_typed_upward(
            events_by_level,
            side,
            level + 1,
            top_level,
            caliber,
            &parent,
            rungs,
            kinds,
            clocks,
            ids,
            confirmed,
        ) {
            return true;
        }
        rungs.pop();
        kinds.pop();
        clocks.pop();
        ids.pop();
        confirmed.pop();
    }
    false
}

/// #92 批量 typed 装配。候选与终端确认的计数由调用方独立统计；本函数只返回完整证书。
pub fn assemble_typed_certificates<F>(
    events_by_level: &[Vec<NestCandidateEvent>],
    exec_level: usize,
    top_level: usize,
    caliber: NestIntervalCaliber,
    terminal_of: F,
) -> Vec<TypedNestCertificate>
where
    F: Fn(&NestCandidateEvent) -> Option<BspBits>,
{
    let Some(bases) = events_by_level.get(exec_level) else {
        return Vec::new();
    };
    bases.iter().filter_map(|base| {
        assemble_typed_certificate(events_by_level, base, top_level, caliber, &terminal_of)
    }).collect()
}

// ═════════ P2 证书生产装配层（strict-nesting-divergence-plan-20260708 §P2）═════════

/// `Cand^δ_ℓ` 定义式（P2 落定，补 spec 疑点2 的缺口；裁决① strict-nesting-rulings-20260708）：
///
/// > **`Cand^δ_ℓ(x) ≔ 塔上 per-level 背驰段谓词`** = [`CandDeltaEvent::cand_delta`]
/// > （`recursive_tower::level_cand_delta`：A/C 段按该级 tower 段结构定位（跨中枢趋势配对，
/// > 0016:62），gauge 复用 divergence.rs MacdArea 默认路径，严格 `curr < prev`）。
///
/// - 裁决②：盘整背驰**不入谓词**（只出 [`CandDeltaEvent::pan_div_diag`] 诊断位，不参与本装配）。
/// - `d_parent_interval` 仅保留 2026-07-10 历史基线的 episode 口径；正式装配由消费者显式
///   选择 [`d_parent_interval_snapshot`] 或 [`d_parent_interval_terminal`]。`confirm_src` 仍只
///   登记确认延迟，不作闸门或排序键。
/// - P1 已证：ℓ=e=0 时该谓词与 `extract_signals` buy1/sell1 背驰确认支**逐 bit 一致**
///   （strict_nest_check 硬门 PASS）⟹ 基例 `Cand^δ_e` 与 `Conf^δ_e` 确认支同源无分叉。
///
/// 历史兼容区间 `event.interval == event.c_episode_interval`；不得将本函数输出冒充完整 `c_p`。
pub fn d_parent_interval(ev: &CandDeltaEvent) -> NestInterval {
    debug_assert_eq!(ev.c_episode_start, ev.interval.0, "episode 左端别名必须一致");
    NestInterval {
        end_time: ev.interval.1 as u64,
        start_time: ev.enter_src as u64,
        idx: 0,
    }
}

/// #43 复议后的规范父区间：只消费已闭合的完整 `c_p` 证书；能力不足时返回 `None`，
/// 不回退到 episode、确认点或其它数值锚。
pub fn d_parent_interval_snapshot(ev: &CandDeltaEvent) -> Option<NestInterval> {
    ev.c_interval_full.map(|(start, end)| NestInterval {
        end_time: end as u64,
        start_time: start as u64,
        idx: 0,
    })
}

/// 终态父区间：必须由调用方提供对应级别对象集，并沿稳定边显式读取；不改写事件快照。
pub fn d_parent_interval_terminal(
    ev: &CandDeltaEvent,
    objects: &[CpScanOwnership],
) -> Option<NestInterval> {
    cp_terminal_certificate(ev, objects).map(|certificate| NestInterval {
        start_time: certificate.c_interval_full.0 as u64,
        end_time: certificate.c_interval_full.1 as u64,
        idx: 0,
    })
}

/// #47 前兼容名；语义固定为确认时快照。新消费者必须显式调用 snapshot/terminal 版本。
#[deprecated(note = "请显式选择 d_parent_interval_snapshot 或 d_parent_interval_terminal")]
pub fn d_parent_interval_full(ev: &CandDeltaEvent) -> Option<NestInterval> {
    d_parent_interval_snapshot(ev)
}

/// 冻结的子包含区间 `D_child := I(A_child)`。
pub fn d_child_interval(ev: &CandDeltaEvent) -> NestInterval {
    NestInterval {
        end_time: ev.a_interval.1 as u64,
        start_time: ev.a_interval.0 as u64,
        idx: 0,
    }
}

/// 单基例证书装配 `N^δ_{ℓ↓e}`：从执行级 e 事件 `base` 向上装配至目标级 ℓ=`top_level`。
///
/// ## 结果包（六要素）
/// - **结论**：`Some(cert)` ⟺ 存在满足三门合取的完整级链；产出证书**构造即有效**
///   （`debug_assert!(cert.n_delta())`）。三门为方向一致、闭包含、每级 Cand：
///   1. **相邻级 Sub 包含**：`I(A_child) ⊆ D_parent`（复用契约锚 [`is_sub`]，闭口径）；
///   2. **每级背驰段必要门**：每级 `cand_delta=true` 且方向 δ 全链一致。
///      `confirm_src` 仅登记，不参与否决或排序。
/// - **定义依据**：spec P5 §6 递归式 `Cand^δ_ℓ ∧ [J^δ_{ℓ-1}⊆J^δ_ℓ] ∧ N^δ_{ℓ-1↓e}` +
///   plan §P2（递降/Sub/必要门三分量）；基例 `Conf^δ_e` 由 `terminal`（上游分类 bit-vector）
///   提供，**不重判**置位。
/// - **边界条件**：(1) `top_level < base.level`、`base.cand_delta=false`、或
///   `!terminal.confirm_side(side)` ⟹ `None`（前置即拒）。(2) `top_level == base.level`
///   ⟹ 纯基例证书（`rungs` 空）。(3) 任一中间级无可行事件 ⟹ `None`（链不完整不出半成品）。
///   (4) 多候选：每级按 `(D_parent, I(A), 原索引)` 升序 DFS 回溯，取字典序最早**可行**链
///   （确定性；贪心最早不可行时回溯到次早，∃ 语义完备）。
/// - **下游推论**：证书校验本体仍是 [`NestCertificate::n_delta`] 自高向低递归（0027:7,11）；
///   本装配只做**生产**，搜索序不改判据。产量口径与 E1 锚C 同源（预期极稀）。
/// - **谱系引用**：P1 谓词层（recursive_tower.rs 铁律：不动 signal.rs/bsp.rs/divergence.rs
///   判据）；锚C 口径 = e1_tri_anchor.rs 相邻级配对复刻。
/// - **影响声明**：纯增量生产层；不改 `Chi`/`confirm`/`is_sub`/`NestCertificate` 消费侧。
///   sidecar 只出证书不改置位（plan 目标）。L0 操作语义（非 L2 alpha）。
#[deprecated(note = "请显式选择 assemble_certificate_snapshot 或 assemble_certificate_terminal")]
pub fn assemble_certificate(
    events_by_level: &[Vec<CandDeltaEvent>],
    base: &CandDeltaEvent,
    top_level: usize,
    terminal: BspBits,
) -> Option<NestCertificate> {
    assemble_certificate_snapshot(events_by_level, base, top_level, terminal)
}

/// 只消费事件确认时快照的严格装配。
pub fn assemble_certificate_snapshot(
    events_by_level: &[Vec<CandDeltaEvent>],
    base: &CandDeltaEvent,
    top_level: usize,
    terminal: BspBits,
) -> Option<NestCertificate> {
    assemble_certificate_with(
        events_by_level,
        base,
        top_level,
        terminal,
        &d_parent_interval_snapshot,
    )
}

/// 只消费终态对象的严格装配；每级都沿事件稳定边读取该级对象集。
pub fn assemble_certificate_terminal(
    events_by_level: &[Vec<CandDeltaEvent>],
    objects_by_level: &[&[CpScanOwnership]],
    base: &CandDeltaEvent,
    top_level: usize,
    terminal: BspBits,
) -> Option<NestCertificate> {
    assemble_certificate_with(
        events_by_level,
        base,
        top_level,
        terminal,
        &|event| {
            objects_by_level
                .get(event.level as usize)
                .and_then(|objects| d_parent_interval_terminal(event, objects))
        },
    )
}

fn assemble_certificate_with<F>(
    events_by_level: &[Vec<CandDeltaEvent>],
    base: &CandDeltaEvent,
    top_level: usize,
    terminal: BspBits,
    parent_interval_of: &F,
) -> Option<NestCertificate>
where
    F: Fn(&CandDeltaEvent) -> Option<NestInterval>,
{
    let e = base.level as usize;
    if top_level < e || !base.cand_delta || !terminal.confirm_side(base.side) {
        return None;
    }
    let base_iv = d_child_interval(base);
    let mut acc: Vec<NestRung> = Vec::with_capacity(top_level - e);
    if !extend_upward(
        events_by_level,
        base.side,
        e + 1,
        top_level,
        &base_iv,
        &mut acc,
        parent_interval_of,
    ) {
        return None;
    }
    // 收集序低→高（自基例向上搜索）；证书 rungs 约定从高(ℓ)到低(e+1)。
    acc.reverse();
    // cert F-01：经收口 builder（三门已由本装配 DFS 保证，finish 再复验 ⊆ 链）。
    let cert = NestCertificateBuilder {
        side: base.side,
        terminal,
        base_confirm_src: Some(base.confirm_src),
        base_interval: base_iv,
        rungs: acc,
    }
    .finish();
    debug_assert!(cert.n_delta(), "装配即校验：产出证书必过 n_delta（三门合取）");
    Some(cert)
}

/// 装配递归核：为级 `level_k` 找满足三门的父事件并继续向上，直至越过 `top_level`。
///
/// 每级候选按 `(D_parent, I(A), 原索引)` 升序 DFS；确认时点不参与排序或闸门。
fn extend_upward<F>(
    events_by_level: &[Vec<CandDeltaEvent>],
    side: Side,
    level_k: usize,
    top_level: usize,
    child_iv: &NestInterval,
    acc: &mut Vec<NestRung>,
    parent_interval_of: &F,
) -> bool
where
    F: Fn(&CandDeltaEvent) -> Option<NestInterval>,
{
    if level_k > top_level {
        return true;
    }
    let Some(evs) = events_by_level.get(level_k) else {
        return false;
    };
    let mut order: Vec<usize> = (0..evs.len()).collect();
    order.sort_by_key(|&i| {
        (
            parent_interval_of(&evs[i])
                .map(|iv| (iv.start_time as usize, iv.end_time as usize))
                .unwrap_or((usize::MAX, usize::MAX)),
            evs[i].a_interval,
            i,
        )
    });
    for i in order {
        let ev = &evs[i];
        // 必要门（cand_delta）+ 方向一致；confirm_src 仅登记，绝不参与否决。
        if !ev.cand_delta || ev.side != side {
            continue;
        }
        // #43：完整父证书未闭合即拒绝；不得回退 episode、确认点或数值锚。
        let Some(iv) = parent_interval_of(ev) else {
            continue;
        };
        // 相邻级 Sub 包含：I(A_child) ⊆ D_parent（复用契约锚 is_sub）。
        if !is_sub(child_iv, &iv) {
            continue;
        }
        acc.push(NestRung::assembled(ev.confirm_src, *child_iv, iv, true));
        let next_child_iv = d_child_interval(ev);
        if extend_upward(
            events_by_level,
            side,
            level_k + 1,
            top_level,
            &next_child_iv,
            acc,
            parent_interval_of,
        ) {
            return true;
        }
        acc.pop();
    }
    false
}

/// 批量装配驱动：执行级 `exec_level` 全部 `cand_delta=true` 基例逐一尝试装配至 `top_level`。
///
/// `terminal_of` 由调用方提供基例终端 bit-vector（上游分类输出查找，**不重判**）；返回
/// `None` 的基例跳过（P1 逐 bit 一致 ⟹ 生产路径不应发生，调用方可自行计数诊断）。
#[deprecated(note = "请显式选择 assemble_certificates_snapshot 或 assemble_certificates_terminal")]
pub fn assemble_certificates<F>(
    events_by_level: &[Vec<CandDeltaEvent>],
    exec_level: usize,
    top_level: usize,
    terminal_of: F,
) -> Vec<NestCertificate>
where
    F: FnMut(&CandDeltaEvent) -> Option<BspBits>,
{
    assemble_certificates_snapshot(events_by_level, exec_level, top_level, terminal_of)
}

/// 批量确认时快照装配。
pub fn assemble_certificates_snapshot<F>(
    events_by_level: &[Vec<CandDeltaEvent>],
    exec_level: usize,
    top_level: usize,
    mut terminal_of: F,
) -> Vec<NestCertificate>
where
    F: FnMut(&CandDeltaEvent) -> Option<BspBits>,
{
    let Some(bases) = events_by_level.get(exec_level) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for base in bases.iter().filter(|b| b.cand_delta) {
        let Some(terminal) = terminal_of(base) else {
            continue;
        };
        if let Some(cert) =
            assemble_certificate_snapshot(events_by_level, base, top_level, terminal)
        {
            out.push(cert);
        }
    }
    out
}

/// 批量终态对象装配。
pub fn assemble_certificates_terminal<F>(
    events_by_level: &[Vec<CandDeltaEvent>],
    objects_by_level: &[&[CpScanOwnership]],
    exec_level: usize,
    top_level: usize,
    mut terminal_of: F,
) -> Vec<NestCertificate>
where
    F: FnMut(&CandDeltaEvent) -> Option<BspBits>,
{
    let Some(bases) = events_by_level.get(exec_level) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for base in bases.iter().filter(|base| base.cand_delta) {
        let Some(terminal) = terminal_of(base) else {
            continue;
        };
        if let Some(certificate) = assemble_certificate_terminal(
            events_by_level,
            objects_by_level,
            base,
            top_level,
            terminal,
        ) {
            out.push(certificate);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::LevelState;
    use super::super::super::types::Center;
    use super::*;

    fn interval(et: u64, st: u64, idx: u64) -> NestInterval {
        NestInterval { end_time: et, start_time: st, idx }
    }

    fn rung(et: u64, st: u64, idx: u64, cand: bool) -> NestRung {
        NestRung { confirm_src: None, child_interval: None, interval: interval(et, st, idx), cand }
    }

    fn buy1_bits() -> BspBits {
        let mut t = BspBits::default();
        t.buy1 = true;
        t
    }

    fn sell1_bits() -> BspBits {
        let mut t = BspBits::default();
        t.sell1 = true;
        t
    }

    #[test]
    fn sel_order_endtime_priority_bit_exact() {
        // endTime 大优先。
        let a = interval(10, 0, 0);
        let b = interval(5, 100, 0);
        assert!(sel_order(&a, &b));
        assert!(!sel_order(&b, &a));
    }

    #[test]
    fn sel_order_starttime_tiebreak_bit_exact() {
        // endTime 相等 → startTime 大优先。
        let a = interval(10, 8, 0);
        let b = interval(10, 5, 0);
        assert!(sel_order(&a, &b));
    }

    #[test]
    fn sel_order_idx_tiebreak_bit_exact() {
        // endTime/startTime 相等 → idx 小优先。
        let a = interval(10, 8, 1);
        let b = interval(10, 8, 5);
        assert!(sel_order(&a, &b));
        assert!(!sel_order(&b, &a));
    }

    #[test]
    fn sel_order_asymmetric() {
        // selOrder_asymm：a 优于 b ⟹ b 不优于 a。
        let a = interval(10, 8, 1);
        let b = interval(5, 3, 9);
        assert!(sel_order(&a, &b));
        assert!(!sel_order(&b, &a));
    }

    #[test]
    fn select_best_picks_lexicographic_winner() {
        let cands = vec![
            interval(5, 3, 0),
            interval(10, 8, 5),
            interval(10, 8, 1), // 同 end/start，idx 最小 ⟹ 胜
            interval(10, 5, 0),
        ];
        let best = select_best(&cands).expect("非空候选有最优");
        assert_eq!(best, interval(10, 8, 1));
    }

    #[test]
    fn select_best_empty_none() {
        assert_eq!(select_best(&[]), None);
    }

    #[test]
    fn selected_key_unique_property() {
        // selected_key_unique：同候选集任意选法键唯一（sel_order 严格全序）。
        let cands = vec![interval(10, 8, 1), interval(10, 8, 5), interval(5, 3, 0)];
        let b1 = select_best(&cands).unwrap();
        // 重排候选集选出的最优键相同。
        let reordered = vec![interval(5, 3, 0), interval(10, 8, 5), interval(10, 8, 1)];
        let b2 = select_best(&reordered).unwrap();
        assert_eq!(b1.sel_key(), b2.sel_key());
    }

    /// ★#100 问题① 验收（多个包含父候选）：J_child⊂J_a ∧ J_child⊂J_b——Sel_Θ（`select_best`/
    /// `sel_order`）从多个包含父中选**唯一且 bit-exact**（重排候选集选出同键，确定性选择器可复现）。
    #[test]
    fn sel_theta_unique_among_multiple_containing_parents() {
        let child = interval(50, 40, 0);
        // 两父都包含 child（is_sub(child, a) ∧ is_sub(child, b)），三键不同。
        let j_a = interval(80, 20, 3);
        let j_b = interval(90, 10, 7);
        assert!(is_sub(&child, &j_a));
        assert!(is_sub(&child, &j_b));
        // Sel_Θ 选唯一（end_time 大优先 ⟹ j_b 90>80）。
        let best = select_best(&[j_a, j_b]).unwrap();
        assert_eq!(best, j_b);
        // bit-exact 可复现：重排候选集选出同键。
        assert_eq!(select_best(&[j_b, j_a]).unwrap().sel_key(), best.sel_key());
    }

    #[test]
    fn is_sub_nesting_bit_exact() {
        // inner [start>=, end<=] 套在 outer 内。
        let outer = interval(20, 0, 0);
        let inner = interval(15, 5, 0);
        assert!(is_sub(&inner, &outer)); // start 5>=0 ∧ end 15<=20
        assert!(!is_sub(&outer, &inner)); // 反向不套
    }

    #[test]
    fn confirm_nonempty_bit_exact() {
        // Λ≠∅ ⟹ confirm（confirm_of_two_three：2/3 共存也确认）。
        let mut t = BspBits::default();
        assert!(!confirm(&t)); // 全零不确认（not_confirm_of_empty）
        t.buy2 = true;
        t.buy3 = true;
        assert!(confirm(&t)); // 2/3 共存确认
    }

    #[test]
    fn confirm_empty_rejected() {
        // 全零 bit-vector 唯一被排除（not_confirm_of_empty）。
        assert!(!confirm(&BspBits::default()));
    }

    #[test]
    fn chi_nest_valid_and_confirmed() {
        // 区间套逐级缩小 + 终端确认 ⟹ χ confirmed。
        let chain = vec![
            LevelNode { cands: vec![interval(100, 0, 0)], chosen: interval(100, 0, 0) },
            LevelNode { cands: vec![interval(80, 10, 0)], chosen: interval(80, 10, 0) },
            LevelNode { cands: vec![interval(60, 20, 0)], chosen: interval(60, 20, 0) },
        ];
        let mut terminal = BspBits::default();
        terminal.buy1 = true;
        let chi = Chi { chain, terminal };
        assert!(chi.nest_valid()); // 100⊇80⊇60 逐级缩小
        assert!(chi.is_confirmed());
        assert_eq!(chi.locator_keys().len(), 3);
    }

    #[test]
    fn chi_broken_nesting_not_valid() {
        // 区间套不缩小（下级 end 超过上级）⟹ nest 不良构。
        let chain = vec![
            LevelNode { cands: vec![interval(60, 20, 0)], chosen: interval(60, 20, 0) },
            LevelNode { cands: vec![interval(100, 0, 0)], chosen: interval(100, 0, 0) }, // end 超界
        ];
        let mut terminal = BspBits::default();
        terminal.buy1 = true;
        let chi = Chi { chain, terminal };
        assert!(!chi.nest_valid());
        assert!(!chi.is_confirmed());
    }

    #[test]
    fn chi_singleton_chain_nest_trivial() {
        // 单级链（操作级=执行级）：无区间套缩小，nest 平凡良构（nest_certificate_unique_singleton）。
        let mut terminal = BspBits::default();
        terminal.buy3 = true;
        let chi = Chi {
            chain: vec![LevelNode { cands: vec![interval(50, 0, 0)], chosen: interval(50, 0, 0) }],
            terminal,
        };
        assert!(chi.nest_valid());
        assert!(chi.is_confirmed());
    }

    #[test]
    fn n_delta_base_case_directional_confirm() {
        // ℓ=e（rungs 空）：N^δ = Conf^δ_e（方向化基例）。
        let cert = NestCertificate {
            side: Side::Long,
            terminal: buy1_bits(),
            base_confirm_src: None,
            base_interval: interval(50, 0, 0),
            rungs: vec![],
        };
        assert!(cert.n_delta()); // 买侧确认 + Long ⟹ 1
        // 方向翻转：买侧 bit 对 Short 不确认 ⟹ 0。
        let cert_short = NestCertificate { side: Side::Short, ..cert.clone() };
        assert!(!cert_short.n_delta());
    }

    #[test]
    fn n_delta_base_case_empty_terminal_rejected() {
        // 全零终端 ⟹ 基例 Conf^δ_e=0。
        let cert = NestCertificate {
            side: Side::Long,
            terminal: BspBits::default(),
            base_confirm_src: None,
            base_interval: interval(50, 0, 0),
            rungs: vec![],
        };
        assert!(!cert.n_delta());
    }

    #[test]
    fn n_delta_two_level_full_certificate() {
        // ℓ=e+1：Cand^δ_ℓ ∧ [J^δ_e ⊆ J^δ_ℓ] ∧ Conf^δ_e。
        let cert = NestCertificate {
            side: Side::Long,
            terminal: buy1_bits(),
            base_confirm_src: None,
            base_interval: interval(60, 20, 0), // J^δ_e 内层
            rungs: vec![rung(100, 0, 0, true)], // J^δ_ℓ 外层，Cand=1
        };
        assert!(is_sub(&interval(60, 20, 0), &interval(100, 0, 0))); // 子⊆父
        assert!(cert.n_delta());
    }

    #[test]
    fn n_delta_false_when_cand_false() {
        // 候选谓词 Cand^δ_ℓ=0 ⟹ 整体翻转（即便区间套 + 确认都成立）。
        let cert = NestCertificate {
            side: Side::Long,
            terminal: buy1_bits(),
            base_confirm_src: None,
            base_interval: interval(60, 20, 0),
            rungs: vec![rung(100, 0, 0, false)],
        };
        assert!(!cert.n_delta());
    }

    #[test]
    fn n_delta_false_when_nesting_broken() {
        // 子⊄父（base end 100 超出 rung end 60）⟹ ⊆ 失败 ⟹ 0。
        let cert = NestCertificate {
            side: Side::Long,
            terminal: buy1_bits(),
            base_confirm_src: None,
            base_interval: interval(100, 0, 0), // 子超界
            rungs: vec![rung(60, 20, 0, true)],
        };
        assert!(!cert.n_delta());
    }

    #[test]
    fn n_delta_subset_direction_is_child_in_parent() {
        // ⊆ 方向校验：父⊆子（反向）不成立——大区间作子级、小区间作父级 ⟹ child⊄parent。
        let cert = NestCertificate {
            side: Side::Long,
            terminal: buy1_bits(),
            base_confirm_src: None,
            base_interval: interval(100, 0, 0), // 大区间作子级
            rungs: vec![rung(60, 20, 0, true)], // 小区间作父级
        };
        assert!(!cert.n_delta());
    }

    #[test]
    fn n_delta_three_level_chain() {
        // ℓ=e+2：100 ⊇ 80 ⊇ 60，两递归级 Cand=1，买侧确认 ⟹ 1。
        let cert = NestCertificate {
            side: Side::Long,
            terminal: buy1_bits(),
            base_confirm_src: None,
            base_interval: interval(60, 20, 0),                          // J^δ_e
            rungs: vec![rung(100, 0, 0, true), rung(80, 10, 0, true)],   // ℓ, ℓ-1
        };
        assert!(cert.n_delta());
        // 中间级 Cand=0 ⟹ 逐级合取翻转为 0。
        let cert_mid_false = NestCertificate {
            rungs: vec![rung(100, 0, 0, true), rung(80, 10, 0, false)],
            ..cert.clone()
        };
        assert!(!cert_mid_false.n_delta());
    }

    #[test]
    fn n_delta_short_direction_certificate() {
        // δ=Short：基例取 conf_minus（卖侧）。
        let cert = NestCertificate {
            side: Side::Short,
            terminal: sell1_bits(),
            base_confirm_src: None,
            base_interval: interval(60, 20, 0),
            rungs: vec![rung(100, 0, 0, true)],
        };
        assert!(cert.n_delta());
        // 同结构换 Long：卖侧 bit 对 Long 不确认 ⟹ 0。
        let cert_long = NestCertificate { side: Side::Long, ..cert.clone() };
        assert!(!cert_long.n_delta());
    }

    // ───────── P2 装配层（assemble_certificate / assemble_certificates）─────────

    fn cev(level: u32, side: Side, src: usize, lo: usize, hi: usize, cand: bool) -> CandDeltaEvent {
        cev_with_a(level, side, src, (lo, hi), (lo, hi), cand)
    }

    fn cev_with_a(
        level: u32,
        side: Side,
        src: usize,
        d_parent: (usize, usize),
        a_interval: (usize, usize),
        cand: bool,
    ) -> CandDeltaEvent {
        CandDeltaEvent {
            level,
            side,
            divergence_confirm_src: src,
            confirm_src: src,
            interval: d_parent,
            a_interval,
            c_episode_start: d_parent.0,
            c_episode_interval: d_parent,
            c_interval_full: Some(d_parent),
            b_parent: None,
            c_structure: None,
            third_class_in_c: None,
            cp_certificate_confirm_src: None,
            full_trend_c_qualified: None,
            full_trend_evidence: None,
            cp_ownership: None,
            enter_src: d_parent.0,
            cand_delta: cand,
            pan_div_diag: false,
        }
    }

    #[test]
    fn full_d_parent_accessor_never_falls_back_to_episode() {
        let mut ev = cev(1, Side::Long, 80, 40, 80, true);
        ev.c_interval_full = None;
        assert!(
            d_parent_interval_snapshot(&ev).is_none(),
            "无完整证书时不得回退 episode"
        );
        ev.c_interval_full = Some((20, 80));
        let full = d_parent_interval_snapshot(&ev).expect("完整 c_p 区间可消费");
        assert_eq!((full.start_time, full.end_time), (20, 80));
        assert_eq!(
            d_parent_interval(&ev).start_time,
            40,
            "历史 episode 口径保持隔离"
        );
    }

    #[test]
    fn assembly_rejects_unclosed_parent_certificate_without_episode_fallback() {
        let base = cev(0, Side::Long, 60, 20, 60, true);
        let mut parent = cev(1, Side::Long, 80, 10, 80, true);
        parent.c_interval_full = None;
        let evs = vec![vec![base.clone()], vec![parent]];
        assert!(
            assemble_certificate_snapshot(&evs, &base, 1, buy1_bits()).is_none(),
            "c_interval_full=None 必须拒绝，不能以 episode [10,80] 回填"
        );
    }

    #[test]
    fn assemble_two_level_chain() {
        // 基例 e=0（[20,60]，src=60）+ 父级 ℓ=1（[10,80]⊇[20,60]，src=50≤60）⟹ 证书成立。
        let base = cev(0, Side::Long, 60, 20, 60, true);
        let evs = vec![
            vec![base.clone()],
            vec![cev(1, Side::Long, 50, 10, 80, true)],
        ];
        let cert =
            assemble_certificate_snapshot(&evs, &base, 1, buy1_bits()).expect("完整链应出证书");
        assert_eq!(cert.rungs.len(), 1);
        assert_eq!(cert.rungs[0].interval, interval(80, 10, 0));
        assert_eq!(cert.base_interval, interval(60, 20, 0));
        assert!(cert.n_delta(), "装配即校验");
    }

    #[test]
    fn assemble_preserves_confirm_src_independent_from_interval_end() {
        // #38：确认时点与结构定位窗右端是两条独立坐标轴；装配不得用 interval.end 回填确认时点。
        let base = cev(0, Side::Long, 90, 40, 80, true);
        let evs = vec![
            vec![base.clone()],
            vec![cev(1, Side::Long, 70, 20, 100, true)],
        ];

        let cert = assemble_certificate_snapshot(&evs, &base, 1, buy1_bits())
            .expect("拆绑后完整链应出证书");

        assert_eq!(cert.base_confirm_src, Some(90));
        assert_ne!(cert.base_confirm_src.unwrap() as u64, cert.base_interval.end_time);
        assert_eq!(cert.rungs[0].confirm_src, Some(70));
        assert_ne!(cert.rungs[0].confirm_src.unwrap() as u64, cert.rungs[0].interval.end_time);
        assert!(cert.n_delta());
    }

    #[test]
    fn dparent_accepts_child_a_inside_and_ignores_confirm_order() {
        let base = cev_with_a(0, Side::Long, 120, (5, 100), (30, 50), true);
        let parent = cev_with_a(1, Side::Long, 150, (20, 80), (10, 70), true);
        let evs = vec![vec![base.clone()], vec![parent]];
        let cert = assemble_certificate_snapshot(&evs, &base, 1, buy1_bits())
            .expect("I(A_child)⊆D_parent 时应装配；confirm_src 顺序仅诊断");
        assert!(cert.n_delta());
        assert_eq!(cert.base_interval(), interval(50, 30, 0));
        assert_eq!(cert.rungs()[0].interval(), interval(80, 20, 0));
    }

    #[test]
    fn dparent_rejects_child_a_outside_even_if_child_c_inside() {
        let base = cev_with_a(0, Side::Long, 60, (30, 50), (10, 50), true);
        let parent = cev_with_a(1, Side::Long, 50, (20, 80), (0, 70), true);
        let evs = vec![vec![base.clone()], vec![parent]];
        assert!(assemble_certificate_snapshot(&evs, &base, 1, buy1_bits()).is_none());
    }

    #[test]
    fn dparent_three_level_checks_each_child_a_edge() {
        let base = cev_with_a(0, Side::Short, 90, (0, 120), (30, 40), true);
        let l1 = cev_with_a(1, Side::Short, 100, (20, 80), (200, 210), true);
        let l2 = cev_with_a(2, Side::Short, 110, (190, 220), (0, 0), true);
        let evs = vec![vec![base.clone()], vec![l1], vec![l2]];
        let cert = assemble_certificate_snapshot(&evs, &base, 2, sell1_bits())
            .expect("每条边都应使用该子事件自己的 I(A)");
        assert!(cert.n_delta());
        assert_eq!(cert.rungs().len(), 2);
    }

    #[test]
    fn assemble_pure_base_when_top_eq_exec() {
        // ℓ=e ⟹ 纯基例（rungs 空），只判 Conf^δ_e 与 Cand^δ_e。
        let base = cev(0, Side::Long, 60, 20, 60, true);
        let evs = vec![vec![base.clone()]];
        let cert = assemble_certificate_snapshot(&evs, &base, 0, buy1_bits()).expect("纯基例");
        assert!(cert.rungs.is_empty());
        assert!(cert.n_delta());
    }

    #[test]
    fn assemble_rejects_parent_cand_false() {
        // 0027:15 必要门：父级 cand_delta=false ⟹ 不出证书。
        let base = cev(0, Side::Long, 60, 20, 60, true);
        let evs = vec![
            vec![base.clone()],
            vec![cev(1, Side::Long, 50, 10, 80, false)],
        ];
        assert!(assemble_certificate_snapshot(&evs, &base, 1, buy1_bits()).is_none());
    }

    #[test]
    fn assemble_rejects_sub_violation() {
        // 父区间 [30,80] 不含子 [20,60]（左端越界）⟹ Sub 门拒。
        let base = cev(0, Side::Long, 60, 20, 60, true);
        let evs = vec![
            vec![base.clone()],
            vec![cev(1, Side::Long, 50, 30, 80, true)],
        ];
        assert!(assemble_certificate_snapshot(&evs, &base, 1, buy1_bits()).is_none());
    }

    #[test]
    fn assemble_ignores_first_seen_descent_for_acceptance() {
        // 裁决：父确认 src=70 > 子确认 src=60 只登记，不得否决。
        let base = cev(0, Side::Long, 60, 20, 60, true);
        let evs = vec![
            vec![base.clone()],
            vec![cev(1, Side::Long, 70, 10, 80, true)],
        ];
        assert!(assemble_certificate_snapshot(&evs, &base, 1, buy1_bits()).is_some());
    }

    #[test]
    fn assemble_rejects_side_mismatch() {
        let base = cev(0, Side::Long, 60, 20, 60, true);
        let evs = vec![
            vec![base.clone()],
            vec![cev(1, Side::Short, 50, 10, 80, true)],
        ];
        assert!(assemble_certificate_snapshot(&evs, &base, 1, buy1_bits()).is_none());
    }

    #[test]
    fn assemble_rejects_missing_level() {
        // ℓ=2 但级 2 无事件 ⟹ 链不完整不出半成品。
        let base = cev(0, Side::Long, 60, 20, 60, true);
        let evs = vec![
            vec![base.clone()],
            vec![cev(1, Side::Long, 50, 10, 80, true)],
        ];
        assert!(assemble_certificate_snapshot(&evs, &base, 2, buy1_bits()).is_none());
    }

    #[test]
    fn assemble_rejects_unconfirmed_terminal() {
        // 基例 Conf^δ_e=false（全零 bits）⟹ 前置即拒。
        let base = cev(0, Side::Long, 60, 20, 60, true);
        let evs = vec![vec![base.clone()]];
        assert!(
            assemble_certificate_snapshot(&evs, &base, 0, BspBits::default()).is_none()
        );
    }

    #[test]
    fn assemble_backtracks_to_feasible_parent() {
        // 最早父（src=40）不含子区间 ⟹ 回溯到次早（src=50）可行父。
        let base = cev(0, Side::Long, 60, 20, 60, true);
        let evs = vec![
            vec![base.clone()],
            vec![
                cev(1, Side::Long, 40, 30, 80, true), // 最早但 Sub 违背
                cev(1, Side::Long, 50, 10, 80, true), // 次早可行
            ],
        ];
        let cert = assemble_certificate_snapshot(&evs, &base, 1, buy1_bits())
            .expect("回溯应找到可行父");
        assert_eq!(cert.rungs[0].interval, interval(80, 10, 0));
    }

    #[test]
    fn assemble_three_level_chain_descent() {
        // 三级链 ℓ=2：区间逐级放大、首见逐级不增（递降）；rungs 从高到低。
        let base = cev(0, Side::Short, 60, 20, 60, true);
        let evs = vec![
            vec![base.clone()],
            vec![cev(1, Side::Short, 55, 10, 80, true)],
            vec![cev(2, Side::Short, 50, 0, 100, true)],
        ];
        let cert = assemble_certificate_snapshot(&evs, &base, 2, sell1_bits()).expect("三级链");
        assert_eq!(cert.rungs.len(), 2);
        assert_eq!(cert.rungs[0].interval, interval(100, 0, 0)); // 级 2（高）
        assert_eq!(cert.rungs[1].interval, interval(80, 10, 0)); // 级 1（低）
        assert!(cert.n_delta());
    }

    #[test]
    fn assemble_certificates_driver_filters_bases() {
        // 驱动：cand_delta=false 基例与 terminal_of=None 基例均跳过。
        let b1 = cev(0, Side::Long, 60, 20, 60, true);
        let b2 = cev(0, Side::Long, 61, 21, 61, false); // 非 Cand 基例
        let b3 = cev(0, Side::Long, 62, 22, 62, true); // terminal 查无
        let evs = vec![
            vec![b1.clone(), b2, b3],
            vec![cev(1, Side::Long, 50, 10, 80, true)],
        ];
        let certs = assemble_certificates_snapshot(&evs, 0, 1, |b| {
            if b.confirm_src == 60 {
                Some(buy1_bits())
            } else {
                None
            }
        });
        assert_eq!(certs.len(), 1);
        assert_eq!(certs[0].base_interval, interval(60, 20, 0));
    }

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
            // 关③ P3 新字段：非终端查询的测试不读本值；终端测试用 `..ev` 结构更新显式覆写。
            b_center_start: 0,
        }
    }

    #[test]
    fn p92_typed_b_uses_c_interval_and_d3_is_sidecar_only() {
        let base = typed_event(
            1, Side::Long, NestDivergenceKind::Trend,
            (30, 50), (10, 70), 50, 100, true,
        );
        let parent = typed_event(
            2, Side::Long, NestDivergenceKind::Consolidation,
            (20, 80), (0, 100), 80, 200, true,
        );
        let events = vec![Vec::new(), vec![base.clone()], vec![parent]];
        let certificate = assemble_typed_certificate(
            &events, &base, 2, NestIntervalCaliber::B, &|_| Some(buy1_bits()),
        ).expect("B: child C [30,50] ⊆ parent C [20,80]");
        assert_eq!(certificate.certificate().base_interval(), interval(50, 30, 50));
        assert_eq!(
            certificate.kinds(),
            &[NestDivergenceKind::Consolidation, NestDivergenceKind::Trend]
        );
        assert_eq!(
            certificate.d3_descent_stats(), (1, 1),
            "逆序只计 sidecar，不否决证书"
        );
        assert!(certificate.certificate().n_delta());
    }

    #[test]
    fn p97_rung_screening_is_structural_and_identities_align() {
        let base = typed_event(
            1, Side::Long, NestDivergenceKind::Consolidation,
            (30, 50), (10, 70), 50, 100, true,
        );
        // 父级 rung 力度未确认（divergence_confirmed=false）：D1 裁定下只看结构，不得否决链。
        let parent = typed_event(
            2, Side::Long, NestDivergenceKind::Consolidation,
            (20, 80), (0, 100), 80, 200, false,
        );
        let events = vec![Vec::new(), vec![base.clone()], vec![parent.clone()]];
        let certificate = assemble_typed_certificate(
            &events, &base, 2, NestIntervalCaliber::B, &|_| Some(buy1_bits()),
        ).expect("#97: rung 初筛只看结构，父级力度未确认不否决链");
        assert_eq!(
            certificate.identities(),
            &[NestEventIdentity::of(&parent), NestEventIdentity::of(&base)],
            "#97: 身份标签按高→低对齐，含基例"
        );
        assert_eq!(certificate.kinds().len(), certificate.identities().len());
        assert_eq!(certificate.judge_at().len(), certificate.identities().len());
        assert!(certificate.certificate().n_delta());
    }

    #[test]
    fn p92_typed_weak_stage_and_terminal_bits_remain_independent() {
        let pan = typed_event(
            1, Side::Long, NestDivergenceKind::Consolidation,
            (30, 50), (10, 70), 50, 100, false,
        );
        let events = vec![Vec::new(), vec![pan.clone()]];
        assert!(
            assemble_typed_certificate(
                &events, &pan, 1, NestIntervalCaliber::B, &|_| Some(buy1_bits()),
            ).is_none(),
            "力度未确认不产背驰段证书，但不改变上游 Cand 身份"
        );

        let confirmed = NestCandidateEvent { divergence_confirmed: true, ..pan };
        let events = vec![Vec::new(), vec![confirmed.clone()]];
        assert!(
            assemble_typed_certificate(
                &events, &confirmed, 1, NestIntervalCaliber::B, &|_| Some(BspBits::default()),
            ).is_none(),
            "盘背不得强置同级 B1；终端仍须真实 Λ 非空"
        );
        let mut buy2 = BspBits::default();
        buy2.buy2 = true;
        assert!(
            assemble_typed_certificate(
                &events, &confirmed, 1, NestIntervalCaliber::B, &|_| Some(buy2),
            ).is_some(),
            "真实 B2 可满足既有 confirm_side 析取"
        );
    }

    // ───────── p117 T1 终端背书级别对齐（S1a 图 §5 四新增 + S0 八夹具 C-b 改写吸收）─────────

    fn buy2_bits() -> BspBits {
        let mut t = BspBits::default();
        t.buy2 = true;
        t
    }

    fn pt(source_index: usize, bits: BspBits) -> BspPoint {
        BspPoint {
            source_index,
            bits,
            pivot_low: 0,
            pivot_high: 0,
            center: None,
            struct_break_dir: None,
            force: None,
        }
    }

    /// 关③ P3：B 中枢身份快照基准（测试夹具共用 start_index；zd/zg/dd/gg/end 任意——
    /// 判定只比 start_index，延伸判同由 `c_window_trend_owner_extension_start_index_judges` 专测）。
    const B_START: usize = 20;

    /// 携 owner 中枢的账本点夹具（owner 判定式 = `center.start_index == b_center_start`，
    /// start_index 判同，禁全字段等式——zd/zg/dd/gg/end_index 由参数另给或置 0，均不入判）。
    fn ptc(source_index: usize, bits: BspBits, center_start: usize) -> BspPoint {
        BspPoint {
            center: Some(Center { zd: 0, zg: 0, dd: 0, gg: 0, start_index: center_start, end_index: 0 }),
            ..pt(source_index, bits)
        }
    }

    /// 判定结构 → bits 投影（多数断言只关心 bit 向量；owner 维度由关③新增测试专断）。
    fn bits_of(e: Option<TerminalEndorsement>) -> Option<BspBits> {
        e.map(|t| t.bits)
    }

    /// 关③ Trend 域事件夹具：kind=Trend 且自带 B 身份快照 `B_START`。
    fn trend_ev(interval_b: (usize, usize), turn_source: usize) -> NestCandidateEvent {
        NestCandidateEvent {
            b_center_start: B_START,
            ..typed_event(
                1, Side::Long, NestDivergenceKind::Trend,
                interval_b, (interval_b.0.saturating_sub(10), turn_source + 10),
                turn_source, turn_source, true,
            )
        }
    }

    /// 关③ Pan 域事件夹具：kind=Consolidation（owner 快照不入判，置 0）。
    fn pan_ev(interval_b: (usize, usize), turn_source: usize) -> NestCandidateEvent {
        typed_event(
            1, Side::Long, NestDivergenceKind::Consolidation,
            interval_b, (interval_b.0.saturating_sub(10), turn_source + 10),
            turn_source, turn_source, true,
        )
    }

    /// 两级分类夹具：levels[0]/levels[1] 各持一本 BSP 账，其余字段默认（全 pub，无 parser 依赖）。
    fn book2(l0: Vec<BspPoint>, l1: Vec<BspPoint>) -> Classification {
        Classification {
            levels: vec![
                LevelState { bsp: std::rc::Rc::new(l0), ..Default::default() },
                LevelState { bsp: std::rc::Rc::new(l1), ..Default::default() },
            ],
        }
    }

    #[test]
    fn event_bsp_book_level_shifts_exactly_one_down() {
        assert_eq!(event_bsp_book_level(1), Some(0));
        assert_eq!(event_bsp_book_level(2), Some(1));
        assert_eq!(event_bsp_book_level(0), None);
        assert_eq!(event_bsp_book_level(6), Some(5));
    }

    #[test]
    fn terminal_bits_at_event_exact_reads_child_book() {
        // levels[0] 有 confirm 点@100 ⟹ 命中（C-a 平移语义；Exact = 诊断常数，关③ 不收紧）。
        let c = book2(vec![pt(100, buy1_bits())], vec![]);
        let ev = typed_event(
            1, Side::Long, NestDivergenceKind::Trend,
            (90, 100), (80, 110), 100, 100, true,
        );
        assert_eq!(bits_of(terminal_bits_at_event(&c, &ev, TerminalMatch::Exact)), Some(buy1_bits()));
        // 仅 levels[1] 有 ⟹ None（新语义防回归：移位后不再读 levels[ℓ]）。
        let c_l1_only = book2(vec![], vec![pt(100, buy1_bits())]);
        assert_eq!(terminal_bits_at_event(&c_l1_only, &ev, TerminalMatch::Exact), None);
        // side 反 ⟹ None。
        let ev_short = NestCandidateEvent { side: Side::Short, ..ev };
        assert_eq!(terminal_bits_at_event(&c, &ev_short, TerminalMatch::Exact), None);
        // event.level=0 ⟹ None（nest 不听 L0）。
        let ev_l0 = NestCandidateEvent { level: 0, ..ev };
        assert_eq!(terminal_bits_at_event(&c, &ev_l0, TerminalMatch::Exact), None);
        // level 越界 ⟹ None。
        let ev_oob = NestCandidateEvent { level: 5, ..ev };
        assert_eq!(terminal_bits_at_event(&c, &ev_oob, TerminalMatch::Exact), None);
    }

    #[test]
    fn terminal_bits_at_event_c_window_picks_earliest_confirm() {
        let c = book2(
            vec![
                pt(99, buy1_bits()),            // 窗口左端外 ⟹ 排除
                ptc(120, buy1_bits(), B_START), // 窗口内最早合法点（一类 ∧ owner=B）⟹ 命中
                pt(130, sell1_bits()),          // 反向 ⟹ 忽略（confirm_side 方向隔离）
                ptc(150, buy2_bits(), B_START), // 同向二类 ⟹ Trend 域非法（关③ P1 点类限定）
                pt(201, buy1_bits()),           // 窗口右端外 ⟹ 排除
            ],
            vec![],
        );
        let ev = trend_ev((100, 200), 200);
        assert_eq!(
            bits_of(terminal_bits_at_event(&c, &ev, TerminalMatch::CWindow)),
            Some(buy1_bits()),
            "窗口 [100,200] 内最早合法点 @120（一类 ∧ owner=B）"
        );
        // 空窗（窗口内无任何点）⟹ None。
        let c_empty = book2(vec![pt(99, buy1_bits()), pt(201, buy1_bits())], vec![]);
        assert_eq!(terminal_bits_at_event(&c_empty, &ev, TerminalMatch::CWindow), None);
    }

    #[test]
    fn assemble_typed_certificate_terminal_uses_shifted_book() {
        let base = NestCandidateEvent {
            b_center_start: B_START,
            ..typed_event(
                1, Side::Long, NestDivergenceKind::Trend,
                (30, 50), (10, 70), 50, 100, true,
            )
        };
        let events = vec![Vec::new(), vec![base]];
        // 基例 level=1 confirmed + levels[0] 窗内合法背书点（一类 ∧ owner=B）@40 ⟹ Some(cert)。
        let c = book2(vec![ptc(40, buy1_bits(), B_START)], vec![]);
        let cert = assemble_typed_certificate(
            &events, &base, 1, NestIntervalCaliber::B,
            &|e| bits_of(terminal_bits_at_event(&c, e, TerminalMatch::CWindow)),
        )
        .expect("levels[0] 窗内背书点 ⟹ 证书成立");
        assert_eq!(cert.certificate().terminal(), &buy1_bits());
        // 删该点 ⟹ None。
        let c_none = book2(vec![], vec![]);
        assert!(assemble_typed_certificate(
            &events, &base, 1, NestIntervalCaliber::B,
            &|e| bits_of(terminal_bits_at_event(&c_none, e, TerminalMatch::CWindow)),
        )
        .is_none());
        // 点只在 levels[1]（levels[0] 空）⟹ None——移位后不再读 levels[1]，防旧语义回潮。
        let c_l1_only = book2(vec![], vec![ptc(40, buy1_bits(), B_START)]);
        assert!(assemble_typed_certificate(
            &events, &base, 1, NestIntervalCaliber::B,
            &|e| bits_of(terminal_bits_at_event(&c_l1_only, e, TerminalMatch::CWindow)),
        )
        .is_none());
    }

    // ───── S0 图 §5.1 八夹具按 C-b 口径改写吸收 ─────
    // 几何关系镜像 113 案代表样本 turn=726571：c_start=726527；B 延伸窗终点域 726540；
    // c 内次级别中枢终点域 726600/726680。级别移位后「级别-k 单元窗」不存在——同一几何
    // 关系落在 levels[ℓ-1] 账本的 source_index 点上，窗口 = [interval_b.0, turn_source]。

    fn s0_event(t_star: usize) -> NestCandidateEvent {
        // 关③ P3：Trend 域事件自带 B 身份快照 B_START；合法背书点须 owner=B（ptc 夹具）。
        NestCandidateEvent {
            b_center_start: B_START,
            ..typed_event(
                1, Side::Long, NestDivergenceKind::Trend,
                (726527, t_star), (726400, t_star), t_star, t_star, true,
            )
        }
    }

    /// 原 `terminal_bridge_exact_hit_degenerates`：t*=726600 恰有合法背书点（一类 ∧
    /// owner=B）时，C-b 与 C-a 逐位一致（向后兼容锁定；Exact = 诊断常数不收紧）。
    #[test]
    fn c_window_hit_at_tstar_degenerates_to_exact() {
        let c = book2(vec![ptc(726600, buy1_bits(), B_START)], vec![]);
        let ev = s0_event(726600);
        assert_eq!(bits_of(terminal_bits_at_event(&c, &ev, TerminalMatch::CWindow)), Some(buy1_bits()));
        assert_eq!(bits_of(terminal_bits_at_event(&c, &ev, TerminalMatch::Exact)), Some(buy1_bits()));
    }

    /// 原 `terminal_bridge_skips_center_being_left_unit`：B 自身结构代表点 @726520
    ///（< c_start=726527，其一类点见证进入 B 的 b-vs-a 前序背驰）被窗口左端结构性排除
    ///（F1 跨结构误记防护在 C-b 下由 c_start 左端承担）⟹ None。
    #[test]
    fn c_window_left_edge_excludes_prior_structure_point() {
        let c = book2(vec![pt(726520, buy1_bits())], vec![]);
        let ev = s0_event(726560);
        assert_eq!(terminal_bits_at_event(&c, &ev, TerminalMatch::CWindow), None);
    }

    /// 原 `terminal_bridge_selects_first_c_unit_covering_tstar`：t*=726650 深在 c 内，
    /// @726600 buy1（owner=B）命中、@726680 buy2 越 t* 排除（且二类在 Trend 域非法）——
    /// 窗口右端 = t*；多中枢 c（037:22）内的合法确认被覆盖。
    #[test]
    fn c_window_tstar_deep_in_c_picks_earliest_within_window() {
        let c = book2(vec![ptc(726600, buy1_bits(), B_START), pt(726680, buy2_bits())], vec![]);
        let ev = s0_event(726650);
        assert_eq!(bits_of(terminal_bits_at_event(&c, &ev, TerminalMatch::CWindow)), Some(buy1_bits()));
    }

    /// 原 `terminal_bridge_witness_not_formed_returns_none`：窗口 [726527,726560] 内
    /// 无任何点（@726520/@726600 皆窗外）⟹ None——037:18 否则条款域的诚实判负。
    #[test]
    fn c_window_witness_absent_returns_none() {
        let c = book2(vec![pt(726520, buy1_bits()), pt(726600, buy1_bits())], vec![]);
        let ev = s0_event(726560);
        assert_eq!(terminal_bits_at_event(&c, &ev, TerminalMatch::CWindow), None);
    }

    /// 原 `terminal_bridge_wrong_side_rejected`：窗口内仅 sell1@726600 ⟹ Long 事件
    /// None——confirm_side 方向过滤保持。
    #[test]
    fn c_window_wrong_side_rejected() {
        let c = book2(vec![pt(726600, sell1_bits())], vec![]);
        let ev = s0_event(726600);
        assert_eq!(terminal_bits_at_event(&c, &ev, TerminalMatch::CWindow), None);
    }

    /// 原 `terminal_bridge_first_confirm_at_shared_endpoint`：同坐标三点（零 bit + buy1
    /// + buy2）⟹ Some(buy1)——关③ Trend 域下零 bit 拒、buy2 二类点类拒，buy1（owner=B）
    /// 为唯一合法点（合法性过滤先于最早性/迭代序）。
    #[test]
    fn c_window_first_confirm_at_shared_coordinate() {
        let c = book2(
            vec![
                pt(726600, BspBits::default()),
                ptc(726600, buy1_bits(), B_START),
                ptc(726600, buy2_bits(), B_START),
            ],
            vec![],
        );
        let ev = s0_event(726600);
        assert_eq!(bits_of(terminal_bits_at_event(&c, &ev, TerminalMatch::CWindow)), Some(buy1_bits()));
    }

    /// 原 `terminal_bridge_pan_event_same_contract`：kind=Consolidation 事件共用同一
    /// 查法（其 interval_b.0 = pan C 起点）⟹ Some——关③ P2：Pan 域谓词 = confirm_side
    /// 精确语义（同向一/二/三类全合法），owner 不入判（center=None 亦合法）。
    #[test]
    fn c_window_pan_event_same_contract() {
        let c = book2(vec![pt(726600, buy1_bits())], vec![]);
        let ev = typed_event(
            1, Side::Long, NestDivergenceKind::Consolidation,
            (726541, 726600), (726400, 726600), 726600, 726600, true,
        );
        assert_eq!(bits_of(terminal_bits_at_event(&c, &ev, TerminalMatch::CWindow)), Some(buy1_bits()));
    }

    /// 原 `terminal_bridge_past_last_unit_returns_none` 的 C-b 消解：t*=726999 越过
    /// 账本全部点，窗口内最早合法点 @726540 仍背书 ⟹ Some——A1「越末单元 ⟹ None」边界与
    /// F2「背书晚于确认」语义差在 C-b 下结构性消解（最早点 ≤ t*，因果序正；裁定 T1.4）。
    /// 关③ Trend 域：@726600 二类非法，@726540 一类 owner=B 合法。
    #[test]
    fn c_window_tstar_past_all_points_still_endorses_earliest() {
        let c = book2(
            vec![ptc(726540, buy1_bits(), B_START), pt(726600, buy2_bits())],
            vec![],
        );
        let ev = s0_event(726999);
        assert_eq!(bits_of(terminal_bits_at_event(&c, &ev, TerminalMatch::CWindow)), Some(buy1_bits()));
    }

    // ───────── 关③ P3 confirm_side 收紧：点类分域 + owner=B start_index 判同 ─────────

    /// 关③ P1/验收线：Trend 域 一类 ∧ owner=B 命中；判定结构携带 owner start_index。
    #[test]
    fn c_window_trend_type1_owner_b_hits() {
        let c = book2(vec![ptc(150, buy1_bits(), B_START)], vec![]);
        let ev = trend_ev((100, 200), 200);
        let endo = terminal_bits_at_event(&c, &ev, TerminalMatch::CWindow)
            .expect("一类 ∧ owner=B ⟹ 合法背书");
        assert_eq!(endo.bits, buy1_bits());
        assert_eq!(endo.owner_center_start, Some(B_START), "owner 判定式 = start_index 判同");
    }

    /// 关③ P1.3：Trend 域 一类 owner≠B 拒（窗内他中枢一类点——prev/OTHER/新生中枢——
    /// 不作本事件背书）；owner 载体缺失（center=None——★owner 载体补齐（补记② 路径 (a)）
    /// 后已非生产形状，此处合成构造锁「载体缺失 ⟹ 拒」的防御语义）同样拒——
    /// 身份合取不可证，诚实判负（非放宽）。
    #[test]
    fn c_window_trend_type1_owner_not_b_rejected() {
        let ev = trend_ev((100, 200), 200);
        let c_other = book2(vec![ptc(150, buy1_bits(), B_START + 9)], vec![]);
        assert_eq!(
            terminal_bits_at_event(&c_other, &ev, TerminalMatch::CWindow),
            None,
            "一类但 owner≠B ⟹ 非法背书"
        );
        let c_none = book2(vec![pt(150, buy1_bits())], vec![]);
        assert_eq!(
            terminal_bits_at_event(&c_none, &ev, TerminalMatch::CWindow),
            None,
            "center=None ⟹ owner 不可证 ⟹ 拒"
        );
    }

    /// 关③ P1：Trend 域 同向二/三类点拒（即使 owner=B）——二/三类不视为充分背书
    ///（037:20 必要合取、061:28 候选域、027:66 构成性定义、024:18 背驰-买卖点定理）。
    #[test]
    fn c_window_trend_type2_type3_rejected() {
        let mut buy3 = BspBits::default();
        buy3.buy3 = true;
        let c = book2(
            vec![ptc(120, buy2_bits(), B_START), ptc(130, buy3, B_START)],
            vec![],
        );
        let ev = trend_ev((100, 200), 200);
        assert_eq!(
            terminal_bits_at_event(&c, &ev, TerminalMatch::CWindow),
            None,
            "同向二/三类 ⟹ Trend 域非法；窗内无一类 ⟹ 诚实判负"
        );
    }

    /// 关③ P3.1/P4 上行通道翻回：最早点为二类、窗内更晚处有一类 owner=B ⟹ 翻回合法——
    /// 合法性过滤先于最早性取点（「最早合法」≠「最早」；二类在场不否决其后一类，
    /// 044:30 延迟常态）。
    #[test]
    fn c_window_trend_later_type1_rescues_earlier_illegal() {
        let c = book2(
            vec![ptc(120, buy2_bits(), B_START), ptc(150, buy1_bits(), B_START)],
            vec![],
        );
        let ev = trend_ev((100, 200), 200);
        let endo = terminal_bits_at_event(&c, &ev, TerminalMatch::CWindow)
            .expect("后至一类 owner=B 救回");
        assert_eq!(endo.bits, buy1_bits(), "取 @150 一类而非 @120 二类（旧语义会取 buy2）");
        assert_eq!(endo.owner_center_start, Some(B_START));
    }

    /// 关③ P3.2（p117 §7/§9-② 升格生产语义）：owner 延伸后 start_index 判同——延伸
    /// 只改写 end/dd/gg，start_index 与 zd/zg 稳定（37/37）；同一 start_index、其余
    /// 字段全异仍判同（禁全字段 `Center` 等式——已证结构性必败）。
    #[test]
    fn c_window_trend_owner_extension_start_index_judges() {
        let mut extended = ptc(150, buy1_bits(), B_START);
        extended.center = Some(Center {
            zd: 100, zg: 200, dd: 90, gg: 210, start_index: B_START, end_index: 999,
        });
        let c = book2(vec![extended], vec![]);
        let ev = trend_ev((100, 200), 200);
        assert_eq!(
            bits_of(terminal_bits_at_event(&c, &ev, TerminalMatch::CWindow)),
            Some(buy1_bits()),
            "end/dd/gg/zd/zg 全异但 start_index 同 ⟹ owner=B"
        );
    }

    /// 关③ P2.5：Pan 域 同向一/二/三类全合法逐型（一类合法但非必需）；各型取最早
    /// 同向点；owner 不入判（center=None 合法，pan 同写 b_center_start 仅归因不作门）。
    #[test]
    fn c_window_pan_all_classes_legal() {
        let mut buy3 = BspBits::default();
        buy3.buy3 = true;
        let ev = pan_ev((100, 200), 200);
        for (bits, tag) in [
            (buy1_bits(), "一类合法（非必需）"),
            (buy2_bits(), "二类合法（盘背内生点类，027:18/057:40）"),
            (buy3, "三类合法（024:44 标准成因）"),
        ] {
            let c = book2(vec![pt(150, bits)], vec![]); // center=None 不拒
            assert_eq!(
                bits_of(terminal_bits_at_event(&c, &ev, TerminalMatch::CWindow)),
                Some(bits),
                "Pan 域 {tag}"
            );
        }
        // 最早性不论点类：三类早于一类 ⟹ 取三类。
        let c = book2(vec![pt(120, buy3), pt(150, buy1_bits())], vec![]);
        assert_eq!(
            bits_of(terminal_bits_at_event(&c, &ev, TerminalMatch::CWindow)),
            Some(buy3),
            "Pan 域取最早同向点（点类不参与最早性）"
        );
    }

    /// 关③ P2.1/不回滚条款：Pan 域 反向点方向隔离——027:92/027:740 证伪域（盘背
    /// 转化为反向三类点）由 `confirm_side` 方向谓词构造性排除，不得放宽。
    #[test]
    fn c_window_pan_reverse_side_isolated() {
        let mut sell2 = BspBits::default();
        sell2.sell2 = true;
        let mut sell3 = BspBits::default();
        sell3.sell3 = true;
        let c = book2(vec![pt(120, sell2), pt(130, sell3)], vec![]);
        let ev = pan_ev((100, 200), 200); // Long 盘背事件
        assert_eq!(
            terminal_bits_at_event(&c, &ev, TerminalMatch::CWindow),
            None,
            "Long 事件窗内仅反向（sell）点 ⟹ None（证伪域与背书域结构性无交）"
        );
        // 镜像：Short 盘背事件 + 同向 sell3 ⟹ 合法。
        let ev_short = NestCandidateEvent { side: Side::Short, ..pan_ev((100, 200), 200) };
        let c_short = book2(vec![pt(130, sell3)], vec![]);
        assert_eq!(
            bits_of(terminal_bits_at_event(&c_short, &ev_short, TerminalMatch::CWindow)),
            Some(sell3),
            "Short 事件同向三类合法"
        );
    }

    /// 关③ 点类分域 partition：同窗同账同 side，kind 分派决定合法域——Trend 域 ⊆
    /// Pan 域（Trend 合法 ⟹ Pan 合法，反向不成立）：Trend 跳过二类取后至一类，
    /// Pan 取最早同向点。
    #[test]
    fn c_window_kind_partition_trend_subset_of_pan() {
        let c = book2(
            vec![ptc(120, buy2_bits(), B_START), ptc(150, buy1_bits(), B_START)],
            vec![],
        );
        let trend = trend_ev((100, 200), 200);
        let pan = pan_ev((100, 200), 200);
        assert_eq!(
            bits_of(terminal_bits_at_event(&c, &trend, TerminalMatch::CWindow)),
            Some(buy1_bits()),
            "Trend 域：二类非法，后至一类 owner=B 合法"
        );
        assert_eq!(
            bits_of(terminal_bits_at_event(&c, &pan, TerminalMatch::CWindow)),
            Some(buy2_bits()),
            "Pan 域：最早同向点（二类）合法"
        );
        // Trend 合法点集 ⊆ Pan 合法点集：同一类 owner=B 点两域同命中。
        let c1 = book2(vec![ptc(150, buy1_bits(), B_START)], vec![]);
        assert_eq!(
            bits_of(terminal_bits_at_event(&c1, &trend, TerminalMatch::CWindow)),
            bits_of(terminal_bits_at_event(&c1, &pan, TerminalMatch::CWindow)),
        );
        // 反向不含：纯二类案 Trend None ∧ Pan Some。
        let c2 = book2(vec![ptc(120, buy2_bits(), B_START)], vec![]);
        assert_eq!(terminal_bits_at_event(&c2, &trend, TerminalMatch::CWindow), None);
        assert!(terminal_bits_at_event(&c2, &pan, TerminalMatch::CWindow).is_some());
    }
}
