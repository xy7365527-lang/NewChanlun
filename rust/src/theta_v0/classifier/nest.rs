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
//!
//! GUARD-ROLE: nest-pipeline
//! （#451；tests/nest_isolation_guard.rs 认上面这一整行豁免，不认文件名——
//! 独立对照实现本体，见 #449 §2 裁定）

use super::super::types::{BspBits, Center, Side, Tick};
use super::bsp::{BspPoint, OwnerRef};
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
        Self {
            confirm_src: None,
            child_interval: None,
            interval,
            cand,
        }
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
        NestCertificateBuilder {
            side,
            terminal,
            base_confirm_src: None,
            base_interval,
            rungs,
        }
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
    fn n_delta_rec(
        side: Side,
        terminal: &BspBits,
        base: &NestInterval,
        rungs: &[NestRung],
    ) -> bool {
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
            self.rungs.iter().enumerate().all(|(i, rung)| {
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
/// `Hash`：身份键（gate `seen`/`by_triple_anchor` 值域/索引 `by_id` 的 HashMap/HashSet 主键；
/// T5b (#208)：旧 `by_end`（seg_c_full 值桥）键域已删）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
        let violations = self
            .judge_at
            .windows(2)
            .filter(|pair| pair[0] > pair[1])
            .count();
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
    /// owner=B——#218 面 B 起判同机制换两族锚：一/三类核心区间 `(zd,zg)` 带判同（事件侧 B
    /// 由 `b_center_start` 当查找键在账本 centers 查出，序号只当键不当身份）、二类一类点
    /// 身份锚判同（`anchor_at(点侧一类点坐标) == (event.extreme_price, event.group_anchor)`，
    /// 全部精确等值无容差，spec owner-attribution-fix-20260724 ID-2）；`Consolidation` =
    /// 同向一/二/三类全合法（`confirm_side` 即精确语义，零改动）。本注释取代旧的
    /// 「窗口内首个破核点的判决中枢必 = B」结构身份保证——该保证已被 T1 复议证伪作废
    ///（bsp-terminal-endorsement-t1-review-20260718:92，frontier 吸收 37/37），不得复引。
    /// 方向由 `confirm_side` 锁死。
    CWindow,
}

/// 终端背书判定结构（关③ P3.2：`terminal_bits_at_event` 返回形状由 `Option<BspBits>`
/// 扩为携带点身份——bits + 命中点 owner 中枢 start_index；属同函数语义内扩展，
/// 消费方同步，不构成第二查法）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalEndorsement {
    /// 命中点的买卖点 bit 向量（方向已由 `confirm_side` 按事件 side 锁死）。
    pub bits: BspBits,
    /// 命中点 owner 中枢的 `start_index`（归因快照，**非判定输入**——#218 面 B 起判定
    /// 已换两族锚，本字段只作归因留痕）。`OwnerRef::Center` 载体取其中枢 `start_index`；
    /// `OwnerRef::Type1Anchor`（二类点，面 A 起归属载体 = 一类点锚坐标）无中枢序号，
    /// 快照如实为 `None`；点无载体（非生产合成形状）同为 `None`。
    pub owner_center_start: Option<usize>,
}

/// #218 面 B：owner 判同进核参照包（spec owner-attribution-fix-20260724 ID-2；T1 供给线
/// 单一来源，禁第二查法、禁 fork 判定核——生产与测量同一共核遍历副产品）。
#[derive(Clone, Copy)]
pub struct OwnerAnchorCtx<'a> {
    /// 点判同 oracle（**仅供二类点判同**）：`anchor_at(source_index)` =（极值价, 合并组锚），
    /// projection.rs:143-145 同一 T1 供给线查法（`fractal_at_source` + `merged_group_anchor`）。
    /// `None` = 点侧锚供给缺失 ⟹ 锚不可解（身份合取不可证，诚实判负 + 子计数单列）。
    pub anchor_at: &'a dyn Fn(usize) -> Option<(Tick, usize)>,
    /// 事件侧两元锚（二类判同参照）：`NestCandidateEventExt` 已带的
    /// `extreme_price`/`group_anchor`（事件构造点就 seg_c.1 = 离开段终点 = 趋势终点极值
    /// 解析，零新增解析），由构建调用方随事件透传。`(_, None)`/`(None, _)` = 事件侧锚
    /// 供给缺失 ⟹ 锚不可解（同上诚实判负）。
    pub event_anchor: (Option<Tick>, Option<usize>),
}

/// owner 判同三态（#218 面 B 核内中间量）：判等 / 判负（两侧可判且实不等）/ 不可解
///（锚供给缺失或 B 查找失败——身份合取不可证，诚实判负）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OwnerVerdict {
    Eq,
    Neq,
    Unresolvable,
}

/// #214 终端背书测量副产品（spec endorsement-failure-instrument-20260724 ID-2/ID-3；
/// 只读计数，判定结果零干预）。由判定核与生产合取**同一次账本遍历**产出——
/// 计数谓词 = 生产合取的分量分解（窗口 → `confirm_side` → owner），不引入任何新谓词。
///
/// 口径：遍历计数（`book_total`/`in_window`/`in_window_same_side`/`in_window_owner_eq`/
/// `owner_*_pts`/`trend_pts`）只对 `CWindow` 臂填充（`Exact` 诊断臂不产遍历计数）；
/// `book_missing` 与 `m` 无关（账本缺失时测量入口在分派前置位）。
/// owner/点级字段只对 `Trend` 域填充（Pan 域无 owner 合取，钉口径⑥ 事件级只计成功/失败）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EndorsementProbe {
    /// 账本级别缺失/越界（`event_bsp_book_level` None 或 `levels` 越界）——
    /// ID-2「无合法点」子计数单列。
    pub book_missing: bool,
    /// 账本（该级别 BSP 全本）总点数。
    pub book_total: usize,
    /// 窗口 `[c_start, t*]` 内点数（不限方向/bit 内容）。
    pub in_window: usize,
    /// 窗口内同向（`confirm_side`）点数。
    pub in_window_same_side: usize,
    /// 窗口内同向且 owner 判等（#218 面 B 两族锚判同：一/三类核心区间 `(zd,zg)` 带判同；
    /// 二类一类点身份锚判同——全部精确等值无容差，v3 硬禁令）点数。
    pub in_window_owner_eq: usize,
    /// 窗口内同向 owner 非等值中「锚不可解」点数（#218 面 B 语义重定，spec ID-4——
    /// 二类点侧/事件侧锚供给缺失、中枢参照 B 查找失败（生产不可达防御）、点无载体
    ///（非生产合成形状）同归：身份合取不可证，诚实判负，子计数单列，钉口径④）。
    pub owner_anchor_missing_pts: usize,
    /// 窗口内同向 owner 非等值中「实不等」点数（判同两侧皆可判且实不等：带不等 / 锚不等）。
    pub owner_real_neq_pts: usize,
    /// 带碰撞观察读数（#218 US-08）：中枢参照判同中**带等但序号不等**（同 (zd,zg) 但
    /// 非同 `b_center_start` 中枢）的点数——带判同收紧（加 dd/gg 外包络）与否的实测依据；
    /// 只读计数，不进判定。
    pub band_eq_start_neq_pts: usize,
    /// 点级二维计数（ID-3）：Trend 域窗口内同向点按 `[一/二/三类][owner 判等/不等]` 计数——
    /// 事件窗内逐窗各计（同一物理点落入多事件窗口逐窗各计一次，钉口径①）；一位多类点
    /// 按所属类逐行各计；「不等」列 = 实不等 + 锚不可解（两子计数分列见上）。C1 相等率
    /// 读数 = `[_][0] / ([_][0] + [_][1])`。
    pub trend_pts: [[usize; 2]; 3],
}

/// 单本账上的终端背书查法核（单一来源；账本只读，不改 `BspPoint` 任何字段/排序/去重）。
///
/// 窗口为 `c_start..=turn_source` 闭区间。`CWindow` 取窗口内**合法**点中 `source_index`
/// 最小者（`min_by_key` 平局取迭代序首个，确定性）——**合法性过滤先于最早性取点**
///（关③ P3.1：最早性只是确定性规则，二/三类点的在场不影响其后一类点的合法性，
/// 044:30 延迟常态；「最早合法」≠「最早」）。合法谓词按事件族 `kind` 分域（关③
/// P1/P2/P3，收紧唯一落点，调用点零限定、禁 fork）：
/// - `Trend`：`confirm_side(side) ∧ owner=B`——P1 重议（2026-07-21 编排者批准）后
///   同向一/二/三类全合法（一类非必需，与 P2 盘背域同构）；owner=B 判同机制（#218
///   面 B，spec ID-2）= 两族精确等值（一/三类核心区间 `(zd,zg)` 带判同；二类一类点
///   身份锚判同——判定核 [`terminal_bits_in_book_core`] 头注精确式）。`b_center_start`
///   = None（旧事件路径 B 身份缺失）⟹ 无合法点（身份合取不可证，诚实判负）。
/// - `Consolidation`：`confirm_side(side)` 即精确语义（同向一/二/三类全合法，一类非
///   必需，P2 裁决 5 零改动）；`b_center_start` 不入判（pan 同写仅归因用途，不作门）。
///
/// `Exact` 为位格等式 + confirm_side 的首个迭代序命中（旧查法语义平移；诊断常数不收紧）。
/// 窗口内无合法点 ⟹ None——037:18 否则条款域的诚实判负，非放宽、非误杀。
///
/// #214（spec ID-1C 私有核 + 双薄入口）：本入口 = 生产薄包装，判定语义单一来源收进
/// [`terminal_bits_in_book_core`]，测量副产品在此丢弃。#218 面 B：签名扩 `centers`
///（账本中枢列表，一/三类 B 带查找键域）与 `anchor_ctx`（二类判同 T1 oracle + 事件
/// 两元锚，spec ID-2 进核路径）。
#[allow(clippy::too_many_arguments)]
pub fn terminal_bits_in_book(
    bsp: &[BspPoint],
    centers: &[Center],
    c_start: usize,
    turn_source: usize,
    side: Side,
    kind: NestDivergenceKind,
    b_center_start: Option<usize>,
    m: TerminalMatch,
    anchor_ctx: &OwnerAnchorCtx,
) -> Option<TerminalEndorsement> {
    terminal_bits_in_book_core(
        bsp,
        centers,
        c_start,
        turn_source,
        side,
        kind,
        b_center_start,
        m,
        anchor_ctx,
    )
    .0
}

/// #214 判定核（核化：原 `terminal_bits_in_book` 判定逻辑收进私有核，合取序/窗口/方向
/// 谓词/owner 判定式逐字保留——ID-6.1 同函数语义内重构）。核同时产出判定结果与失败
/// 分类探针：同一次账本遍历逐点复算生产合取分量并副产品计数，测量口径与生产口径
/// 结构性不可漂移（US-05，无第二查法）。
///
/// ★#218 面 B（spec owner-attribution-fix-20260724 ID-2）：owner 判同从 `start_index`
/// 序号判同换**两族精确等值**（v3 硬禁令：无容差；零序号判同——#206 身份教义）：
/// - **中枢判同**（一/三类，`OwnerRef::Center` 载体）：`point.center.(zd, zg) == B.(zd, zg)`
///   ——中枢身份 = 其核心区间价格带（定义内容）；事件侧 B 由 `b_center_start` 在账本
///   `centers` 查出（**序号只当查找键、不当身份依据**——同一快照内一致；延伸只改写
///   end/dd/gg，start_index 与 zd/zg 稳定，p117 §7 实测 37/37）。价格带恒在场，无
///   「不可解」边角（B 查无 = 生产不可达防御，归锚不可解诚实判负）。
/// - **点判同**（二类，`OwnerRef::Type1Anchor` 载体）：`anchor_at(点侧一类点坐标) ==
///   (event.extreme_price, event.group_anchor)`——身份锚 =（极值价, 合并组锚）；锚解析
///   走 T1 供给线 oracle（[`OwnerAnchorCtx`]，projection.rs:143-145 同一单查，禁第二
///   查法）；事件自带 seg_c.1 两元锚，零新增解析。锚不可解（任一侧供给缺失）= 身份
///   合取不可证 = 诚实判负 + 子计数单列。
/// `b_center_start = None`（旧事件路径 B 身份缺失）⟹ 无合法点（诚实判负，语义不动）。
///
/// `CWindow` 臂生产语义 = `filter(窗口 ∧ confirm_side ∧ 点类分域)` →
/// `min_by_key(source_index)`（平局保留迭代序首个）；下方单次遍历中 `best` 只在严格
/// 更小 `source_index` 时替换 ⟺ `min_by_key` 首个平局保留——逐 bit 等价。
#[allow(clippy::too_many_arguments)]
fn terminal_bits_in_book_core(
    bsp: &[BspPoint],
    centers: &[Center],
    c_start: usize,
    turn_source: usize,
    side: Side,
    kind: NestDivergenceKind,
    b_center_start: Option<usize>,
    m: TerminalMatch,
    anchor_ctx: &OwnerAnchorCtx,
) -> (Option<TerminalEndorsement>, EndorsementProbe) {
    let endorsement = |point: &BspPoint| TerminalEndorsement {
        bits: point.bits,
        // #218 面 A 载体形态：Center 变体取中枢 start_index（归因快照，非判定输入）；
        // 二类点 Type1Anchor 载体无中枢序号（快照如实 None）。
        owner_center_start: match point.center {
            Some(OwnerRef::Center(c)) => Some(c.start_index),
            _ => None,
        },
    };
    match m {
        // issue #747 C1 口径订正（评审 review-747.log 条目③）：本行（`TerminalMatch::Exact`）
        // 字面同构 `bsp::bind_turn`（同 crate `classifier/bsp.rs`）单源函数体，但**不是**「生产
        // 绑定规则原始手写发生地」——`Exact` 仅诊断 bin（p117_s1a:759 / p125:474）与单测可达，
        // 生产恒走下方 `TerminalMatch::CWindow` 窗口臂（`min_by_key`，与本行不同形）。不改调
        // 单源 API 的理由是 GUARD-ROLE 对照臂「保持独立实现」纪律本身（本文件头 GUARD-ROLE
        // 声明），**不是** ADR-0005 禁令所迫——ADR-0005 条款 9 的禁令方向是「判据 crate 禁引
        // nest 产物」，nest.rs 反向调用同 crate `bsp` helper（本文件已 `use super::bsp::…`）
        // 不在该禁令射程内。其余 16+ 处同形态诊断复制点已单源改调 `bind_turn`/`bsp_at`/
        // `bsp_bit_at`；此处逐字保留独立实现，是对照臂设计意图，非引用规避。
        TerminalMatch::Exact => (
            bsp.iter()
                .find(|point| point.source_index == turn_source && point.bits.confirm_side(side))
                .map(endorsement),
            // Exact = 诊断常数臂，不产测量计数（装置只管生产 CWindow 口径，spec ID-2）。
            EndorsementProbe::default(),
        ),
        TerminalMatch::CWindow => {
            let mut probe = EndorsementProbe {
                book_total: bsp.len(),
                ..Default::default()
            };
            let mut best: Option<&BspPoint> = None;
            for point in bsp {
                if !(c_start <= point.source_index && point.source_index <= turn_source) {
                    continue; // 窗口外点：只经 book_total 入账（宽口径「窗外有点」= book_total>0）
                }
                probe.in_window += 1;
                if !point.bits.confirm_side(side) {
                    continue;
                }
                probe.in_window_same_side += 1;
                let legal = match kind {
                    NestDivergenceKind::Consolidation => true,
                    NestDivergenceKind::Trend => {
                        // P1 重议（2026-07-21 编排者批准）：Trend 域同向一/二/三类全合法
                        //（一类非必需，与 P2 盘背域同构）；owner=B 身份合取保留。
                        // #218 面 B：判同机制换两族锚（判定式见本函数头注，零序号判同）。
                        let owner = match point.center {
                            Some(OwnerRef::Center(pc)) => {
                                // 中枢判同（一/三类）：point.center.(zd,zg) == B.(zd,zg)；
                                // B 由 b_center_start 当查找键在账本 centers 查出。
                                match b_center_start
                                    .and_then(|bs| centers.iter().find(|c| c.start_index == bs))
                                {
                                    Some(b) if pc.zd == b.zd && pc.zg == b.zg => {
                                        // 带碰撞观察（US-08）：带等但序号不等 = 同带不同
                                        // 中枢可疑量级（收紧 dd/gg 与否的实测依据）。
                                        if pc.start_index != b.start_index {
                                            probe.band_eq_start_neq_pts += 1;
                                        }
                                        OwnerVerdict::Eq
                                    }
                                    Some(_) => OwnerVerdict::Neq,
                                    None => OwnerVerdict::Unresolvable, // B 查无（生产不可达防御）
                                }
                            }
                            Some(OwnerRef::Type1Anchor(src)) => {
                                // 点判同（二类）：anchor_at(一类点坐标) == 事件两元锚
                                //（精确等值无容差）；任一侧锚供给缺失 = 锚不可解。
                                // fail-closed 合一（codex 评审 2026-07-24 采纳）：
                                // `b_center_start = None`（旧事件路径 B 身份缺失）⟹
                                // 锚不可解判负——与中枢判同族同一「身份合取不可证 ⟹ 诚实
                                // 判负」纪律（生产路径 b_center_start 为 usize 恒 Some，
                                // 本臂仅合成/旧事件边角可达）。
                                if b_center_start.is_none() {
                                    OwnerVerdict::Unresolvable
                                } else {
                                    match ((anchor_ctx.anchor_at)(src), anchor_ctx.event_anchor) {
                                        (Some(pa), (Some(ep), Some(eg))) => {
                                            if pa == (ep, eg) {
                                                OwnerVerdict::Eq
                                            } else {
                                                OwnerVerdict::Neq
                                            }
                                        }
                                        _ => OwnerVerdict::Unresolvable,
                                    }
                                }
                            }
                            None => OwnerVerdict::Unresolvable, // 点无载体（非生产合成形状）
                        };
                        let owner_eq = matches!(owner, OwnerVerdict::Eq);
                        // ID-3 点级二维（类 × owner）：方向已锁，按事件 side 取对应类 bit。
                        let (b1, b2, b3) = match side {
                            Side::Long => (point.bits.buy1, point.bits.buy2, point.bits.buy3),
                            Side::Short => (point.bits.sell1, point.bits.sell2, point.bits.sell3),
                        };
                        for (row, on) in [(0usize, b1), (1, b2), (2, b3)] {
                            if on {
                                probe.trend_pts[row][usize::from(!owner_eq)] += 1;
                            }
                        }
                        match owner {
                            OwnerVerdict::Eq => probe.in_window_owner_eq += 1,
                            OwnerVerdict::Neq => probe.owner_real_neq_pts += 1,
                            // 锚不可解（语义重定，钉口径④）：供给缺失/B 查无/无载体同归。
                            OwnerVerdict::Unresolvable => probe.owner_anchor_missing_pts += 1,
                        }
                        owner_eq
                    }
                };
                if legal {
                    match best {
                        Some(b) if b.source_index <= point.source_index => {}
                        _ => best = Some(point),
                    }
                }
            }
            (best.map(endorsement), probe)
        }
    }
}

/// nest 事件的终端背书（级别对齐查法，p117 T1 = S0+S1a 合并项的生产单一来源；关③ P3 =
/// confirm_side 收紧唯一落点——kind 分域点类限定在本函数语义层，调用点零限定，禁 fork）。
///
/// 账本 = `levels[event_bsp_book_level(e.level)?].bsp`（一/三类判同的事件侧 B 带从同层
/// `centers` 查出）；窗口左端 = `e.interval_b.0`（c_start = `departure_move_c_start` 当前
/// episode 首腿，失败离开→回中枢→重新离开不桥接）；右端 = `e.turn_source`（R1 后 = 确认
/// 点 t*，因果序正，无前视入证）。点类限定按 `e.kind` 分派（Trend = 同向一/二/三类 ∧
/// owner=B——P1 重议 2026-07-21 编排者批准后一/二/三类全合法；#218 面 B 判同机制 = 两族
/// 精确等值：一/三类核心区间 `(zd,zg)` 带判同（B 由 `e.b_center_start` 当查找键查出，
/// 序号只当键不当身份）、二类一类点身份锚判同（`anchor_ctx` 进核，spec ID-2）；
/// Consolidation = 同向一/二/三类全合法）。账本级别缺失/越界 ⟹ None。
///
/// S0 的 A1 桥键（c 责任单元 `terminal_bits_bridged`）被级别移位 + C-b 同构吸收，
/// 不并列实装（裁定 T1.4，单一来源纪律：旧查法删除，不得保留为 fallback）。
///
/// #214（spec ID-1C）：本入口 = 生产薄包装（语义不动，探针副产品丢弃）；
/// 测量侧入口 = [`terminal_bits_at_event_measured`]（共核，无第二查法）。
pub fn terminal_bits_at_event(
    c: &Classification,
    e: &NestCandidateEvent,
    m: TerminalMatch,
    anchor_ctx: &OwnerAnchorCtx,
) -> Option<TerminalEndorsement> {
    terminal_bits_at_event_measured(c, e, m, anchor_ctx).0
}

/// #214 测量入口（spec ID-1C 双薄入口之测量侧；唯一消费方 =
/// `nest_index::build_nest_certificate_index`）：与生产入口共核——同一次账本遍历
/// 带出判定结果与失败分类探针（[`EndorsementProbe`]），判定分量与
/// [`terminal_bits_at_event`] 逐值相等（结构性不可漂移，US-05）。
pub fn terminal_bits_at_event_measured(
    c: &Classification,
    e: &NestCandidateEvent,
    m: TerminalMatch,
    anchor_ctx: &OwnerAnchorCtx,
) -> (Option<TerminalEndorsement>, EndorsementProbe) {
    let Some(book) = event_bsp_book_level(e.level).and_then(|level| c.levels.get(level)) else {
        // 账本级别缺失/越界（nest 不听 L0 之外的结构性 None）——ID-2「无合法点」
        // 子计数单列；生产入口同路径返回 None（语义不动）。
        return (
            None,
            EndorsementProbe {
                book_missing: true,
                ..Default::default()
            },
        );
    };
    terminal_bits_in_book_core(
        &book.bsp,
        &book.centers,
        e.interval_b.0,
        e.turn_source,
        e.side,
        e.kind,
        Some(e.b_center_start),
        m,
        anchor_ctx,
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
        (
            typed_interval(event, caliber).sel_key(),
            event.turn_source,
            index,
        )
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
    bases
        .iter()
        .filter_map(|base| {
            assemble_typed_certificate(events_by_level, base, top_level, caliber, &terminal_of)
        })
        .collect()
}

// ═════════ P2 证书生产装配层（strict-nesting-divergence-plan-20260708 §P2）═════════

/// `Cand^δ_ℓ` 定义式（P2 落定，补 spec 疑点2 的缺口；裁决① strict-nesting-rulings-20260708）：
///
/// > **`Cand^δ_ℓ(x) ≔ 塔上 per-level 背驰段谓词`** = [`CandDeltaEvent::cand_delta`]
/// > （`recursive_tower::level_cand_delta`：A/C 段按该级 tower 段结构定位（跨中枢趋势配对，
/// > 0016:62），gauge 复用 divergence.rs MacdArea 默认路径，严格 `curr < prev`）。
///
/// - 盘整背驰当前**不入谓词**（只出 [`CandDeltaEvent::pan_div_diag`] 诊断位，不参与本装配）——
///   此系**实装态而非裁定态**：0708「盘背不入链」裁决已由正式文书
///   `chanlun/escalate/nest-migration-ruling-20260716.md` 裁决⑤「盘背入链」（链背驰段域 = 趋势 ∪ 盘整，
///   typed 分流保留）supersede（#250/#260 查实，人裁签字）；provider 扩域缺口（Consolidation
///   `dir=None` 被跳过）在该文 §三附带发现登记在案，扩域实装落地前本装配维持趋势-only（清理票 #726）。
/// - `d_parent_interval` 仅保留 2026-07-10 历史基线的 episode 口径；正式装配由消费者显式
///   选择 [`d_parent_interval_snapshot`] 或 [`d_parent_interval_terminal`]。`confirm_src` 仍只
///   登记确认延迟，不作闸门或排序键。
/// - P1 已证：ℓ=e=0 时该谓词与 `extract_signals` buy1/sell1 背驰确认支**逐 bit 一致**
///   （strict_nest_check 硬门 PASS）⟹ 基例 `Cand^δ_e` 与 `Conf^δ_e` 确认支同源无分叉。
///
/// 历史兼容区间 `event.interval == event.c_episode_interval`；不得将本函数输出冒充完整 `c_p`。
pub fn d_parent_interval(ev: &CandDeltaEvent) -> NestInterval {
    debug_assert_eq!(
        ev.c_episode_start, ev.interval.0,
        "episode 左端别名必须一致"
    );
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
    assemble_certificate_with(events_by_level, base, top_level, terminal, &|event| {
        objects_by_level
            .get(event.level as usize)
            .and_then(|objects| d_parent_interval_terminal(event, objects))
    })
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
    debug_assert!(
        cert.n_delta(),
        "装配即校验：产出证书必过 n_delta（三门合取）"
    );
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
    use super::super::super::types::Center;
    use super::super::LevelState;
    use super::*;

    fn interval(et: u64, st: u64, idx: u64) -> NestInterval {
        NestInterval {
            end_time: et,
            start_time: st,
            idx,
        }
    }

    fn rung(et: u64, st: u64, idx: u64, cand: bool) -> NestRung {
        NestRung {
            confirm_src: None,
            child_interval: None,
            interval: interval(et, st, idx),
            cand,
        }
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
            LevelNode {
                cands: vec![interval(100, 0, 0)],
                chosen: interval(100, 0, 0),
            },
            LevelNode {
                cands: vec![interval(80, 10, 0)],
                chosen: interval(80, 10, 0),
            },
            LevelNode {
                cands: vec![interval(60, 20, 0)],
                chosen: interval(60, 20, 0),
            },
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
            LevelNode {
                cands: vec![interval(60, 20, 0)],
                chosen: interval(60, 20, 0),
            },
            LevelNode {
                cands: vec![interval(100, 0, 0)],
                chosen: interval(100, 0, 0),
            }, // end 超界
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
            chain: vec![LevelNode {
                cands: vec![interval(50, 0, 0)],
                chosen: interval(50, 0, 0),
            }],
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
        let cert_short = NestCertificate {
            side: Side::Short,
            ..cert.clone()
        };
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
            base_interval: interval(60, 20, 0), // J^δ_e
            rungs: vec![rung(100, 0, 0, true), rung(80, 10, 0, true)], // ℓ, ℓ-1
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
        let cert_long = NestCertificate {
            side: Side::Long,
            ..cert.clone()
        };
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
        assert_ne!(
            cert.base_confirm_src.unwrap() as u64,
            cert.base_interval.end_time
        );
        assert_eq!(cert.rungs[0].confirm_src, Some(70));
        assert_ne!(
            cert.rungs[0].confirm_src.unwrap() as u64,
            cert.rungs[0].interval.end_time
        );
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
        assert!(assemble_certificate_snapshot(&evs, &base, 0, BspBits::default()).is_none());
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
        let cert =
            assemble_certificate_snapshot(&evs, &base, 1, buy1_bits()).expect("回溯应找到可行父");
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
            1,
            Side::Long,
            NestDivergenceKind::Trend,
            (30, 50),
            (10, 70),
            50,
            100,
            true,
        );
        let parent = typed_event(
            2,
            Side::Long,
            NestDivergenceKind::Consolidation,
            (20, 80),
            (0, 100),
            80,
            200,
            true,
        );
        let events = vec![Vec::new(), vec![base.clone()], vec![parent]];
        let certificate =
            assemble_typed_certificate(&events, &base, 2, NestIntervalCaliber::B, &|_| {
                Some(buy1_bits())
            })
            .expect("B: child C [30,50] ⊆ parent C [20,80]");
        assert_eq!(
            certificate.certificate().base_interval(),
            interval(50, 30, 50)
        );
        assert_eq!(
            certificate.kinds(),
            &[NestDivergenceKind::Consolidation, NestDivergenceKind::Trend]
        );
        assert_eq!(
            certificate.d3_descent_stats(),
            (1, 1),
            "逆序只计 sidecar，不否决证书"
        );
        assert!(certificate.certificate().n_delta());
    }

    #[test]
    fn p97_rung_screening_is_structural_and_identities_align() {
        let base = typed_event(
            1,
            Side::Long,
            NestDivergenceKind::Consolidation,
            (30, 50),
            (10, 70),
            50,
            100,
            true,
        );
        // 父级 rung 力度未确认（divergence_confirmed=false）：D1 裁定下只看结构，不得否决链。
        let parent = typed_event(
            2,
            Side::Long,
            NestDivergenceKind::Consolidation,
            (20, 80),
            (0, 100),
            80,
            200,
            false,
        );
        let events = vec![Vec::new(), vec![base.clone()], vec![parent.clone()]];
        let certificate =
            assemble_typed_certificate(&events, &base, 2, NestIntervalCaliber::B, &|_| {
                Some(buy1_bits())
            })
            .expect("#97: rung 初筛只看结构，父级力度未确认不否决链");
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
            1,
            Side::Long,
            NestDivergenceKind::Consolidation,
            (30, 50),
            (10, 70),
            50,
            100,
            false,
        );
        let events = vec![Vec::new(), vec![pan.clone()]];
        assert!(
            assemble_typed_certificate(&events, &pan, 1, NestIntervalCaliber::B, &|_| Some(
                buy1_bits()
            ),)
            .is_none(),
            "力度未确认不产背驰段证书，但不改变上游 Cand 身份"
        );

        let confirmed = NestCandidateEvent {
            divergence_confirmed: true,
            ..pan
        };
        let events = vec![Vec::new(), vec![confirmed.clone()]];
        assert!(
            assemble_typed_certificate(&events, &confirmed, 1, NestIntervalCaliber::B, &|_| Some(
                BspBits::default()
            ),)
            .is_none(),
            "盘背不得强置同级 B1；终端仍须真实 Λ 非空"
        );
        let mut buy2 = BspBits::default();
        buy2.buy2 = true;
        assert!(
            assemble_typed_certificate(&events, &confirmed, 1, NestIntervalCaliber::B, &|_| Some(
                buy2
            ),)
            .is_some(),
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

    /// 携 owner 中枢参照载体的账本点夹具（一/三类；#218 面 A 载体形态 = OwnerRef::Center）。
    /// 判同机制（面 B）= 核心区间 (zd,zg) 带判同——本夹具带恒 (0,0)，配 book2c 的
    /// `ctr(0, 0, B_START)` 作 B 即带判等；要判负用 `ptb` 显式给异带中枢。
    fn ptc(source_index: usize, bits: BspBits, center_start: usize) -> BspPoint {
        BspPoint {
            center: Some(super::super::bsp::OwnerRef::Center(Center {
                zd: 0,
                zg: 0,
                dd: 0,
                gg: 0,
                start_index: center_start,
                end_index: 0,
            })),
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
                1,
                Side::Long,
                NestDivergenceKind::Trend,
                interval_b,
                (interval_b.0.saturating_sub(10), turn_source + 10),
                turn_source,
                turn_source,
                true,
            )
        }
    }

    /// 关③ Pan 域事件夹具：kind=Consolidation（owner 快照不入判，置 0）。
    fn pan_ev(interval_b: (usize, usize), turn_source: usize) -> NestCandidateEvent {
        typed_event(
            1,
            Side::Long,
            NestDivergenceKind::Consolidation,
            interval_b,
            (interval_b.0.saturating_sub(10), turn_source + 10),
            turn_source,
            turn_source,
            true,
        )
    }

    /// 两级分类夹具：levels[0]/levels[1] 各持一本 BSP 账，其余字段默认（全 pub，无 parser 依赖）。
    fn book2(l0: Vec<BspPoint>, l1: Vec<BspPoint>) -> Classification {
        Classification {
            levels: vec![
                LevelState {
                    bsp: std::rc::Rc::new(l0),
                    ..Default::default()
                },
                LevelState {
                    bsp: std::rc::Rc::new(l1),
                    ..Default::default()
                },
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
            1,
            Side::Long,
            NestDivergenceKind::Trend,
            (90, 100),
            (80, 110),
            100,
            100,
            true,
        );
        assert_eq!(
            bits_of(terminal_bits_at_event(
                &c,
                &ev,
                TerminalMatch::Exact,
                &ctx_degenerate()
            )),
            Some(buy1_bits())
        );
        // 仅 levels[1] 有 ⟹ None（新语义防回归：移位后不再读 levels[ℓ]）。
        let c_l1_only = book2(vec![], vec![pt(100, buy1_bits())]);
        assert_eq!(
            terminal_bits_at_event(&c_l1_only, &ev, TerminalMatch::Exact, &ctx_degenerate()),
            None
        );
        // side 反 ⟹ None。
        let ev_short = NestCandidateEvent {
            side: Side::Short,
            ..ev
        };
        assert_eq!(
            terminal_bits_at_event(&c, &ev_short, TerminalMatch::Exact, &ctx_degenerate()),
            None
        );
        // event.level=0 ⟹ None（nest 不听 L0）。
        let ev_l0 = NestCandidateEvent { level: 0, ..ev };
        assert_eq!(
            terminal_bits_at_event(&c, &ev_l0, TerminalMatch::Exact, &ctx_degenerate()),
            None
        );
        // level 越界 ⟹ None。
        let ev_oob = NestCandidateEvent { level: 5, ..ev };
        assert_eq!(
            terminal_bits_at_event(&c, &ev_oob, TerminalMatch::Exact, &ctx_degenerate()),
            None
        );
    }

    #[test]
    fn terminal_bits_at_event_c_window_picks_earliest_confirm() {
        // #218 面 B（机械改写归因：序号等值构造 → 带等值构造）：账本 centers 供 B 带，
        // ptc 点带恒 (0,0) == B 带 ⟹ 带判等合法（同带异序号亦判等，见 i218 专测）。
        let c = book2c(
            vec![
                pt(99, buy1_bits()),            // 窗口左端外 ⟹ 排除
                ptc(120, buy1_bits(), B_START), // 窗口内最早合法点（一类 ∧ owner=B）⟹ 命中
                pt(130, sell1_bits()),          // 反向 ⟹ 忽略（confirm_side 方向隔离）
                ptc(150, buy2_bits(), B_START), // 同向二类带判等合法（P1 重议）但晚于 @120
                pt(201, buy1_bits()),           // 窗口右端外 ⟹ 排除
            ],
            vec![],
            vec![ctr(0, 0, B_START)],
        );
        let ev = trend_ev((100, 200), 200);
        assert_eq!(
            bits_of(terminal_bits_at_event(
                &c,
                &ev,
                TerminalMatch::CWindow,
                &ctx_degenerate()
            )),
            Some(buy1_bits()),
            "窗口 [100,200] 内最早合法点 @120（一类 ∧ owner=B）"
        );
        // 空窗（窗口内无任何点）⟹ None。
        let c_empty = book2(vec![pt(99, buy1_bits()), pt(201, buy1_bits())], vec![]);
        assert_eq!(
            terminal_bits_at_event(&c_empty, &ev, TerminalMatch::CWindow, &ctx_degenerate()),
            None
        );
    }

    #[test]
    fn assemble_typed_certificate_terminal_uses_shifted_book() {
        let base = NestCandidateEvent {
            b_center_start: B_START,
            ..typed_event(
                1,
                Side::Long,
                NestDivergenceKind::Trend,
                (30, 50),
                (10, 70),
                50,
                100,
                true,
            )
        };
        let events = vec![Vec::new(), vec![base]];
        // 基例 level=1 confirmed + levels[0] 窗内合法背书点（一类 ∧ owner=B）@40 ⟹ Some(cert)。
        // #218 面 B（机械改写归因：带等值构造）：levels[0] 带判等点 @40。
        let c = book2c(
            vec![ptc(40, buy1_bits(), B_START)],
            vec![],
            vec![ctr(0, 0, B_START)],
        );
        let cert = assemble_typed_certificate(&events, &base, 1, NestIntervalCaliber::B, &|e| {
            bits_of(terminal_bits_at_event(
                &c,
                e,
                TerminalMatch::CWindow,
                &ctx_degenerate(),
            ))
        })
        .expect("levels[0] 窗内背书点 ⟹ 证书成立");
        assert_eq!(cert.certificate().terminal(), &buy1_bits());
        // 删该点 ⟹ None。
        let c_none = book2(vec![], vec![]);
        assert!(
            assemble_typed_certificate(&events, &base, 1, NestIntervalCaliber::B, &|e| bits_of(
                terminal_bits_at_event(&c_none, e, TerminalMatch::CWindow, &ctx_degenerate())
            ),)
            .is_none()
        );
        // 点只在 levels[1]（levels[0] 空）⟹ None——移位后不再读 levels[1]，防旧语义回潮。
        let c_l1_only = book2(vec![], vec![ptc(40, buy1_bits(), B_START)]);
        assert!(
            assemble_typed_certificate(&events, &base, 1, NestIntervalCaliber::B, &|e| bits_of(
                terminal_bits_at_event(&c_l1_only, e, TerminalMatch::CWindow, &ctx_degenerate())
            ),)
            .is_none()
        );
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
                1,
                Side::Long,
                NestDivergenceKind::Trend,
                (726527, t_star),
                (726400, t_star),
                t_star,
                t_star,
                true,
            )
        }
    }

    /// 原 `terminal_bridge_exact_hit_degenerates`：t*=726600 恰有合法背书点（一类 ∧
    /// owner=B）时，C-b 与 C-a 逐位一致（向后兼容锁定；Exact = 诊断常数不收紧）。
    #[test]
    fn c_window_hit_at_tstar_degenerates_to_exact() {
        // #218 面 B（机械改写归因：序号等值 → 带等值构造）。
        let c = book2c(
            vec![ptc(726600, buy1_bits(), B_START)],
            vec![],
            vec![ctr(0, 0, B_START)],
        );
        let ev = s0_event(726600);
        assert_eq!(
            bits_of(terminal_bits_at_event(
                &c,
                &ev,
                TerminalMatch::CWindow,
                &ctx_degenerate()
            )),
            Some(buy1_bits())
        );
        assert_eq!(
            bits_of(terminal_bits_at_event(
                &c,
                &ev,
                TerminalMatch::Exact,
                &ctx_degenerate()
            )),
            Some(buy1_bits())
        );
    }

    /// 原 `terminal_bridge_skips_center_being_left_unit`：B 自身结构代表点 @726520
    ///（< c_start=726527，其一类点见证进入 B 的 b-vs-a 前序背驰）被窗口左端结构性排除
    ///（F1 跨结构误记防护在 C-b 下由 c_start 左端承担）⟹ None。
    #[test]
    fn c_window_left_edge_excludes_prior_structure_point() {
        let c = book2(vec![pt(726520, buy1_bits())], vec![]);
        let ev = s0_event(726560);
        assert_eq!(
            terminal_bits_at_event(&c, &ev, TerminalMatch::CWindow, &ctx_degenerate()),
            None
        );
    }

    /// 原 `terminal_bridge_selects_first_c_unit_covering_tstar`：t*=726650 深在 c 内，
    /// @726600 buy1（owner=B）命中、@726680 buy2 越 t* 排除——
    /// 窗口右端 = t*；多中枢 c（037:22）内的合法确认被覆盖。
    /// #218 面 B（机械改写归因：带等值构造）。
    #[test]
    fn c_window_tstar_deep_in_c_picks_earliest_within_window() {
        let c = book2c(
            vec![ptc(726600, buy1_bits(), B_START), pt(726680, buy2_bits())],
            vec![],
            vec![ctr(0, 0, B_START)],
        );
        let ev = s0_event(726650);
        assert_eq!(
            bits_of(terminal_bits_at_event(
                &c,
                &ev,
                TerminalMatch::CWindow,
                &ctx_degenerate()
            )),
            Some(buy1_bits())
        );
    }

    /// 原 `terminal_bridge_witness_not_formed_returns_none`：窗口 [726527,726560] 内
    /// 无任何点（@726520/@726600 皆窗外）⟹ None——037:18 否则条款域的诚实判负。
    #[test]
    fn c_window_witness_absent_returns_none() {
        let c = book2(
            vec![pt(726520, buy1_bits()), pt(726600, buy1_bits())],
            vec![],
        );
        let ev = s0_event(726560);
        assert_eq!(
            terminal_bits_at_event(&c, &ev, TerminalMatch::CWindow, &ctx_degenerate()),
            None
        );
    }

    /// 原 `terminal_bridge_wrong_side_rejected`：窗口内仅 sell1@726600 ⟹ Long 事件
    /// None——confirm_side 方向过滤保持。
    #[test]
    fn c_window_wrong_side_rejected() {
        let c = book2(vec![pt(726600, sell1_bits())], vec![]);
        let ev = s0_event(726600);
        assert_eq!(
            terminal_bits_at_event(&c, &ev, TerminalMatch::CWindow, &ctx_degenerate()),
            None
        );
    }

    /// 原 `terminal_bridge_first_confirm_at_shared_endpoint`：同坐标三点（零 bit + buy1
    /// + buy2）⟹ Some(buy1)——关③ Trend 域下零 bit 拒、buy2 二类点类拒，buy1（owner=B）
    /// 为唯一合法点（合法性过滤先于最早性/迭代序）。
    #[test]
    fn c_window_first_confirm_at_shared_coordinate() {
        // #218 面 B（机械改写归因：带等值构造；buy1 迭代序先于 buy2，合法过滤先于最早性）。
        let c = book2c(
            vec![
                pt(726600, BspBits::default()),
                ptc(726600, buy1_bits(), B_START),
                ptc(726600, buy2_bits(), B_START),
            ],
            vec![],
            vec![ctr(0, 0, B_START)],
        );
        let ev = s0_event(726600);
        assert_eq!(
            bits_of(terminal_bits_at_event(
                &c,
                &ev,
                TerminalMatch::CWindow,
                &ctx_degenerate()
            )),
            Some(buy1_bits())
        );
    }

    /// 原 `terminal_bridge_pan_event_same_contract`：kind=Consolidation 事件共用同一
    /// 查法（其 interval_b.0 = pan C 起点）⟹ Some——关③ P2：Pan 域谓词 = confirm_side
    /// 精确语义（同向一/二/三类全合法），owner 不入判（center=None 亦合法）。
    #[test]
    fn c_window_pan_event_same_contract() {
        let c = book2(vec![pt(726600, buy1_bits())], vec![]);
        let ev = typed_event(
            1,
            Side::Long,
            NestDivergenceKind::Consolidation,
            (726541, 726600),
            (726400, 726600),
            726600,
            726600,
            true,
        );
        assert_eq!(
            bits_of(terminal_bits_at_event(
                &c,
                &ev,
                TerminalMatch::CWindow,
                &ctx_degenerate()
            )),
            Some(buy1_bits())
        );
    }

    /// 原 `terminal_bridge_past_last_unit_returns_none` 的 C-b 消解：t*=726999 越过
    /// 账本全部点，窗口内最早合法点 @726540 仍背书 ⟹ Some——A1「越末单元 ⟹ None」边界与
    /// F2「背书晚于确认」语义差在 C-b 下结构性消解（最早点 ≤ t*，因果序正；裁定 T1.4）。
    /// 关③ Trend 域：@726600 二类非法，@726540 一类 owner=B 合法。
    #[test]
    fn c_window_tstar_past_all_points_still_endorses_earliest() {
        // #218 面 B（机械改写归因：带等值构造）。
        let c = book2c(
            vec![ptc(726540, buy1_bits(), B_START), pt(726600, buy2_bits())],
            vec![],
            vec![ctr(0, 0, B_START)],
        );
        let ev = s0_event(726999);
        assert_eq!(
            bits_of(terminal_bits_at_event(
                &c,
                &ev,
                TerminalMatch::CWindow,
                &ctx_degenerate()
            )),
            Some(buy1_bits())
        );
    }

    // ───────── 关③ P3 confirm_side 收紧：点类分域 + owner=B start_index 判同 ─────────

    /// 关③ P1/验收线：Trend 域 一类 ∧ owner=B 命中；判定结构携带 owner start_index。
    #[test]
    fn c_window_trend_type1_owner_b_hits() {
        // #218 面 B（机械改写归因：序号等值 → 带等值构造——判同机制换核心区间带判同）：
        // 点 center 带 == B 带 ⟹ 判等；owner_center_start 归因快照语义不动（Center 载体）。
        let c = book2c(
            vec![ptc(150, buy1_bits(), B_START)],
            vec![],
            vec![ctr(0, 0, B_START)],
        );
        let ev = trend_ev((100, 200), 200);
        let endo = terminal_bits_at_event(&c, &ev, TerminalMatch::CWindow, &ctx_degenerate())
            .expect("一类 ∧ owner=B（带判同）⟹ 合法背书");
        assert_eq!(endo.bits, buy1_bits());
        assert_eq!(
            endo.owner_center_start,
            Some(B_START),
            "归因快照 = Center 载体 start_index（非判定输入）"
        );
    }

    /// 关③ P1.3：Trend 域 一类 owner≠B 拒（窗内他中枢一类点——prev/OTHER/新生中枢——
    /// 不作本事件背书）；owner 载体缺失（center=None——★owner 载体补齐（补记② 路径 (a)）
    /// 后已非生产形状，此处合成构造锁「载体缺失 ⟹ 拒」的防御语义）同样拒——
    /// 身份合取不可证，诚实判负（非放宽）。
    #[test]
    fn c_window_trend_type1_owner_not_b_rejected() {
        // #218 面 B（机械改写归因：序号不等 → 带不等构造）：异带中枢 owner≠B ⟹ 拒。
        let ev = trend_ev((100, 200), 200);
        let c_other = book2c(
            vec![ptb(150, buy1_bits(), ctr(0, 1, B_START + 9))],
            vec![],
            vec![ctr(0, 0, B_START)],
        );
        assert_eq!(
            terminal_bits_at_event(&c_other, &ev, TerminalMatch::CWindow, &ctx_degenerate()),
            None,
            "一类但带不等（owner≠B）⟹ 非法背书"
        );
        let c_none = book2(vec![pt(150, buy1_bits())], vec![]);
        assert_eq!(
            terminal_bits_at_event(&c_none, &ev, TerminalMatch::CWindow, &ctx_degenerate()),
            None,
            "center=None ⟹ 锚不可解 ⟹ owner 不可证 ⟹ 拒"
        );
    }

    /// P1 重议（2026-07-21 编排者批准）：Trend 域 同向二/三类点合法（owner=B 时）——
    /// 与 P2（盘背域）同构。全定义策略要求趋势域完全覆盖三类买卖点。
    #[test]
    fn c_window_trend_type2_type3_accepted() {
        // #218 面 A+B（机械改写归因：二类载体 = 一类点锚 + 锚判等；三类 = 带判等）：
        // 二类点一类锚坐标 30，oracle(30)=(900,70) == 事件两元锚 ⟹ 判等。
        static ORACLE: &[(usize, (Tick, usize))] = &[(30, (900, 70))];
        let oracle = synth_oracle(ORACLE);
        let ctx = OwnerAnchorCtx {
            anchor_at: &oracle,
            event_anchor: (Some(900), Some(70)),
        };
        let mut buy3 = BspBits::default();
        buy3.buy3 = true;
        let c = book2c(
            vec![pta(120, buy2_bits(), 30), ptc(130, buy3, B_START)],
            vec![],
            vec![ctr(0, 0, B_START)],
        );
        let ev = trend_ev((100, 200), 200);
        let result = terminal_bits_at_event(&c, &ev, TerminalMatch::CWindow, &ctx)
            .expect("同向二类锚判等 ⟹ Trend 域合法（P1 重议 + #218 锚判同）");
        assert_eq!(result.bits, buy2_bits(), "取最早同向点 @120 buy2");
    }

    /// P1 重议后：Trend 域二三类合法，取最早同向 owner=B 点（不再跳过二类取后至一类）。
    #[test]
    fn c_window_trend_earliest_same_direction_owner_b() {
        // #218 面 A+B（机械改写归因：二类载体 = 一类点锚 + 锚判等；一类 = 带判等）：
        // 取最早同向 owner 判等点 @120 buy2（锚判等），不取后至一类。
        static ORACLE: &[(usize, (Tick, usize))] = &[(30, (900, 70))];
        let oracle = synth_oracle(ORACLE);
        let ctx = OwnerAnchorCtx {
            anchor_at: &oracle,
            event_anchor: (Some(900), Some(70)),
        };
        let c = book2c(
            vec![pta(120, buy2_bits(), 30), ptc(150, buy1_bits(), B_START)],
            vec![],
            vec![ctr(0, 0, B_START)],
        );
        let ev = trend_ev((100, 200), 200);
        let endo = terminal_bits_at_event(&c, &ev, TerminalMatch::CWindow, &ctx)
            .expect("P1 重议 + #218：二类锚判等合法");
        assert_eq!(
            endo.bits,
            buy2_bits(),
            "P1 重议：取最早同向 owner 判等点 @120 buy2"
        );
        assert_eq!(
            endo.owner_center_start, None,
            "二类点载体 = 一类点锚（Type1Anchor）无中枢序号，归因快照如实 None（#218 面 A）"
        );
    }

    /// #218 面 B（spec ID-2 中枢判同族；机械改写归因：判同机制 start_index → (zd,zg)
    /// 带判同，旧测试 `c_window_trend_owner_extension_start_index_judges` 语义由本测试继承）：
    /// 带判同只看核心区间——(a) **序号漂移**：点 center 与 B 同带但 start_index/end/dd/gg
    /// 全异 ⟹ 判等（延伸只改写 end/dd/gg、序号漂移均不影响带判同；旧序号判同必判负）；
    /// (b) 同序号但带异 ⟹ 判负（旧序号判同必判等——「同中枢序号 ≠ 同归属链」#44 先例）。
    #[test]
    fn c_window_trend_owner_band_judges_over_start_index() {
        let mut extended = ptc(150, buy1_bits(), B_START);
        extended.center = Some(super::super::bsp::OwnerRef::Center(Center {
            zd: 100,
            zg: 200,
            dd: 90,
            gg: 210,
            start_index: B_START + 77,
            end_index: 999,
        }));
        let c = book2c(vec![extended], vec![], vec![ctr(100, 200, B_START)]);
        let ev = trend_ev((100, 200), 200);
        assert_eq!(
            bits_of(terminal_bits_at_event(
                &c,
                &ev,
                TerminalMatch::CWindow,
                &ctx_degenerate()
            )),
            Some(buy1_bits()),
            "(a) 带同但序号/外包络全异 ⟹ 带判同判等（序号判同必判负）"
        );
        let mut drifted = ptc(150, buy1_bits(), B_START);
        drifted.center = Some(super::super::bsp::OwnerRef::Center(Center {
            zd: 100,
            zg: 201,
            dd: 90,
            gg: 210,
            start_index: B_START,
            end_index: 0,
        }));
        let c2 = book2c(vec![drifted], vec![], vec![ctr(100, 200, B_START)]);
        assert_eq!(
            terminal_bits_at_event(&c2, &ev, TerminalMatch::CWindow, &ctx_degenerate()),
            None,
            "(b) 同序号但带异 ⟹ 带判同判负（序号判同必判等）"
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
                bits_of(terminal_bits_at_event(
                    &c,
                    &ev,
                    TerminalMatch::CWindow,
                    &ctx_degenerate()
                )),
                Some(bits),
                "Pan 域 {tag}"
            );
        }
        // 最早性不论点类：三类早于一类 ⟹ 取三类。
        let c = book2(vec![pt(120, buy3), pt(150, buy1_bits())], vec![]);
        assert_eq!(
            bits_of(terminal_bits_at_event(
                &c,
                &ev,
                TerminalMatch::CWindow,
                &ctx_degenerate()
            )),
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
            terminal_bits_at_event(&c, &ev, TerminalMatch::CWindow, &ctx_degenerate()),
            None,
            "Long 事件窗内仅反向（sell）点 ⟹ None（证伪域与背书域结构性无交）"
        );
        // 镜像：Short 盘背事件 + 同向 sell3 ⟹ 合法。
        let ev_short = NestCandidateEvent {
            side: Side::Short,
            ..pan_ev((100, 200), 200)
        };
        let c_short = book2(vec![pt(130, sell3)], vec![]);
        assert_eq!(
            bits_of(terminal_bits_at_event(
                &c_short,
                &ev_short,
                TerminalMatch::CWindow,
                &ctx_degenerate()
            )),
            Some(sell3),
            "Short 事件同向三类合法"
        );
    }

    /// P1 重议后：Trend 域和 Pan 域同构——都取最早同向 owner=B 点。
    #[test]
    fn c_window_kind_trend_pan_same_behavior() {
        // #218 面 B（机械改写归因：带等值构造）：Trend 侧 owner 带判等同构 Pan 侧零门。
        let c = book2c(
            vec![
                ptc(120, buy2_bits(), B_START),
                ptc(150, buy1_bits(), B_START),
            ],
            vec![],
            vec![ctr(0, 0, B_START)],
        );
        let trend = trend_ev((100, 200), 200);
        let pan = pan_ev((100, 200), 200);
        assert_eq!(
            bits_of(terminal_bits_at_event(
                &c,
                &trend,
                TerminalMatch::CWindow,
                &ctx_degenerate()
            )),
            Some(buy2_bits()),
            "Trend 域：P1 重议后取最早同向 owner 判等点"
        );
        assert_eq!(
            bits_of(terminal_bits_at_event(
                &c,
                &pan,
                TerminalMatch::CWindow,
                &ctx_degenerate()
            )),
            Some(buy2_bits()),
            "Pan 域：最早同向点（二类）合法"
        );
        let c1 = book2c(
            vec![ptc(150, buy1_bits(), B_START)],
            vec![],
            vec![ctr(0, 0, B_START)],
        );
        assert_eq!(
            bits_of(terminal_bits_at_event(
                &c1,
                &trend,
                TerminalMatch::CWindow,
                &ctx_degenerate()
            )),
            bits_of(terminal_bits_at_event(
                &c1,
                &pan,
                TerminalMatch::CWindow,
                &ctx_degenerate()
            )),
        );
        let c2 = book2c(
            vec![ptc(120, buy2_bits(), B_START)],
            vec![],
            vec![ctr(0, 0, B_START)],
        );
        assert!(
            terminal_bits_at_event(&c2, &trend, TerminalMatch::CWindow, &ctx_degenerate())
                .is_some()
        );
        assert!(
            terminal_bits_at_event(&c2, &pan, TerminalMatch::CWindow, &ctx_degenerate()).is_some()
        );
    }

    // ───────── #214 背书失败原因测量装置（spec endorsement-failure-instrument-20260724，
    // tracker #215；主接缝 = `build_nest_certificate_index`，spec Testing Decisions
    // 接缝清单 1——事件账本进、索引+统计出，一个接缝覆盖装置全部外部行为）─────────
    //
    // 覆盖：四桶归属（ID-2 穿透序 窗口→方向→owner 首因互斥完备）、完备性平账、
    // 点级（类 × owner）二维计数（ID-3，含成功事件）、按级 × kind 分解（ID-4，钉口径②）、
    // 索引不变性（ID-6/US-06：既有六字段语义/张数/身份键/rungs 不受装置影响）。
    // 构造器复用本模块 book2/pt/ptc/typed_event/trend_ev/pan_ev 族（spec 指定先例）。

    /// #214 主接缝薄封装：caliber 固定生产口径 B（cert-index-caliber-audit-20260721）。
    /// #218 面 B：主接缝签名扩 oracle + 事件锚查找（spec ID-2）；无二类点用例传
    /// 恒缺锚供给（锚判同不被触发），二类用例经 i218_build 显式给合成锚。
    fn i214_build(
        c: &Classification,
        evs: Vec<Vec<NestCandidateEvent>>,
    ) -> super::super::nest_index::NestCertificateIndex {
        super::super::nest_index::build_nest_certificate_index(
            c,
            &evs,
            NestIntervalCaliber::B,
            &never_anchor,
            &no_event_anchor,
        )
    }

    /// ID-2 规则3：窗内有同向点但 owner 判等 0 ⟹ owner 锚不等桶（#218 面 B 桶名）；
    /// 锚不可解（点无载体）与实不等（带不等）同归本桶、子计数分列（钉口径④）。
    /// #218 面 B（机械改写归因：序号等值构造 → 带等值/锚构造）。
    #[test]
    fn i214_owner_start_neq_bucket_identity_subcounts() {
        let c = book2c(
            vec![
                ptb(150, buy1_bits(), ctr(0, 1, B_START + 9)), // 一类但带不等 ⟹ 实不等
                pt(151, buy1_bits()),                          // 无载体 ⟹ 锚不可解
            ],
            vec![],
            vec![ctr(0, 0, B_START)],
        );
        let ev1 = trend_ev((100, 200), 200);
        let ev2 = trend_ev((100, 201), 201); // 同窗两点、身份键互异（turn_source 区分）
        let index = i214_build(&c, vec![vec![], vec![ev1, ev2]]);
        let inst = index.instrument();
        assert_eq!(inst.trend_owner_anchor_neq, 2, "两事件同归 owner 锚不等桶");
        assert_eq!(
            inst.trend_success
                + inst.trend_out_of_window
                + inst.trend_opposite_side
                + inst.trend_no_valid_point,
            0
        );
        assert_eq!(
            inst.owner_real_neq_pts, 2,
            "实不等点逐事件窗各计（钉口径① 事件窗内）"
        );
        assert_eq!(
            inst.owner_anchor_missing_pts, 2,
            "无载体归锚不可解子计数（#218 语义重定）"
        );
        assert_eq!(
            inst.trend_pts[0],
            [0, 4],
            "一类点 2 点 × 2 事件窗 = 4，全 owner 不等"
        );
        assert_eq!(index.stats().base_events, 2);
        assert_eq!(index.stats().assembled, 0);
        assert!(index.is_empty());
    }

    /// ID-2 规则1（宽口径，钉口径③）：窗内无点 ∧ 账本窗外有点（不限方向）⟹ 窗口外桶。
    #[test]
    fn i214_out_of_window_bucket_wide_caliber() {
        let c = book2(
            vec![
                pt(99, sell1_bits()),           // 窗左外（异向亦计「账本窗外有点」——宽口径）
                ptc(250, buy1_bits(), B_START), // 窗右外同向
            ],
            vec![],
        );
        let ev = trend_ev((100, 200), 200);
        let index = i214_build(&c, vec![vec![], vec![ev]]);
        let inst = index.instrument();
        assert_eq!(inst.trend_out_of_window, 1);
        assert_eq!(
            inst.trend_no_valid_point, 0,
            "账本有点（虽窗外）不归无合法点"
        );
        assert_eq!(
            inst.trend_success + inst.trend_owner_anchor_neq + inst.trend_opposite_side,
            0
        );
    }

    /// ID-2 规则2：窗内有点但同向点数 0（异向/零 bit 点在场）⟹ 异向桶。
    #[test]
    fn i214_opposite_side_bucket() {
        let c = book2(
            vec![
                pt(150, sell1_bits()),       // 异向
                pt(160, BspBits::default()), // 零 bit（非 confirm_side）——窗内有点但同向 0
            ],
            vec![],
        );
        let ev = trend_ev((100, 200), 200);
        let index = i214_build(&c, vec![vec![], vec![ev]]);
        let inst = index.instrument();
        assert_eq!(inst.trend_opposite_side, 1);
        assert_eq!(
            inst.trend_success
                + inst.trend_owner_anchor_neq
                + inst.trend_out_of_window
                + inst.trend_no_valid_point,
            0
        );
    }

    /// ID-2 规则1：账本无任何点 ⟹ 无合法点（book_empty 子计数）；账本级别缺失/越界
    /// ⟹ 无合法点（book_missing 子计数单列——nest 不听 L0 之外的结构性 None）。
    #[test]
    fn i214_no_valid_point_subcounts() {
        let c = book2(vec![], vec![]); // levels[0] 空账；levels[2] 越界缺失
        let ev_empty = trend_ev((100, 200), 200); // level1 → 读 levels[0]（空账）
        let ev_missing = NestCandidateEvent {
            b_center_start: B_START,
            ..typed_event(
                3,
                Side::Short,
                NestDivergenceKind::Trend,
                (100, 200),
                (90, 210),
                200,
                200,
                true,
            )
        }; // level3 → 读 levels[2]（越界）
        let index = i214_build(&c, vec![vec![], vec![ev_empty], vec![], vec![ev_missing]]);
        let inst = index.instrument();
        assert_eq!(inst.trend_no_valid_point, 2);
        assert_eq!(inst.nvp_book_empty, 1, "账本空（任何点都无）");
        assert_eq!(inst.nvp_book_missing, 1, "级别越界结构性 None 单列");
        assert_eq!(inst.trend_out_of_window, 0);
        assert_eq!(
            inst.trend_success + inst.trend_owner_anchor_neq + inst.trend_opposite_side,
            0
        );
    }

    /// ID-2 规则4 + ID-3：窗内有同向 owner 判等点 ⟹ 判定必命中（成功不计桶）；
    /// 成功事件的窗内同向点同样入点级二维计数（相等率分母覆盖成功+失败全体）。
    /// #218 面 A+B（机械改写归因：二类载体 = 一类点锚 + 锚判等；一类实不等 = 带不等）。
    #[test]
    fn i214_success_not_bucketed_points_counted() {
        static ORACLE: &[(usize, (Tick, usize))] = &[(30, (900, 70))];
        let oracle = synth_oracle(ORACLE);
        let event_anchor = |_: &NestCandidateEvent| (Some(900), Some(70));
        let c = book2c(
            vec![
                pta(120, buy2_bits(), 30), // 二类锚判等 ⟹ 成功（P1 重议 + #218）
                ptb(130, buy1_bits(), ctr(0, 1, B_START + 5)), // 一类带不等 ⟹ 实不等
                pt(140, sell1_bits()),     // 异向不入点级
            ],
            vec![],
            vec![ctr(0, 0, B_START)],
        );
        let ev = trend_ev((100, 200), 200);
        let index = i218_build(&c, vec![vec![], vec![ev]], &oracle, &event_anchor);
        let inst = index.instrument();
        assert_eq!(inst.trend_success, 1);
        assert_eq!(
            inst.trend_owner_anchor_neq
                + inst.trend_out_of_window
                + inst.trend_opposite_side
                + inst.trend_no_valid_point,
            0,
            "成功事件不计桶"
        );
        assert_eq!(inst.trend_pts[1], [1, 0], "二类 owner 判等 1（锚判同）");
        assert_eq!(inst.trend_pts[0], [0, 1], "一类 owner 不等 1（带不等）");
        assert_eq!(inst.owner_real_neq_pts, 1);
        assert_eq!(index.stats().assembled, 1);
        assert_eq!(index.stats().rungs_0, 1);
        assert!(index.get(&NestEventIdentity::of(&ev)).is_some());
    }

    /// 完备性平账（replay 校验共用断言）：Trend 事件总数 = 成功 + 四桶合计；
    /// 按级 × kind 两维分解（ID-4，钉口径②）与既有六字段对账一致。
    #[test]
    fn i214_completeness_reconciliation_and_level_kind_decomp() {
        // #218 面 B（机械改写归因：带等值/带不等构造）。
        let c = book2c(
            vec![
                ptb(120, buy1_bits(), ctr(0, 0, B_START + 5)), // ev_s 成功（带判等，序号漂移）
                ptb(320, buy1_bits(), ctr(0, 1, B_START + 9)), // ev_o owner 实不等（带不等）
                pt(720, sell1_bits()),                         // ev_x 异向
            ],
            vec![], // ev_n（level2）空账 ⟹ 无合法点（book_empty）
            vec![ctr(0, 0, B_START)],
        );
        let ev_s = trend_ev((100, 200), 200);
        let ev_o = trend_ev((300, 400), 400);
        let ev_w = trend_ev((500, 600), 600); // 窗内无点、账本窗外有点 ⟹ 窗口外
        let ev_x = trend_ev((700, 800), 800);
        let ev_n = NestCandidateEvent {
            b_center_start: B_START,
            ..typed_event(
                2,
                Side::Short,
                NestDivergenceKind::Trend,
                (100, 200),
                (90, 210),
                200,
                200,
                true,
            )
        };
        let ev_m = NestCandidateEvent {
            b_center_start: B_START,
            ..typed_event(
                3,
                Side::Short,
                NestDivergenceKind::Trend,
                (100, 200),
                (90, 210),
                200,
                200,
                true,
            )
        }; // level3 ⟹ 账本越界（book_missing）
        let index = i214_build(
            &c,
            vec![vec![], vec![ev_s, ev_o, ev_w, ev_x], vec![ev_n], vec![ev_m]],
        );
        let inst = index.instrument();
        assert_eq!(inst.trend_success, 1);
        assert_eq!(inst.trend_owner_anchor_neq, 1);
        assert_eq!(inst.trend_out_of_window, 1);
        assert_eq!(inst.trend_opposite_side, 1);
        assert_eq!(inst.trend_no_valid_point, 2);
        assert_eq!(inst.nvp_book_empty + inst.nvp_book_missing, 2);
        let trend_total = inst.trend_total();
        assert_eq!(trend_total, 6, "Trend 域事件总数 = 成功 + 四桶合计");
        assert_eq!(
            inst.base_trend(),
            6,
            "kind 分解 Trend 计数 = 事件级平账总数"
        );
        assert_eq!(inst.base_consolidation(), 0);
        // 按级 × kind 两维分解（行 = 级别槽，槽 0 恒零；列 [Trend, Consolidation]）
        assert_eq!(
            inst.base_by_level_kind,
            vec![[0, 0], [4, 0], [1, 0], [1, 0]]
        );
        assert_eq!(
            inst.assembled_by_level_kind,
            vec![[0, 0], [1, 0], [0, 0], [0, 0]]
        );
        assert_eq!(
            inst.indexed_by_level_kind,
            vec![[0, 0], [1, 0], [0, 0], [0, 0]]
        );
        // 既有六字段语义不动（与装置计数对账）
        let st = index.stats();
        assert_eq!((st.base_events, st.assembled, st.indexed), (6, 1, 1));
        assert_eq!((st.rungs_0, st.rungs_1, st.rungs_2_plus), (1, 0, 0));
        assert_eq!(st.base_events, trend_total + inst.base_consolidation());
    }

    /// ID-3 点级二维：窗口内同向点按（一/二/三类 × owner 判等/不等）计数；一位多类点
    /// 按所属类逐行各计；异向点/窗外点不入计。wf7 相等率读数 = c2/c3 行 eq ÷ (eq+ne)。
    /// #218 面 A+B（机械改写归因：序号等值 → 带/锚等值构造；「身份缺失」重定 = 锚不可解）。
    #[test]
    fn i214_point_level_two_dim_all_classes() {
        static ORACLE: &[(usize, (Tick, usize))] = &[(30, (900, 70))];
        let oracle = synth_oracle(ORACLE);
        let event_anchor = |_: &NestCandidateEvent| (Some(901), Some(70)); // 与点侧锚实不等
        let mut buy3 = BspBits::default();
        buy3.buy3 = true;
        let mut buy12 = buy1_bits();
        buy12.buy2 = true; // 一位两类（合成形状：一、二类行各计一次）
        let c = book2c(
            vec![
                ptc(50, buy1_bits(), B_START + 9), // 窗外 ⟹ 不计
                ptc(110, buy1_bits(), B_START),    // c1 带判等
                pta(120, buy2_bits(), 30),         // c2 锚实不等（点侧 900 vs 事件侧 901）
                ptc(130, buy3, B_START),           // c3 带判等
                pt(140, buy2_bits()),              // c2 锚不可解（无载体）
                ptc(150, buy12, B_START),          // c1 带判等 + c2 带判等（多位点逐行各计）
                pt(160, sell1_bits()),             // 异向 ⟹ 不计
            ],
            vec![],
            vec![ctr(0, 0, B_START)],
        );
        let ev = trend_ev((100, 200), 200);
        let index = i218_build(&c, vec![vec![], vec![ev]], &oracle, &event_anchor);
        let inst = index.instrument();
        assert_eq!(
            inst.trend_pts[0],
            [2, 0],
            "一类：eq 2（@110 + 多位点 @150）"
        );
        assert_eq!(
            inst.trend_pts[1],
            [1, 2],
            "二类：eq 1（@150 带判等）；ne 2（@120 锚实不等 + @140 锚不可解）"
        );
        assert_eq!(inst.trend_pts[2], [1, 0], "三类：eq 1（@130）");
        assert_eq!(inst.owner_real_neq_pts, 1);
        assert_eq!(inst.owner_anchor_missing_pts, 1);
        assert_eq!(
            inst.trend_success, 1,
            "owner 判等点在场 ⟹ 命中（最早合法 @110）"
        );
    }

    /// ID-4 按级 × kind 分解 + Pan 域只计成功/失败总数（钉口径⑥：四桶不适用 Pan）。
    #[test]
    fn i214_level_kind_decomposition_and_pan_totals() {
        // #218 面 B（机械改写归因：带判等构造）。
        let c = book2c(
            vec![
                ptc(110, buy1_bits(), B_START), // ev_a Trend 成功（带判等）
                pt(310, sell1_bits()),          // ev_b Trend 异向
                pt(510, buy1_bits()),           // ev_c Pan 成功（center=None 合法，P2）
            ],
            vec![pt(110, buy2_bits())], // ev_e Pan 成功（levels[1] 账）
            vec![ctr(0, 0, B_START)],
        );
        let ev_a = trend_ev((100, 200), 200);
        let ev_b = trend_ev((300, 400), 400);
        let ev_c = pan_ev((500, 600), 600);
        let ev_d = NestCandidateEvent {
            // level2 Trend：窗 [700,800] 内无点、账本窗外有点 ⟹ 窗口外桶
            b_center_start: B_START,
            ..typed_event(
                2,
                Side::Long,
                NestDivergenceKind::Trend,
                (700, 800),
                (690, 810),
                800,
                800,
                true,
            )
        };
        let ev_e = typed_event(
            2,
            Side::Long,
            NestDivergenceKind::Consolidation,
            (100, 200),
            (90, 210),
            200,
            200,
            true,
        );
        let index = i214_build(&c, vec![vec![], vec![ev_a, ev_b, ev_c], vec![ev_d, ev_e]]);
        let inst = index.instrument();
        assert_eq!(inst.base_by_level_kind, vec![[0, 0], [2, 1], [1, 1]]);
        assert_eq!(inst.assembled_by_level_kind, vec![[0, 0], [1, 1], [0, 1]]);
        assert_eq!(inst.indexed_by_level_kind, inst.assembled_by_level_kind);
        assert_eq!(inst.base_trend(), 3);
        assert_eq!(inst.base_consolidation(), 2);
        assert_eq!(inst.pan_success, 2);
        assert_eq!(inst.pan_fail, 0);
        assert_eq!(inst.trend_success, 1);
        assert_eq!(inst.trend_opposite_side, 1);
        assert_eq!(inst.trend_out_of_window, 1);
        assert_eq!(
            inst.trend_pts,
            [[1, 0], [0, 0], [0, 0]],
            "点级二维只计 Trend 事件窗内同向点（Pan 域零计数）"
        );
    }

    /// Pan 域失败只计总数（钉口径⑥）：异向/窗空失败不落入 Trend 四桶，点级二维零计数。
    #[test]
    fn i214_pan_failure_totals_only() {
        let c = book2(
            vec![
                pt(150, buy1_bits()),  // ev_ok 成功
                pt(550, sell1_bits()), // ev_opp 异向
            ],
            vec![],
        );
        let ev_ok = pan_ev((100, 200), 200);
        let ev_empty = pan_ev((300, 400), 400); // 窗内无点（账本窗外有点）——Pan 无「窗口外」桶
        let ev_opp = pan_ev((500, 600), 600);
        let index = i214_build(&c, vec![vec![], vec![ev_ok, ev_empty, ev_opp]]);
        let inst = index.instrument();
        assert_eq!(inst.pan_success, 1);
        assert_eq!(inst.pan_fail, 2, "Pan 域失败只计总数，不分桶");
        assert_eq!(
            inst.trend_success
                + inst.trend_owner_anchor_neq
                + inst.trend_out_of_window
                + inst.trend_opposite_side
                + inst.trend_no_valid_point,
            0
        );
        assert_eq!(inst.trend_pts, [[0, 0]; 3]);
        assert_eq!(index.stats().assembled, 1, "仅 ev_ok 装配（纯基例）");
    }

    /// 零干预（ID-6/US-06）：索引内容不受装置影响——同一输入经主接缝产出的证书与
    /// 既有生产路径（`assemble_typed_certificate` + `terminal_bits_at_event` 闭包）
    /// 逐字段相等；既有六字段/rungs 构成语义不动。
    #[test]
    fn i214_index_invariance_vs_production_assembly() {
        // #218 面 B（机械改写归因：带判等构造；同一 ctx 双路对照）。
        let c = book2c(
            vec![ptc(120, buy1_bits(), B_START)],
            vec![],
            vec![ctr(0, 0, B_START)],
        );
        let base = trend_ev((100, 200), 200);
        let parent = NestCandidateEvent {
            b_center_start: B_START,
            ..typed_event(
                2,
                Side::Long,
                NestDivergenceKind::Trend,
                (50, 300),
                (40, 310),
                300,
                300,
                true,
            )
        };
        let evs = vec![vec![], vec![base], vec![parent]];
        let index = i214_build(&c, evs.clone());
        // 生产路径直装（装置合入前的唯一查法形状）
        let direct = assemble_typed_certificate(&evs, &base, 2, NestIntervalCaliber::B, &|e| {
            terminal_bits_at_event(&c, e, TerminalMatch::CWindow, &ctx_degenerate()).map(|t| t.bits)
        })
        .expect("父区间包含 ⟹ 链深 1 证书");
        let via_index = index
            .get(&NestEventIdentity::of(&base))
            .expect("索引内含基例证书");
        assert_eq!(
            via_index, &direct,
            "索引证书与生产直装逐字段相等（装置零干预）"
        );
        let st = index.stats();
        assert_eq!((st.base_events, st.assembled, st.indexed), (2, 1, 1));
        assert_eq!((st.rungs_0, st.rungs_1, st.rungs_2_plus), (0, 1, 0));
        assert_eq!(st.single_level_share(), Some(0.0));
        // 装置读数：base 成功；parent（level2）读 levels[1] 空账 ⟹ 无合法点（book_empty）
        let inst = index.instrument();
        assert_eq!(inst.trend_success, 1);
        assert_eq!(inst.trend_no_valid_point, 1);
        assert_eq!(inst.nvp_book_empty, 1);
    }

    /// 共核一致性（ID-1C）：测量入口判定分量与生产入口逐值相等（同核双薄入口，
    /// 无第二查法——测量口径与生产口径结构性不可漂移，US-05）。
    #[test]
    fn i214_measured_entry_judgment_matches_production() {
        // #218 面 B（机械改写归因：带等值/带不等构造；同一 ctx 喂双入口）。
        let c = book2c(
            vec![
                pt(99, buy1_bits()),                           // 窗外
                ptc(120, buy2_bits(), B_START),                // 二类带判等（CWindow 命中点）
                ptb(150, buy1_bits(), ctr(0, 1, B_START + 9)), // 带不等 ⟹ owner 实不等
                pt(160, sell1_bits()),                         // 异向
                pt(201, buy1_bits()),                          // 窗外
            ],
            vec![],
            vec![ctr(0, 0, B_START)],
        );
        for ev in [trend_ev((100, 200), 200), pan_ev((100, 200), 200)] {
            for m in [TerminalMatch::CWindow, TerminalMatch::Exact] {
                assert_eq!(
                    terminal_bits_at_event_measured(&c, &ev, m, &ctx_degenerate()).0,
                    terminal_bits_at_event(&c, &ev, m, &ctx_degenerate()),
                    "双入口判定分量逐值相等（共核）"
                );
            }
        }
    }

    /// 测量入口契约（判定层自决补测，不进验收接缝清单）：Exact 诊断臂不产计数
    /// （探针零值）；账本缺失经测量入口以 `book_missing` 标记带出；生产薄包装语义不变。
    #[test]
    fn i214_measured_entry_exact_probe_empty_and_book_missing() {
        let c = book2(vec![pt(100, buy1_bits())], vec![]);
        let ev = trend_ev((90, 100), 100);
        let (hit, probe) =
            terminal_bits_at_event_measured(&c, &ev, TerminalMatch::Exact, &ctx_degenerate());
        assert_eq!(
            hit.map(|t| t.bits),
            Some(buy1_bits()),
            "Exact 臂判定与生产入口一致"
        );
        assert_eq!(probe, EndorsementProbe::default(), "Exact 诊断臂不产计数");
        // 生产薄包装丢弃探针、语义不变
        assert_eq!(
            terminal_bits_at_event(&c, &ev, TerminalMatch::Exact, &ctx_degenerate())
                .map(|t| t.bits),
            Some(buy1_bits())
        );
        // 账本级别越界：测量入口以 book_missing 带出；生产入口仍 None（语义不动）
        let ev_oob = NestCandidateEvent { level: 5, ..ev };
        let (miss, probe_oob) =
            terminal_bits_at_event_measured(&c, &ev_oob, TerminalMatch::CWindow, &ctx_degenerate());
        assert_eq!(miss, None);
        assert!(probe_oob.book_missing, "级别越界 ⟹ book_missing 子计数标记");
        assert_eq!(
            terminal_bits_at_event(&c, &ev_oob, TerminalMatch::CWindow, &ctx_degenerate()),
            None
        );
    }

    // ───────── #218 面 B（spec owner-attribution-fix-20260724 ID-2/ID-4；#211 路线 B 终裁）：
    // owner 判同从 start_index 序号判同换两族精确等值——一/三类：核心区间 (zd,zg) 带判同
    //（事件侧 B 由 b_center_start 当查找键在账本 centers 查出，序号只当键不当身份）；
    // 二类：anchor_at(点侧一类点坐标) == (event.extreme_price, event.group_anchor)。
    // 主接缝 = `build_nest_certificate_index`（spec Testing Decisions 接缝清单 1）。─────────

    use super::super::super::types::Tick;
    use super::super::bsp::OwnerRef;

    /// 中枢夹具（zd/zg/start_index 显式给；dd/gg/end 判同不入——带判同只看核心区间）。
    fn ctr(zd: i64, zg: i64, start_index: usize) -> Center {
        Center {
            zd,
            zg,
            dd: 0,
            gg: 0,
            start_index,
            end_index: 0,
        }
    }

    /// 携中枢参照载体的账本点夹具（一/三类；#218 面 A 载体形态）。
    fn ptb(source_index: usize, bits: BspBits, center: Center) -> BspPoint {
        BspPoint {
            center: Some(OwnerRef::Center(center)),
            ..pt(source_index, bits)
        }
    }

    /// 携一类点锚载体的账本点夹具（二类；anchor_src = 该走势一类点坐标）。
    fn pta(source_index: usize, bits: BspBits, anchor_src: usize) -> BspPoint {
        BspPoint {
            center: Some(OwnerRef::Type1Anchor(anchor_src)),
            ..pt(source_index, bits)
        }
    }

    /// 带 centers 的两级分类夹具（面 B：一/三类判同需账本 centers 查 B 带；B 只当查找键）。
    fn book2c(l0: Vec<BspPoint>, l1: Vec<BspPoint>, centers0: Vec<Center>) -> Classification {
        Classification {
            levels: vec![
                LevelState {
                    bsp: std::rc::Rc::new(l0),
                    centers: std::rc::Rc::new(centers0),
                    ..Default::default()
                },
                LevelState {
                    bsp: std::rc::Rc::new(l1),
                    ..Default::default()
                },
            ],
        }
    }

    /// 恒缺锚 oracle（Pan 域/Exact 臂/非 owner 用例不消费锚——参照包置空合法）。
    fn never_anchor(_: usize) -> Option<(Tick, usize)> {
        None
    }

    /// 合成 oracle：坐标 → 两元锚（锚供给缺失坐标 = None ⟹ 锚不可解）。
    fn synth_oracle(
        entries: &'static [(usize, (Tick, usize))],
    ) -> impl Fn(usize) -> Option<(Tick, usize)> {
        move |x| entries.iter().find(|(s, _)| *s == x).map(|(_, a)| *a)
    }

    /// Pan/Exact/非 owner 用例参照包（锚判同不入判，供给置空）。
    fn ctx_degenerate() -> OwnerAnchorCtx<'static> {
        OwnerAnchorCtx {
            anchor_at: &never_anchor,
            event_anchor: (None, None),
        }
    }

    /// #218 主接缝薄封装：caliber 固定生产口径 B（同 i214_build 先例），锚供给显式入参。
    fn i218_build(
        c: &Classification,
        evs: Vec<Vec<NestCandidateEvent>>,
        anchor_at: &dyn Fn(usize) -> Option<(Tick, usize)>,
        event_anchor_of: &dyn Fn(&NestCandidateEvent) -> (Option<Tick>, Option<usize>),
    ) -> super::super::nest_index::NestCertificateIndex {
        super::super::nest_index::build_nest_certificate_index(
            c,
            &evs,
            NestIntervalCaliber::B,
            anchor_at,
            event_anchor_of,
        )
    }

    /// 恒缺事件锚（二类判同事件侧供给缺失用例）。
    fn no_event_anchor(_: &NestCandidateEvent) -> (Option<Tick>, Option<usize>) {
        (None, None)
    }

    /// 序号漂移用例（spec 接缝清单 1 钉案）：一/三类载体与事件 B 的 `start_index` **不同**
    /// 但 (zd,zg) 相同 ⟹ 带判同判等（旧序号判同必判负——换锚的直接行为证据）；
    /// 带等且序号不等 = 同带不同中枢（带碰撞可疑）⟹ `band_eq_start_neq_pts` 观察计数（US-08）。
    #[test]
    fn i218_band_eq_despite_start_index_drift() {
        let b = ctr(100, 200, B_START);
        let c = book2c(
            vec![ptb(150, buy1_bits(), ctr(100, 200, B_START + 77))], // 同带、异序号
            vec![],
            vec![b],
        );
        let ev = trend_ev((100, 200), 200);
        let index = i218_build(&c, vec![vec![], vec![ev]], &never_anchor, &no_event_anchor);
        let inst = index.instrument();
        assert_eq!(
            inst.trend_success, 1,
            "带判同：序号漂移不杀背书（(zd,zg) 同 ⟹ owner 判等）"
        );
        assert_eq!(inst.trend_pts[0], [1, 0], "一类点带判同等值 1");
        assert_eq!(
            inst.band_eq_start_neq_pts, 1,
            "带等但序号不等 = 带碰撞观察读数（US-08）"
        );
        assert_eq!(index.stats().assembled, 1);
        assert!(index.get(&NestEventIdentity::of(&ev)).is_some());
    }

    /// 反向漂移用例：载体与事件 B 同 `start_index` 但 (zd,zg) 不同 ⟹ 带判同判负
    ///（旧序号判同必判等——「延伸改带不改序号」序号判同判不出，带判同判得出）。
    #[test]
    fn i218_band_neq_despite_same_start_index() {
        let b = ctr(100, 200, B_START);
        let c = book2c(
            vec![ptb(150, buy1_bits(), ctr(100, 201, B_START))], // 同序号、异带
            vec![],
            vec![b],
        );
        let ev = trend_ev((100, 200), 200);
        let index = i218_build(&c, vec![vec![], vec![ev]], &never_anchor, &no_event_anchor);
        let inst = index.instrument();
        assert_eq!(
            inst.trend_owner_anchor_neq, 1,
            "带不等 ⟹ owner 锚不等桶（序号判同会误判等）"
        );
        assert_eq!(inst.trend_success, 0);
        assert_eq!(inst.owner_real_neq_pts, 1, "两侧皆可判且实不等（带不等）");
        assert_eq!(inst.trend_pts[0], [0, 1]);
        assert_eq!(index.stats().assembled, 0);
    }

    /// 二类判同判等：`anchor_at(点侧一类点坐标) == (event.extreme_price, event.group_anchor)`
    ///（身份锚 =（极值价, 合并组锚），事件自带 seg_c.1 两元锚，零新增解析）。
    #[test]
    fn i218_type2_anchor_eq_endorses() {
        static ORACLE: &[(usize, (Tick, usize))] = &[(30, (900, 70))];
        let oracle = synth_oracle(ORACLE);
        let event_anchor = |_: &NestCandidateEvent| (Some(900), Some(70));
        let c = book2c(
            vec![pta(150, buy2_bits(), 30)],
            vec![],
            vec![ctr(100, 200, B_START)],
        );
        let ev = trend_ev((100, 200), 200);
        let index = i218_build(&c, vec![vec![], vec![ev]], &oracle, &event_anchor);
        let inst = index.instrument();
        assert_eq!(
            inst.trend_success, 1,
            "二类点一类锚 == 事件 extreme 锚 ⟹ 判等背书"
        );
        assert_eq!(inst.trend_pts[1], [1, 0], "二类点锚判同等值 1");
        assert_eq!(inst.owner_anchor_missing_pts, 0);
        assert_eq!(index.stats().assembled, 1);
    }

    /// 二类判同判负：两侧锚皆可解但实不等 ⟹ owner 锚不等桶 + real_neq 子计数。
    #[test]
    fn i218_type2_anchor_neq_rejected() {
        static ORACLE: &[(usize, (Tick, usize))] = &[(30, (900, 70))];
        let oracle = synth_oracle(ORACLE);
        let event_anchor = |_: &NestCandidateEvent| (Some(901), Some(70)); // 极值价差 1 tick
        let c = book2c(
            vec![pta(150, buy2_bits(), 30)],
            vec![],
            vec![ctr(100, 200, B_START)],
        );
        let ev = trend_ev((100, 200), 200);
        let index = i218_build(&c, vec![vec![], vec![ev]], &oracle, &event_anchor);
        let inst = index.instrument();
        assert_eq!(inst.trend_owner_anchor_neq, 1);
        assert_eq!(
            inst.owner_real_neq_pts, 1,
            "锚实不等（精确等值无容差，v3 硬禁令）"
        );
        assert_eq!(inst.owner_anchor_missing_pts, 0);
        assert_eq!(inst.trend_success, 0);
        assert_eq!(inst.trend_pts[1], [0, 1]);
    }

    /// 二类锚不可解子计数（spec ID-2/ID-4）：(a) 点侧锚供给缺失（oracle 未命中坐标）；
    /// (b) 事件侧锚供给缺失（事件锚 None）——身份合取不可证 = 诚实判负 + 子计数单列。
    #[test]
    fn i218_type2_anchor_unresolvable_subcount() {
        // (a) 点侧缺失：oracle 不覆盖坐标 30。
        let event_anchor = |_: &NestCandidateEvent| (Some(900), Some(70));
        let c = book2c(
            vec![pta(150, buy2_bits(), 30)],
            vec![],
            vec![ctr(100, 200, B_START)],
        );
        let ev = trend_ev((100, 200), 200);
        let index = i218_build(&c, vec![vec![], vec![ev]], &never_anchor, &event_anchor);
        let inst = index.instrument();
        assert_eq!(
            inst.trend_owner_anchor_neq, 1,
            "锚不可解 ⟹ 诚实判负入 owner 锚不等桶"
        );
        assert_eq!(
            inst.owner_anchor_missing_pts, 1,
            "点侧锚供给缺失 ⟹ 锚不可解子计数"
        );
        assert_eq!(inst.owner_real_neq_pts, 0);
        assert_eq!(
            inst.trend_pts[1],
            [0, 1],
            "不可解归 owner 不等列（与实不等同列、子计数分列）"
        );
        // (b) 事件侧缺失：oracle 可解但事件锚 None。
        static ORACLE: &[(usize, (Tick, usize))] = &[(30, (900, 70))];
        let oracle = synth_oracle(ORACLE);
        let index_b = i218_build(&c, vec![vec![], vec![ev]], &oracle, &no_event_anchor);
        let inst_b = index_b.instrument();
        assert_eq!(inst_b.trend_owner_anchor_neq, 1);
        assert_eq!(
            inst_b.owner_anchor_missing_pts, 1,
            "事件侧锚供给缺失同归锚不可解"
        );
        assert_eq!(inst_b.owner_real_neq_pts, 0);
    }

    /// 中枢参照判同的「不可解」防御臂：事件 `b_center_start` 在账本 centers 查无
    ///（生产不可达——B 快照与账本同塔同源，合成防御）⟹ 锚不可解 = 诚实判负 + 子计数。
    #[test]
    fn i218_band_b_lookup_missing_is_unresolvable() {
        let c = book2c(
            vec![ptb(150, buy1_bits(), ctr(100, 200, B_START))],
            vec![],
            vec![ctr(100, 200, B_START + 999)], // 账本 centers 无 b_center_start=20 的对象
        );
        let ev = trend_ev((100, 200), 200);
        let index = i218_build(&c, vec![vec![], vec![ev]], &never_anchor, &no_event_anchor);
        let inst = index.instrument();
        assert_eq!(inst.trend_owner_anchor_neq, 1);
        assert_eq!(
            inst.owner_anchor_missing_pts, 1,
            "B 查找失败 = 身份合取不可证 ⟹ 锚不可解"
        );
        assert_eq!(inst.owner_real_neq_pts, 0);
        assert_eq!(inst.trend_success, 0);
    }

    /// 四桶新口径完备性平账 + 点级（类 × 判同）计数（spec 接缝清单 1）：一/二/三类混合
    /// 场景，成功 + 四桶 = Trend 事件总数；点级计数含成功事件窗内同向点。
    #[test]
    fn i218_four_bucket_completeness_new_caliber() {
        static ORACLE: &[(usize, (Tick, usize))] = &[(30, (900, 70))];
        let oracle = synth_oracle(ORACLE);
        let event_anchor = |_: &NestCandidateEvent| (Some(900), Some(70));
        let mut buy3 = BspBits::default();
        buy3.buy3 = true;
        let c = book2c(
            vec![
                ptb(120, buy1_bits(), ctr(100, 200, B_START + 5)), // 一类带等（序号漂移）⟹ ev_s 成功
                pta(320, buy2_bits(), 30),                         // 二类锚等 ⟹ ev_s2 成功
                ptb(340, buy3, ctr(100, 201, B_START)),            // 三类带不等 ⟹ ev_o 桶
                pt(720, sell1_bits()),                             // 异向 ⟹ ev_x 桶
            ],
            vec![],
            vec![ctr(100, 200, B_START)],
        );
        let ev_s = trend_ev((100, 200), 200);
        let ev_s2 = trend_ev((300, 330), 330);
        let ev_o = trend_ev((330, 400), 400);
        let ev_w = trend_ev((500, 600), 600); // 窗内无点、账本窗外有点 ⟹ 窗口外
        let ev_x = trend_ev((700, 800), 800);
        let index = i218_build(
            &c,
            vec![vec![], vec![ev_s, ev_s2, ev_o, ev_w, ev_x]],
            &oracle,
            &event_anchor,
        );
        let inst = index.instrument();
        assert_eq!(inst.trend_success, 2, "一类带等 + 二类锚等各成功一件");
        assert_eq!(inst.trend_owner_anchor_neq, 1, "三类带不等一桶");
        assert_eq!(inst.trend_out_of_window, 1);
        assert_eq!(inst.trend_opposite_side, 1);
        assert_eq!(inst.trend_no_valid_point, 0);
        assert_eq!(
            inst.trend_total(),
            5,
            "Trend 总数 = 成功 + 四桶（完备性平账）"
        );
        assert_eq!(inst.base_trend(), 5);
        // 点级计数：ev_s 窗内一类带等 1；ev_s2 窗内二类锚等 1；ev_o 窗内三类带不等 1。
        assert_eq!(inst.trend_pts[0], [1, 0]);
        assert_eq!(inst.trend_pts[1], [1, 0]);
        assert_eq!(inst.trend_pts[2], [0, 1]);
        assert_eq!(inst.owner_real_neq_pts, 1);
        assert_eq!(
            inst.band_eq_start_neq_pts, 1,
            "一类点带等同带异序号 ⟹ 带碰撞 1"
        );
        // 遍历计数（不变量断言集读数）：账本 4 点；各事件窗内/同向逐窗各计。
        assert_eq!(
            inst.scan_book_total,
            4 * 5,
            "5 事件 × 账本 4 点（每事件一次全账遍历）"
        );
        assert_eq!(inst.scan_in_window, 1 + 1 + 1 + 0 + 1);
        assert_eq!(inst.scan_in_window_same_side, 1 + 1 + 1 + 0 + 0);
        assert_eq!(index.stats().assembled, 2);
    }

    /// 不变量断言（spec ID-5）：异向桶/窗口外桶/Pan 域计数与旧口径逐值一致——非 owner
    /// 路径（窗口/方向/Pan 谓词）一个 bit 不动。
    #[test]
    fn i218_non_owner_invariants_bit_identical() {
        let c = book2c(
            vec![
                pt(99, sell1_bits()), // 窗左外（异向亦计「账本窗外有点」，宽口径）
                ptb(250, buy1_bits(), ctr(100, 200, B_START)), // 窗右外同向
                pt(150, sell1_bits()), // 异向
                pt(160, BspBits::default()), // 零 bit
            ],
            vec![],
            vec![ctr(100, 200, B_START)],
        );
        let ev_w = trend_ev((100, 200), 200); // 窗内有 2 点全异向/零 bit？——@150/@160 在窗内
        let index = i218_build(
            &c,
            vec![vec![], vec![ev_w]],
            &never_anchor,
            &no_event_anchor,
        );
        let inst = index.instrument();
        assert_eq!(
            inst.trend_opposite_side, 1,
            "窗内有点但同向 0 ⟹ 异向桶（方向谓词不动）"
        );
        assert_eq!(inst.scan_in_window, 2);
        assert_eq!(inst.scan_in_window_same_side, 0);
        // Pan 域：同向一/二/三类全合法（confirm_side 即精确语义，owner 不入判）——
        // center=None 合法，与旧口径逐值一致。
        let c2 = book2(vec![pt(150, buy2_bits())], vec![]);
        let ev_pan = pan_ev((100, 200), 200);
        let index_pan = i218_build(
            &c2,
            vec![vec![], vec![ev_pan]],
            &never_anchor,
            &no_event_anchor,
        );
        let inst_pan = index_pan.instrument();
        assert_eq!(inst_pan.pan_success, 1);
        assert_eq!(inst_pan.pan_fail, 0);
        assert_eq!(
            inst_pan.trend_pts,
            [[0, 0]; 3],
            "Pan 域点级零计数（钉口径⑥ 不动）"
        );
    }

    /// Exact 诊断臂不消费锚参照包（C-a 常数不动）：新签名下判定与探针零值照旧。
    #[test]
    fn i218_exact_arm_untouched_by_anchor_ctx() {
        let c = book2(vec![pt(100, buy1_bits())], vec![]);
        let ev = trend_ev((90, 100), 100);
        let (hit, probe) =
            terminal_bits_at_event_measured(&c, &ev, TerminalMatch::Exact, &ctx_degenerate());
        assert_eq!(hit.map(|t| t.bits), Some(buy1_bits()));
        assert_eq!(
            probe,
            EndorsementProbe::default(),
            "Exact 臂不产计数（探针零值照旧）"
        );
    }

    /// fail-closed 合一（codex 评审 2026-07-24 采纳）：`b_center_start = None`（旧事件
    /// 路径 B 身份缺失）⟹ 两族判同一律锚不可解判负——中枢判同族（B 查找无键）与点判
    /// 同族（锚可解亦然）都不得判等（「身份合取不可证 ⟹ 诚实判负」纪律合一）。
    #[test]
    fn i218_b_identity_missing_fails_closed_both_families() {
        static ORACLE: &[(usize, (Tick, usize))] = &[(30, (900, 70))];
        let oracle = synth_oracle(ORACLE);
        let ctx = OwnerAnchorCtx {
            anchor_at: &oracle,
            event_anchor: (Some(900), Some(70)),
        };
        let book = [
            ptb(150, buy1_bits(), ctr(100, 200, B_START)),
            pta(151, buy2_bits(), 30),
        ];
        let centers = [ctr(100, 200, B_START)];
        for (i, tag) in [(0usize, "一类（中枢判同族）"), (1, "二类（点判同族）")] {
            let hit = terminal_bits_in_book(
                &book[i..i + 1],
                &centers,
                100,
                200,
                Side::Long,
                NestDivergenceKind::Trend,
                None,
                TerminalMatch::CWindow,
                &ctx,
            );
            assert_eq!(
                hit, None,
                "b_center_start=None ⟹ {tag} 亦诚实判负（fail-closed 合一）"
            );
            let (_h, probe) = terminal_bits_in_book_core(
                &book[i..i + 1],
                &centers,
                100,
                200,
                Side::Long,
                NestDivergenceKind::Trend,
                None,
                TerminalMatch::CWindow,
                &ctx,
            );
            assert_eq!(probe.owner_anchor_missing_pts, 1, "{tag} 归锚不可解子计数");
            assert_eq!(probe.in_window_owner_eq, 0);
        }
    }
}
