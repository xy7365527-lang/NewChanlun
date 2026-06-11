//! 配置与变体表 — 有机赋格 v2 消融轴（v2 §8.1）。
//!
//! 与 Python v1 `OrganicConfig` 的轴差异（v2 矛盾修正的配置面）：
//!   - `master_seg_end` 删除（C1：master 出场 = sell1 类型隔离，无配置可重开三触发）
//!   - `sub_anchor` 新增（C3 G1：REV 开腿次级别方向锚定；消融轴 S）
//!   - `tranche` 新增（C4：递归建仓 vs 一次性满 frac_k——V1f/V1r 隔离轴）
//!   - `sell2_trigger` 新增（C6 §5.3 矩阵 Sell2 格的显式表态；消融轴 R2，默认关）
//!   - `rev_gate` 语义升级为点态门（C7；v1 积累集语义无对应配置位——被整体替换）

use crate::buysellpoint::BspKind;

/// 市场语境。futures（INV-3 真实空头）不在枚举里——F1 实装时新增变体，
/// 编译器强制所有 match 点表态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketMode {
    Stock,
}

/// 止损模式。A/B 当前同语义（2% 核心止损），保留区分位（Python parity）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopMode {
    None,
    A,
    B,
}

impl StopMode {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "none" => Some(StopMode::None),
            "A" => Some(StopMode::A),
            "B" => Some(StopMode::B),
            _ => None,
        }
    }

    pub fn is_on(self) -> bool {
        matches!(self, StopMode::A | StopMode::B)
    }
}

/// REV 关腿消融轴 R（type1 买分量的确认强度）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevClose {
    /// confirmed type1 买（T5 原样）。
    Conf,
    /// type1 买放宽为 candidate。
    Cand,
    /// candidate type1 买 ∧ 次级别买点（27课区间套）。
    Nested,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sizing {
    Equal,
    Structure,
}

/// 深度门 θ 的取值方式（2026-06-11 θ 自适应任务）。
///
/// 严格性裁定（三方案对比，analysis/theta_adaptive_results.md §2）：
///   方案A（锚自身振幅的分位数）= 自指退化（单个数无分位数）；
///   方案C（标的级全样本固定分位数）= 回测期内前瞻（用未来中枢定过去门槛）；
///   方案B（因果滚动分位数，本枚举 AdaptiveQuantile）= 最严格——零前瞻、
///   零标的级拟合，q/window/min_obs 为预注册变体常数。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThetaMode {
    /// θ ≡ cfg.theta_depth（在册基线语义，V2f/V2of/V2r 不变）。
    Fixed,
    /// θ_t(k) = 该层最近 window 个中枢相对振幅的 q 分位（nearest-rank，
    /// 排除当前锚自身）；样本 < min_obs 回退 cfg.theta_depth。
    AdaptiveQuantile { q: f64, window: usize, min_obs: usize },
}

/// REV 声部循环模式（38课循环 voice 实装，2026-06-11）。
///
/// Single = 在册行为：一次 REV 腿（开一条→闭一条→回 RIDE，腿级配对谓词）。
/// Cycle38 = 38课循环：宿主（ladder+1）趋势存续期间，voice@k 反复
/// "本级别卖点→卖出（短差开）→ 次级别（k−1）买点→买回（短差闭）"，
/// 循环终止 = 宿主趋势态结束（第38课"这个过程可以不断延续下去，直到……
/// 不创新高或者盘整背驰为止"——趋势态行翻落即终止信号的工程读数）。
/// 循环存续条件用趋势态（kind==Trend ∧ direction==Up），不用中枢域；
/// 进出信号用买卖点（BSP/盘背事件流），不用分型（任务硬约束）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevCycle {
    Single,
    Cycle38,
}

/// Cycle38 循环腿的买回（闭腿）判据消融轴（2026-06-11 任务）。
///
/// 编排者原则：**区间套无论正反都存在**——卖出端用区间套（大级别定方向 +
/// 本级别卖点定时机 + 次级别确认精度），买回端也必须先有本级别判据
/// （同锚 Buy1 / ZD 触线）再谈次级别精度。SubAny（在册 C38base）跳过了
/// 本级别判据直接用次级别任意买点，违反区间套——已否证（OKLO −472.8pp /
/// BRN −70.4pp，死因 = 买回级别错配，胜率 33-36%）。
///
/// 数据依赖：Buy1/Zd/Paired 的比较基准是开腿时的锚中枢快照（同锚 cs / ZD 线）
/// ——锚不可定义（无存活中枢）时开腿保守拒绝并计数（n_c38_nocenter_rejects，
/// 不静默放行先例）。SubAny 不需要锚，开腿路径逐位不变（在册零接触）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevCycleClose {
    /// C38base（在册，已否证）：次级别（k−1）任意买点买回。
    SubAny,
    /// C38buy1：同锚 confirmed Buy1 买回（V2oa25 单次腿的 T5 配对判据）。
    Buy1,
    /// C38zd：ZD 触线买回（几何兑现，V2oa25 单次腿的触线判据）。
    Zd,
    /// C38pair：V2oa25 完整配对闭腿集 = T7 confirmed Buy3 > T6 candidate
    /// Buy3 预回补（pre_type3 轴）> 同锚 confirmed Buy1 > ZD 触线——
    /// step_down_paired 的 if 链在 V2oa25 配置位下（r2_anchor_zg /
    /// r3_t6_sub_confirm / sc_t7_close / buy2_close 全关）的精确退化形式。
    /// "循环版 V2oa25"：循环结构 × 基线闭腿判据的因果分离实验臂。
    Paired,
    /// C38bsp（编排者递归因果纠正 2026-06-11）：本级别任意 confirmed 买点
    /// （Buy3 > Buy2 > Buy1，**任意锚**）买回——买卖点之间有递归因果：
    /// 一卖后回落，次级别反向运动构造出二买（反弹不创新低=底部确认）或
    /// 三买（回调不破中枢=回补位），这些涌现的买点即买回机会。"利用所有
    /// 买卖点"原则的循环形态。candidate Buy3（t6）不入集（在册负槽）。
    /// 无锚依赖（任意锚买点不需要开腿锚快照）——开腿路径同 SubAny。
    BspAny,
    /// C38bspzd：BspAny + ZD 触线几何兜底（买点缺席时的兑现保障）。
    /// ZD 线需要锚 ⇒ 有锚依赖（开腿同 Buy1/Zd/Paired 路径）。
    BspAnyZd,
    /// C38b1zd（**post-hoc 探索臂**，2026-06-11 七臂数据驱动）：兑现型闭因
    /// 子集 = 任意锚 confirmed Buy1 ∨ ZD 触线——七臂逐腿 payoff 分解显示
    /// buy1_any（wr 62-64% 两标的）与 zd 是唯二正闭因，buy3（wr 21-27%）/
    /// t6 是止损型负槽。数据挖掘风险显式声明：本臂是同数据 post-hoc 组合，
    /// 结论上限 = 探索性（L2），需新标的预注册验证才可升级。
    Buy1AnyZd,
}

impl RevCycleClose {
    /// 闭腿判据是否依赖开腿锚快照（同锚比较基准 / ZD 线）。无依赖的臂
    /// 开腿不捕获锚（声明=能力：不消费就不要求）。
    pub fn needs_anchor(self) -> bool {
        !matches!(self, RevCycleClose::SubAny | RevCycleClose::BspAny)
    }
}

/// master 入场模式（2026-06-11 任务：master 入场侧递归建仓）。
///
/// Full = 在册行为：区间套确认 bar 一次性满仓（O0≡P5 零接触面）。
/// Recursive = 同一入场 bar 只部署 base_frac，此后更高级别 confirmed buy1
/// 逐档追加（quota 翻倍——RecursivePosition WeightFn::Exp2 同构：级别时间
/// 尺度几何递增的镜像；26课"级别的意义基本只和买卖量有关"），buy1 落在
/// 当前最高涌现层 = "最高级别确认" → 补满剩余全部。入场触发/出场逻辑
/// 零改动：仍区间套确认 bar 开仓（同 bar 同价）、entry_ladder 的 sell1
/// 一次性全清（未部署现金计入 close 的 total_value——资金守恒）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EntryMode {
    Full,
    Recursive { base_frac: f64 },
}

/// REV 开腿锚定强度（C3 消融轴 S）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubAnchor {
    /// 无锚定（v1 形态，对照基线用）。
    Off,
    /// G1 默认：dir_row[k−1] == Down（次级别方向行，需 D3 磁带行）。
    Direction,
    /// D 强锚定：次级别反向中枢已形成（消融轴 S）。
    CenterFormed,
}

/// 有机赋格 v2 配置。P5 继承轴默认值 = P5 逐字（V0≡P5 守卫的基线锚定）。
#[derive(Debug, Clone)]
pub struct OrganicConfig {
    // ── P5 继承轴 ──
    pub open_kinds: Vec<BspKind>,
    pub hard_type3: bool,
    pub pre_type3: bool,
    pub center_gate: bool,
    pub theta_amp: f64,
    pub same_center_close: bool,
    pub osc_mode: bool,
    pub osc_buy_sub: bool,
    // ── 有机扩展轴（v2） ──
    pub rev_mode: bool,
    pub sub_anchor: SubAnchor,
    /// false = 一次性 frac_k（V1f）；true = tranche 递归建仓/平仓（V1r，需 D3 磁带行）。
    pub tranche: bool,
    pub rev_gate: bool,
    pub rev_close: RevClose,
    /// C6 §5.3 Sell2 格：默认 no-op；消融轴 R2 显式表态位。
    pub sell2_trigger: bool,
    /// REV 配对修正（2026-06-10 编排者任务）：开腿 kind 展开（震荡型/逃逸型）
    /// + 闭腿配对（同锚 confirmed Buy1 / Buy3 回补 / ZD 触线，拒 kind 不匹配）。
    /// false = legacy 段终结三触发 + buy_any 盲闭（V1/V1f 形态，对照基线）。
    pub rev_paired: bool,
    /// 深度门槛：开 REV 前要求锚中枢振幅 (ZG−ZD)/price ≥ θ_depth（初值 1%）。
    /// 0.0 = 门关（V2p：配对修正的独立因果隔离位）。仅 rev_paired 路径消费。
    /// theta_mode=AdaptiveQuantile 时本值降为 warm-up 回退值。
    pub theta_depth: f64,
    /// θ 取值方式（Fixed = 在册基线；AdaptiveQuantile = 因果滚动分位数）。
    /// 仅 rev_paired 路径消费（legacy 腿无深度门概念）。
    pub theta_mode: ThetaMode,
    /// 成本门下界倍数 k（θ 相对化落地任务，2026-06-11）：AdaptiveQuantile
    /// 分支的 θ_eff = max(θ_quantile, k × friction_rt)——分位数可低至任意小
    /// （高频域中枢振幅趋零），低于 k 倍往返摩擦的门放行的腿期望必负，
    /// 下界是相对化语义的严格组成部分而非可选项（35课成本门的 REV 主腿形态）。
    /// 仅 AdaptiveQuantile 分支消费（Fixed 模式 θ 由变体预注册，逐位不变）。
    pub theta_cost_k: f64,
    /// 往返摩擦成本（比例）。默认 10bps（卖飞修复 be9.7bps 先例的量级锚定，
    /// 与 sub_friction_rt 同值不同消费点——本值供 REV 主腿成本门下界）。
    pub friction_rt: f64,
    /// 41课门（REV 主腿形态，θ 相对化落地任务）：REV 开腿前检查直接父级别
    /// （ladder+1）向上走势衰竭状态——相邻同向（Up）段创新高 ∧ 当前段窗口内
    /// 无盘整背驰 = 上涨趋势未完 = 拒开反向腿（"大级别走势没有任何衰竭时
    /// 参与反向小级别买卖点是刀口舔血"）。与 sub_l41_gate（Fractal 子腿守
    /// 父级别 Down 衰竭）镜像；与 rev_gate（FatigueGate，需 run_high 行）
    /// 数据基础不同——本门用 D3 方向行 + close + div 事件流（磁带已产出）。
    /// n_rev_l41_rejects 可观测。仅 rev_paired 路径消费。
    pub rev_l41_gate: bool,
    /// 逃逸型开腿开关（kind 标注的消融轴）：首跑 kind 分解显示逃逸型三标的
    /// 一致为负（OKLO V2p −4.4K / V2f −39.8K，QQQ −10.1K）而震荡型胜率 53%+——
    /// false = 只开震荡型（V2o/V2of）。仅 rev_paired 路径消费。
    pub rev_escape_open: bool,
    // ── 盘背递归正则化消融位（2026-06-11 任务；调研报告
    //    analysis/consolidation_div_regularization_research.md §4）──
    /// R1：盘背卖开腿要求次级别 Sell 侧证据（卖侧背驰 ∪ confirmed Sell1）落在
    /// 次级别最近 Up run 窗口内（≈ 本级别 C 段，27课区间套一步收缩）。
    /// 仅约束盘背触发源（bit1）；Sell1 触发的震荡型开腿不受影响。
    /// 需要 D3 dir_flips 行（run_anchor 给窗口起点）。仅 rev_paired 路径消费。
    pub r1_sub_sell_open: bool,
    /// R2：震荡型兑现锚 ZD→ZG（对齐原文保证域"理论只能保证其回拉原来的走势
    /// 中枢"，第24课/编纂版第四节）。ZD 降为条件延伸目标：触 ZG 时次级别回拉
    /// 走势尚未完成（腿生命期内无次级别买侧证据）且延伸档门过（中枢振幅 ≥ θ）
    /// 则持有至 ZD/次级别买证据。深度门语义同步：(c−ZG)/c ≥ θ（可兑现段）。
    /// 仅 rev_paired 震荡型消费。
    pub r2_anchor_zg: bool,
    /// R3：t6 预回补（candidate Buy3）要求次级别"回跌不重回中枢"已成立证据：
    /// 腿生命期内次级别买侧证据已出现 ∧ 回拉低点 > ZG。证据缺失时 t6 不触发
    /// （T7 confirmed Buy3 不变）。仅 rev_paired 震荡型消费。
    pub r3_t6_sub_confirm: bool,
    // ── 次级别确认完整递归消融位（2026-06-11 任务；27课区间套延拓到全部
    //    操作点。统一谓词 BarRows::sub_confirm = 同 bar 事件证据 ∨ D3 方向行
    //    结构证据——非 R1 的窗口记忆形式，bi 层有 D3 行故有效域非空）──
    /// SCe：ARMED→入场取消 SUB_EXPIRY 超时 fallback——无次级别买确认不入场
    /// （type1 买开仓的严格区间套；超时后继续等待直到确认或 sell1 撤防）。
    pub sc_entry_strict: bool,
    /// SCm：master type1 卖出场（含 earning 升级出场）要求次级别卖侧确认
    /// （次级别向上走势结束证据）。确认缺失时本 bar 持有（出场延迟可观测）。
    pub sc_master_exit: bool,
    /// SCo：main 降成本腿卖开（type1/type2 卖）要求次级别卖侧确认。
    pub sc_main_open: bool,
    /// SCc：main 腿同锚 confirmed Buy1 闭腿（normal）要求次级别买侧确认
    /// （次级别向下走势结束证据）。pre/hard type3 通道不受影响。
    pub sc_main_close: bool,
    /// SCr：REV 开腿（rev_paired 全触发源）要求次级别卖侧确认。与 R1 的差异：
    /// R1 = 窗口记忆 + 仅事件证据（k=2 空定义域反选，已否证）；SCr = 同 bar
    /// 共现 + D3 方向行（bi 层有效域非空）。仅 rev_paired 路径消费。
    pub sc_rev_open: bool,
    /// SC7：confirmed Buy3 回补（main hard / REV T7）要求次级别买侧确认
    /// （24课"回抽不破"的次级别形式）。candidate Buy3（t6/pre）不加确认——
    /// R3 判决先例：抑制 t6 只把出场漏到更差价位。预注册风险：与 R3 同构
    /// （延迟 type3 通道出场），若 REV 净恶化即否证。
    pub sc_t7_close: bool,
    // ── type2 消融位（2026-06-11 任务；C段修复后 type2 0→数百解锁）──
    /// T2o：confirmed Sell2 入配对开腿震荡型触发集（17课对称/定律一：type2 卖
    /// = type1 卖后次级别下跌结束再上涨不创新高的结束点——顶部确认，比 Sell1
    /// 更确认的反向段开启信号）。锚解析与 Sell1 同路径（alive 中枢 + 深度门），
    /// trigger 掩码 bit3。仅 rev_paired 路径消费；与 legacy 轴 `sell2_trigger`
    /// 互不相通。
    pub sell2_open: bool,
    /// T2c：confirmed Buy2 入配对闭腿集（reason=10），配对规则镜像 Buy1：
    /// 震荡型同锚（e.cs == 锚 cs）/ 逃逸型任意锚。定律一：type2 由次级别
    /// type1 构成——type1 买说"下跌可能结束"，type2 买说"确实结束"（回落不
    /// 创新低），REV 空腿假设在此被结构性否证 ⇒ 回补。默认 false 时 Buy2
    /// 仍计入 n_rev_mismatch_holds（反事实可观测不变）。仅 rev_paired 消费。
    pub buy2_close: bool,
    pub sizing: Sizing,
    pub earning_reaction: bool,
    pub market_mode: MarketMode,
    /// T4b-(c) run_anchor 归属容差（设计未给值的实现常数，显式化为配置而非魔数）。
    pub t4b_anchor_margin: i64,
    // ── 递归赋格（2026-06-11 任务；analysis/recursive_fugue_ultimate_design.md）──
    /// REV 腿内嵌套子 LOU 的最大深度。0 = 在册行为（无子腿，O0≡P5/V2of 零接触）；
    /// 1 = voice REV 窗口内 k−1 级别反弹腿（深度1 L2 实验）。符号交替塔：
    /// 奇数深度先买后卖，偶数深度先卖后买。仅 rev_paired 路径消费（runner guard）。
    /// 递归终止 = 深度预算 ∧ 子级别 ≥ FIRST_BSP_LADDER ∧ 振幅 ≥ 2×sub_friction_rt。
    pub rev_sub_depth: usize,
    /// 子腿往返摩擦成本（比例）。经济终止条件：子级别锚中枢相对振幅
    /// (ZG−ZD)/c < 2×本值 ⇒ 拒开（期望必负，设计报告 §4.1 经济下限）。
    /// 默认 0.001（≈10bps 往返，卖飞修复 be9.7bps 先例的量级锚定）。
    /// 仅 Zhongshu 模式消费（Fractal 无中枢振幅可测——分型即信号）。
    pub sub_friction_rt: f64,
    /// 子腿操作锚模式（2026-06-11 笔级分型任务）。
    /// Zhongshu = 在册行为（开闭腿以 k−1 存活中枢 ZG/ZD 为域——O_sub0 两票
    /// 否证的对象：74% 宿主被"无中枢"拒，父腿趋势运行中子域不可定义）；
    /// Fractal = 38课程式的**笔级分型近似**（编排者概念纠正 2026-06-11）：
    /// "顶卖底买"以 D3 方向行翻转为进出点（翻 Down=顶分型确认→卖，
    /// 翻 Up=底分型确认→买回），不需要中枢域。**非严格形式**——38课的
    /// 操作对象是趋势级别的反向线段（趋势=至少两个同向中枢的走势，其内部
    /// 每段回调/反弹线段有内部结构故短差有域），严格进出判据是段间盘整
    /// 背驰（第38课"根据其内部结构可以判断其背驰或盘整背驰结束点，先卖出
    /// ……不跌破第一段低点，重新买入"），不是笔的分型翻转。本模式的否证
    /// 结论有效域限近似实现本身。仅 rev_sub_depth > 0 时消费。
    pub sub_mode: SubMode,
    // ── P1 双门（2026-06-11 任务；Fractal 子腿的 35课成本门 + 41课门）──
    /// 成本门（35课："交易成本+交易误差相对波幅不够小的级别，长期操作没有
    /// 意义"）。Fractal 子腿无逐腿锚振幅可测（分型即信号），级别可操作性
    /// 用该层中枢振幅的因果滚动中位数 θ_q（DepthRef，零前瞻）度量：
    /// θ_eff = max(θ_q, sub_cost_k × sub_friction_rt)；级别可操作 ⟺
    /// θ_q ≥ θ_eff ⟺ θ_q ≥ k×friction——典型振幅不够覆盖 k 倍往返成本的
    /// 级别自动关闭（n_sub_cost_rejects 可观测）。参照不可定义的级别
    /// （bi 级无中枢事件流 / warm-up 样本不足）保守拒绝并独立计数
    /// （n_sub_cost_noref_rejects，"不静默放行"先例）。仅 Fractal 消费
    /// （Zhongshu 模式的逐锚 2×sub_friction_rt 经济门已是其成本门形态）。
    pub sub_cost_gate: bool,
    /// 成本门倍数 k（θ 下限 = k × sub_friction_rt；35课"不够小"的量化档）。
    pub sub_cost_k: f64,
    /// REV 声部循环模式（38课循环 voice，2026-06-11）。Single = 在册单次腿；
    /// Cycle38 = 趋势存续期间循环短差（见 RevCycle docstring）。Cycle38 路径
    /// **不消费 rev_l41_gate**——其语义（宿主趋势未衰竭 ⇒ 拒开反向腿）与
    /// 循环存续条件（宿主趋势存续 ⇒ 循环开放）正面矛盾；41课时效语义在
    /// Cycle38 下由 master 出场承载（type1 卖 = 衰竭信号本体，只管 master
    /// 级别），voice 级别不受限（任务裁决：41课门改绑 master 级别）。
    /// 每条循环短差腿过成本门（35课：该层典型中枢振幅 θ_q ≥
    /// theta_cost_k × friction_rt，DepthRef 因果滚动中位数，零前瞻）。
    pub rev_cycle: RevCycle,
    /// Cycle38 买回判据（见 RevCycleClose docstring）。仅 rev_cycle=Cycle38
    /// 路径消费；Single 模式是死配置位（runner guard 不拒——默认值零接触）。
    pub rev_cycle_close: RevCycleClose,
    /// 41课门（41课："大级别走势没有任何衰竭时参与反向小级别买卖点是刀口
    /// 舔血"）。子腿开腿前检查直接父级别（self.ladder+1）走势衰竭状态
    /// （TrendExhaustion，市场性质）：相邻同向（Down）段创新低 ∧ 当前段窗口
    /// 内无盘整背驰 = 趋势未完 = 拒开（n_sub_l41_rejects 可观测）。
    /// 与 rev_gate（C7 FatigueGate）的差异：数据基础是 D3 方向行 + close
    /// 序列 + div 事件流（当前磁带已产出），非 run_high 行（未产出，
    /// rev_gate fail-fast）。仅 Fractal 子腿消费。
    pub sub_l41_gate: bool,
    /// master 入场模式（见 EntryMode docstring）。Full = 在册满仓入场。
    pub entry_mode: EntryMode,
}

/// 成本门 θ_q 的分位数（中位数 = 该层"典型"中枢振幅；35课判据是级别的
/// 常态波幅，非尾部）。预注册常数，不做标的级拟合（方案 C 前瞻否定先例）。
pub const SUB_COST_Q: f64 = 0.5;
/// 成本门参照集最小样本数（与 θ 自适应变体 min_obs=10 同值——warm-up 期
/// 参照不可定义，保守拒绝不静默放行）。
pub const SUB_COST_MIN_OBS: usize = 10;

/// 递归子腿的操作锚模式（见 `OrganicConfig::sub_mode`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubMode {
    /// 中枢域锚（在册）：开腿需 k−1 存活中枢，ZG/ZD 为边界。
    Zhongshu,
    /// 笔级分型锚（38课程式的近似实现，非严格形式——严格形式 = 趋势级
    /// 反向线段锚 + 段间盘整背驰进出，未实装）：方向行翻转为进出点，
    /// 存在论下限 = bi 级。
    Fractal,
}

impl Default for OrganicConfig {
    fn default() -> Self {
        OrganicConfig {
            open_kinds: vec![BspKind::Type1, BspKind::Type2],
            hard_type3: true,
            pre_type3: true,
            center_gate: true,
            theta_amp: 0.01,
            same_center_close: true,
            osc_mode: true,
            osc_buy_sub: false,
            rev_mode: false,
            sub_anchor: SubAnchor::Off,
            tranche: false,
            rev_gate: false,
            rev_close: RevClose::Conf,
            sell2_trigger: false,
            rev_paired: false,
            theta_depth: 0.0,
            theta_mode: ThetaMode::Fixed,
            theta_cost_k: 2.0,
            friction_rt: 0.001,
            rev_l41_gate: false,
            rev_escape_open: true,
            r1_sub_sell_open: false,
            r2_anchor_zg: false,
            r3_t6_sub_confirm: false,
            sc_entry_strict: false,
            sc_master_exit: false,
            sc_main_open: false,
            sc_main_close: false,
            sc_rev_open: false,
            sc_t7_close: false,
            sell2_open: false,
            buy2_close: false,
            sizing: Sizing::Equal,
            earning_reaction: false,
            market_mode: MarketMode::Stock,
            t4b_anchor_margin: 0,
            rev_sub_depth: 0,
            sub_friction_rt: 0.001,
            sub_mode: SubMode::Zhongshu,
            sub_cost_gate: false,
            sub_cost_k: 2.0,
            sub_l41_gate: false,
            rev_cycle: RevCycle::Single,
            rev_cycle_close: RevCycleClose::SubAny,
            entry_mode: EntryMode::Full,
        }
    }
}

/// v2 变体表（§8.1 消融轴正交分解）。`O0` 为 `V0` 别名（对账脚本兼容）。
pub fn variant(name: &str) -> Option<OrganicConfig> {
    let base = OrganicConfig::default();
    match name {
        // V0：≡P5 逐位守卫（trades+trace+counters 三面）
        "V0" | "O0" => Some(base),
        // V1：rev 裸开（无 G1/tranche/门）——v1 O1v 的 v2 语义类比
        // （差异仅 C2 osc 不截断 + C5 单 tranche 区间套读出退化为 home 层），
        // 与在册 O1v 基线（Δ=−63.7/−41.4/−186.2pp）直接可比，隔离 C2 的因果。
        "V1" => Some(OrganicConfig { rev_mode: true, ..base }),
        // V1f：G1 锚定的独立因果（一次性 frac_k = O1v+G1）
        "V1f" => Some(OrganicConfig {
            rev_mode: true,
            sub_anchor: SubAnchor::Direction,
            ..base
        }),
        // V1r：递归建仓/平仓的独立因果（V1r−V1f）
        "V1r" => Some(OrganicConfig {
            rev_mode: true,
            sub_anchor: SubAnchor::Direction,
            tranche: true,
            ..base
        }),
        // V2：门 v2 因果（V2−V1r）+ 门开率定义域
        "V2" => Some(OrganicConfig {
            rev_mode: true,
            sub_anchor: SubAnchor::Direction,
            tranche: true,
            rev_gate: true,
            ..base
        }),
        // V3：规模轴（含 REV）
        "V3" => Some(OrganicConfig {
            rev_mode: true,
            sub_anchor: SubAnchor::Direction,
            tranche: true,
            rev_gate: true,
            sizing: Sizing::Structure,
            ..base
        }),
        // V3′：规模轴独立重验（无 REV）——v1 判据3 欠账
        "V3p" => Some(OrganicConfig {
            sizing: Sizing::Structure,
            ..base
        }),
        // V4：earning 反作用（前置：触发率>0）
        "V4" => Some(OrganicConfig {
            rev_mode: true,
            sub_anchor: SubAnchor::Direction,
            tranche: true,
            rev_gate: true,
            sizing: Sizing::Structure,
            earning_reaction: true,
            ..base
        }),
        // 消融 S：强锚定（次级别反向中枢已形成）
        "VS" => Some(OrganicConfig {
            rev_mode: true,
            sub_anchor: SubAnchor::CenterFormed,
            tranche: true,
            ..base
        }),
        // 消融 R：REV 关腿 type1 买确认强度（v1 O2c/O2n 轴的 v2 形态，exploratory）
        "VRc" => Some(OrganicConfig {
            rev_mode: true,
            sub_anchor: SubAnchor::Direction,
            tranche: true,
            rev_close: RevClose::Cand,
            ..base
        }),
        "VRn" => Some(OrganicConfig {
            rev_mode: true,
            sub_anchor: SubAnchor::Direction,
            tranche: true,
            rev_close: RevClose::Nested,
            ..base
        }),
        // V2p：REV 配对修正（kind 展开开腿 + 配对闭腿），深度门关——
        // 隔离配对修正自身的因果（V2f−V2p = 深度门独立贡献）。
        // sub_anchor=Off：G1 方向锚定已按预注册条件否证（t6 占比 3/3 上升），
        // 配对修正的"存活中枢"条件是其替代锚定。
        "V2p" => Some(OrganicConfig {
            rev_mode: true,
            rev_paired: true,
            theta_depth: 0.0,
            ..base
        }),
        // V2f：配对修正 + 深度门槛 θ=1%（2026-06-10 任务主变体）。
        "V2f" => Some(OrganicConfig {
            rev_mode: true,
            rev_paired: true,
            theta_depth: 0.01,
            ..base
        }),
        // V2o：配对修正，仅震荡型开腿（逃逸型关——kind 标注首跑数据驱动的
        // 消融：逃逸型三标的一致为负，见 rev_v2_paired_backtest.md）。
        "V2o" => Some(OrganicConfig {
            rev_mode: true,
            rev_paired: true,
            theta_depth: 0.0,
            rev_escape_open: false,
            ..base
        }),
        // V2of：仅震荡型 + 深度门 θ=1%。
        // V2r 为其别名（C段修复后全量重跑任务，2026-06-11）：只接 type1/盘背卖
        // 开 REV（= 震荡型，逃逸型 Sell3 关）+ 振幅 θ_depth=1% + kind 配对闭腿
        // ——任务规格与 V2of 配置逐位相同，别名而非复制（O0/V0 同一先例）。
        "V2of" | "V2r" => Some(OrganicConfig {
            rev_mode: true,
            rev_paired: true,
            theta_depth: 0.01,
            rev_escape_open: false,
            ..base
        }),
        // ── θ 自适应深度门（2026-06-11 任务；基线 = V2of/V2r 的 θ=1% 固定门）──
        // 预注册常数：window=50 / min_obs=10 / q ∈ {0.25, 0.50}——不做标的级
        // 拟合（方案 C 被否定为回测期内前瞻）。theta_depth=0.01 保留为 warm-up
        // 回退值（样本不足时维持基线门，不静默放行）。
        // ── θ 相对化默认门（2026-06-11 落地任务）：V2oa25 升级为 REV 配对
        // 路径的默认门——AdaptiveQuantile q=0.25 + 成本门下界
        // θ_eff = max(θ_q, 2×10bps)（theta_cost_k/friction_rt 默认值，
        // 在 effective_theta 的 Adaptive 分支无条件生效）+ 41课门
        // （rev_l41_gate：父级别上行无衰竭拒开反向腿）。
        // 注意：与首验 V2oa25（纯自适应，OKLO+520/BRN+71.5pp）不逐位可比
        // ——首验数字在 theta_adaptive_backtest.json 在册，本定义是落地形态。
        "V2oa25" => Some(OrganicConfig {
            theta_mode: ThetaMode::AdaptiveQuantile { q: 0.25, window: 50, min_obs: 10 },
            rev_l41_gate: true,
            ..variant("V2of").expect("V2of 在上方注册")
        }),
        "V2oa50" => Some(OrganicConfig {
            theta_mode: ThetaMode::AdaptiveQuantile { q: 0.50, window: 50, min_obs: 10 },
            ..variant("V2of").expect("V2of 在上方注册")
        }),
        // V2oa25C38：38课循环 voice（2026-06-11 任务）。基线 = V2oa25 默认门；
        // 差异轴：rev_cycle=Cycle38（趋势存续期循环短差，替换单次 REV 腿）
        // + rev_l41_gate=false（Cycle38 路径不消费该门——保留 true 是死配置位，
        // 声明=能力；41课时效语义由循环存续条件 + master 出场承载）。
        // θ 自适应/成本门下界继承 V2oa25（循环开腿的成本门用同一
        // theta_cost_k × friction_rt 下界，参照 = 该层因果滚动中位数）。
        "V2oa25C38" => Some(OrganicConfig {
            rev_cycle: RevCycle::Cycle38,
            rev_l41_gate: false,
            ..variant("V2oa25").expect("V2oa25 在上方注册")
        }),
        // ── 38课循环买回判据消融（2026-06-11 任务）：C38base（上方 V2oa25C38，
        // 次级别任意买点）已否证（OKLO −472.8/BRN −70.4pp，死因 = 买回级别
        // 错配）。编排者原则"区间套无论正反都存在"——买回端也必须先过本级别
        // 判据。三臂只动 rev_cycle_close 一轴（开腿触发/成本门/循环存续全同；
        // 唯一伴随差异 = 锚捕获准入，n_c38_nocenter_rejects 可观测）。──
        "V2oa25C38buy1" => Some(OrganicConfig {
            rev_cycle_close: RevCycleClose::Buy1,
            ..variant("V2oa25C38").expect("V2oa25C38 在上方注册")
        }),
        "V2oa25C38zd" => Some(OrganicConfig {
            rev_cycle_close: RevCycleClose::Zd,
            ..variant("V2oa25C38").expect("V2oa25C38 在上方注册")
        }),
        // C38pair = 循环版 V2oa25：判别 C38pair vs V2oa25 基线 ⇒ 循环结构
        // 自身的增量价值（> 基线 = 反复操作贡献正 alpha；≤ 基线 = 循环结构
        // 无增量，38课循环假设在事件流近似下整体关闭）。
        "V2oa25C38pair" => Some(OrganicConfig {
            rev_cycle_close: RevCycleClose::Paired,
            ..variant("V2oa25C38").expect("V2oa25C38 在上方注册")
        }),
        // 编排者递归因果纠正臂（2026-06-11）：一卖→回落→二买/三买涌现即
        // 买回机会——本级别任意 confirmed 买点（任意锚）买回；bspzd 加
        // ZD 几何兜底。
        "V2oa25C38bsp" => Some(OrganicConfig {
            rev_cycle_close: RevCycleClose::BspAny,
            ..variant("V2oa25C38").expect("V2oa25C38 在上方注册")
        }),
        "V2oa25C38bspzd" => Some(OrganicConfig {
            rev_cycle_close: RevCycleClose::BspAnyZd,
            ..variant("V2oa25C38").expect("V2oa25C38 在上方注册")
        }),
        // post-hoc 探索臂：兑现型闭因子集（buy1_any + zd，排除 buy3/t6
        // 止损型负槽）——数据挖掘风险见 RevCycleClose::Buy1AnyZd docstring。
        "V2oa25C38b1zd" => Some(OrganicConfig {
            rev_cycle_close: RevCycleClose::Buy1AnyZd,
            ..variant("V2oa25C38").expect("V2oa25C38 在上方注册")
        }),
        // V2oa25F1：默认门 + 笔级分型递归 depth=1 + P1 双门（成本门/41课门的
        // 子腿形态——P0+P1 判决：成本门=亏损有界化，41课门首次非死门）。
        "V2oa25F1" => Some(OrganicConfig {
            rev_sub_depth: 1,
            sub_mode: SubMode::Fractal,
            sub_cost_gate: true,
            sub_l41_gate: true,
            ..variant("V2oa25").expect("V2oa25 在上方注册")
        }),
        // ── 盘背递归正则化消融矩阵（2026-06-11；基线 = V2r，三位独立掩码）──
        "V2rR1" => Some(OrganicConfig {
            r1_sub_sell_open: true,
            ..variant("V2r").expect("V2r 在上方注册")
        }),
        "V2rR2" => Some(OrganicConfig {
            r2_anchor_zg: true,
            ..variant("V2r").expect("V2r 在上方注册")
        }),
        "V2rR3" => Some(OrganicConfig {
            r3_t6_sub_confirm: true,
            ..variant("V2r").expect("V2r 在上方注册")
        }),
        "V2rR12" => Some(OrganicConfig {
            r1_sub_sell_open: true,
            r2_anchor_zg: true,
            ..variant("V2r").expect("V2r 在上方注册")
        }),
        "V2rR123" => Some(OrganicConfig {
            r1_sub_sell_open: true,
            r2_anchor_zg: true,
            r3_t6_sub_confirm: true,
            ..variant("V2r").expect("V2r 在上方注册")
        }),
        // ── 次级别确认完整递归消融矩阵（2026-06-11；基线 = V2r，六位独立掩码）──
        "V2rSCe" => Some(OrganicConfig {
            sc_entry_strict: true,
            ..variant("V2r").expect("V2r 在上方注册")
        }),
        "V2rSCm" => Some(OrganicConfig {
            sc_master_exit: true,
            ..variant("V2r").expect("V2r 在上方注册")
        }),
        "V2rSCo" => Some(OrganicConfig {
            sc_main_open: true,
            ..variant("V2r").expect("V2r 在上方注册")
        }),
        "V2rSCc" => Some(OrganicConfig {
            sc_main_close: true,
            ..variant("V2r").expect("V2r 在上方注册")
        }),
        "V2rSCr" => Some(OrganicConfig {
            sc_rev_open: true,
            ..variant("V2r").expect("V2r 在上方注册")
        }),
        "V2rSC7" => Some(OrganicConfig {
            sc_t7_close: true,
            ..variant("V2r").expect("V2r 在上方注册")
        }),
        "V2rSCall" => Some(OrganicConfig {
            sc_entry_strict: true,
            sc_master_exit: true,
            sc_main_open: true,
            sc_main_close: true,
            sc_rev_open: true,
            sc_t7_close: true,
            ..variant("V2r").expect("V2r 在上方注册")
        }),
        // ── type2 消融矩阵（2026-06-11；基线 = V2r，开腿/闭腿两位独立掩码）──
        "V2rT2o" => Some(OrganicConfig {
            sell2_open: true,
            ..variant("V2r").expect("V2r 在上方注册")
        }),
        "V2rT2c" => Some(OrganicConfig {
            buy2_close: true,
            ..variant("V2r").expect("V2r 在上方注册")
        }),
        "V2rT2" => Some(OrganicConfig {
            sell2_open: true,
            buy2_close: true,
            ..variant("V2r").expect("V2r 在上方注册")
        }),
        // ── 递归赋格深度1（2026-06-11；基线 = V2of，O_sub1/O_sub0 预注册判据）──
        // depth=0 即 V2of 本体（基线不另设名）；S1 = REV 腿内挂 k−1 反弹腿。
        "V2ofS1" => Some(OrganicConfig {
            rev_sub_depth: 1,
            ..variant("V2of").expect("V2of 在上方注册")
        }),
        // F1 = REV 窗口内 k−1 笔级分型短差腿（38课"向下段顶卖底买"直读，
        // 不需要中枢域——O_sub0 否证的"无中枢拒"在此模式无定义域缺口）。
        "V2ofF1" => Some(OrganicConfig {
            rev_sub_depth: 1,
            sub_mode: SubMode::Fractal,
            ..variant("V2of").expect("V2of 在上方注册")
        }),
        // F2 = 分型子腿再嵌一层（depth=2：子腿开放窗口内 k−2 级分型短差，
        // step_fractal 递归预算 path.depth() < rev_sub_depth ∧ k ≥ 2；
        // P0-a 任务 2026-06-11，预注册判据 depth2 > depth1）。
        "V2ofF2" => Some(OrganicConfig {
            rev_sub_depth: 2,
            sub_mode: SubMode::Fractal,
            ..variant("V2of").expect("V2of 在上方注册")
        }),
        // ── P1 双门四格消融（2026-06-11；基线 = V2ofF1 裸分型子腿）──
        // c = 仅成本门（35课）；g = 仅41课门；cg = 双门。
        "V2ofF1c" => Some(OrganicConfig {
            sub_cost_gate: true,
            ..variant("V2ofF1").expect("V2ofF1 在上方注册")
        }),
        "V2ofF1g" => Some(OrganicConfig {
            sub_l41_gate: true,
            ..variant("V2ofF1").expect("V2ofF1 在上方注册")
        }),
        "V2ofF1cg" => Some(OrganicConfig {
            sub_cost_gate: true,
            sub_l41_gate: true,
            ..variant("V2ofF1").expect("V2ofF1 在上方注册")
        }),
        // ── master 入场侧递归建仓（2026-06-11；基线 = V2oa25，单轴 entry_mode）──
        // base_frac 三档：0.2（深递归，最多 3 档追加）/ 1/3 / 0.5（浅递归）。
        // voice（V2oa25 REV+域腿）逐位不碰——唯一差异轴是 master 入场过程。
        "V2oa25_rec" => Some(OrganicConfig {
            entry_mode: EntryMode::Recursive { base_frac: 0.2 },
            ..variant("V2oa25").expect("V2oa25 在上方注册")
        }),
        "V2oa25_rec3" => Some(OrganicConfig {
            entry_mode: EntryMode::Recursive { base_frac: 1.0 / 3.0 },
            ..variant("V2oa25").expect("V2oa25 在上方注册")
        }),
        "V2oa25_rec5" => Some(OrganicConfig {
            entry_mode: EntryMode::Recursive { base_frac: 0.5 },
            ..variant("V2oa25").expect("V2oa25 在上方注册")
        }),
        // 消融 R2：Sell2 入段终结触发集（§5.3 矩阵 Sell2 格的表态轴，exploratory）
        "VR2" => Some(OrganicConfig {
            rev_mode: true,
            sub_anchor: SubAnchor::Direction,
            tranche: true,
            sell2_trigger: true,
            ..base
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v0_is_p5() {
        let cfg = variant("V0").unwrap();
        assert!(!cfg.rev_mode);
        assert_eq!(cfg.open_kinds, vec![BspKind::Type1, BspKind::Type2]);
        assert!(cfg.hard_type3 && cfg.pre_type3 && cfg.center_gate);
        assert_eq!(cfg.theta_amp, 0.01);
        assert!(cfg.osc_mode && !cfg.osc_buy_sub);
        assert_eq!(cfg.sizing, Sizing::Equal);
    }

    #[test]
    fn o0_alias() {
        assert!(variant("O0").is_some());
        assert!(variant("nonexistent").is_none());
    }

    #[test]
    fn p1_dual_gate_variants_inherit_v2off1() {
        // 四格消融：裸（V2ofF1）门位全关；c/g/cg 仅差门位（其余逐位同基线）
        let base = variant("V2ofF1").unwrap();
        assert!(!base.sub_cost_gate && !base.sub_l41_gate);
        assert_eq!(base.sub_cost_k, 2.0);
        assert_eq!(base.sub_friction_rt, 0.001);
        for (name, cost, l41) in [
            ("V2ofF1c", true, false),
            ("V2ofF1g", false, true),
            ("V2ofF1cg", true, true),
        ] {
            let cfg = variant(name).unwrap();
            assert_eq!(cfg.sub_cost_gate, cost, "{name}");
            assert_eq!(cfg.sub_l41_gate, l41, "{name}");
            assert_eq!(cfg.rev_sub_depth, 1);
            assert_eq!(cfg.sub_mode, SubMode::Fractal);
            assert_eq!(cfg.sub_cost_k, 2.0);
        }
    }

    #[test]
    fn theta_adaptive_variants_inherit_v2of() {
        let v2of = variant("V2of").unwrap();
        assert_eq!(v2of.theta_mode, ThetaMode::Fixed);
        assert!(!v2of.rev_l41_gate);
        for (name, q) in [("V2oa25", 0.25), ("V2oa50", 0.50)] {
            let cfg = variant(name).unwrap();
            assert_eq!(
                cfg.theta_mode,
                ThetaMode::AdaptiveQuantile { q, window: 50, min_obs: 10 }
            );
            // 与 V2of 逐位一致的继承面（含 θ=1% 回退值）
            assert_eq!(cfg.theta_depth, 0.01);
            assert!(cfg.rev_mode && cfg.rev_paired && !cfg.rev_escape_open);
            // 成本门下界常数（Adaptive 分支无条件生效）：2 × 10bps = 0.2%
            assert_eq!(cfg.theta_cost_k, 2.0);
            assert_eq!(cfg.friction_rt, 0.001);
        }
        // 默认门 = V2oa25：41课门开；V2oa50 保持纯自适应（消融对照位）
        assert!(variant("V2oa25").unwrap().rev_l41_gate);
        assert!(!variant("V2oa50").unwrap().rev_l41_gate);
    }

    #[test]
    fn cycle38_variant_inherits_v2oa25_with_l41_off() {
        // 默认全变体 rev_cycle=Single（cycle38 默认关，depth=0 行为不变）
        assert_eq!(OrganicConfig::default().rev_cycle, RevCycle::Single);
        assert_eq!(variant("V2oa25").unwrap().rev_cycle, RevCycle::Single);
        let cfg = variant("V2oa25C38").unwrap();
        assert_eq!(cfg.rev_cycle, RevCycle::Cycle38);
        // 41课门改绑 master（Cycle38 不消费 rev_l41_gate ⇒ 显式 false 非死位）
        assert!(!cfg.rev_l41_gate);
        // 其余继承 V2oa25：θ 自适应 + 成本门下界 + 震荡型触发集
        assert_eq!(
            cfg.theta_mode,
            ThetaMode::AdaptiveQuantile { q: 0.25, window: 50, min_obs: 10 }
        );
        assert!(cfg.rev_mode && cfg.rev_paired && !cfg.rev_escape_open);
        assert_eq!(cfg.theta_cost_k, 2.0);
        assert_eq!(cfg.friction_rt, 0.001);
        assert_eq!(cfg.rev_sub_depth, 0);
    }

    #[test]
    fn cycle38_close_ablation_variants_single_axis() {
        // 默认/在册：rev_cycle_close=SubAny（C38base 零接触）
        assert_eq!(OrganicConfig::default().rev_cycle_close, RevCycleClose::SubAny);
        let base = variant("V2oa25C38").unwrap();
        assert_eq!(base.rev_cycle_close, RevCycleClose::SubAny);
        // 三臂只动 rev_cycle_close 一轴——其余字段与 V2oa25C38 逐位相同
        for (name, want) in [
            ("V2oa25C38buy1", RevCycleClose::Buy1),
            ("V2oa25C38zd", RevCycleClose::Zd),
            ("V2oa25C38pair", RevCycleClose::Paired),
            ("V2oa25C38bsp", RevCycleClose::BspAny),
            ("V2oa25C38bspzd", RevCycleClose::BspAnyZd),
            ("V2oa25C38b1zd", RevCycleClose::Buy1AnyZd),
        ] {
            let cfg = variant(name).unwrap();
            assert_eq!(cfg.rev_cycle_close, want, "{name}");
            assert_eq!(cfg.rev_cycle, RevCycle::Cycle38, "{name}");
            let normalized =
                OrganicConfig { rev_cycle_close: RevCycleClose::SubAny, ..cfg };
            // Vec 字段（open_kinds）相等 + 标量字段逐位（Debug 串比较——
            // OrganicConfig 未派生 PartialEq，f64 字段在变体表中全为字面常数）
            assert_eq!(format!("{normalized:?}"), format!("{base:?}"), "{name}");
        }
        // Paired 的前提：V2oa25 配置位下 step_down_paired 闭腿链精确退化为
        // T7>T6>Buy1>ZD——四个被退化掉的配置位必须全关（否则等价推导失效）
        assert!(!base.r2_anchor_zg && !base.r3_t6_sub_confirm);
        assert!(!base.sc_t7_close && !base.buy2_close);
        assert!(base.pre_type3); // T6 在集合内的前提
    }

    #[test]
    fn recursive_entry_variants_single_axis() {
        // 默认/在册全变体 entry_mode=Full（O0≡P5 零接触面）
        assert_eq!(OrganicConfig::default().entry_mode, EntryMode::Full);
        let base = variant("V2oa25").unwrap();
        assert_eq!(base.entry_mode, EntryMode::Full);
        // 三臂只动 entry_mode 一轴——其余字段与 V2oa25 逐位相同
        for (name, frac) in
            [("V2oa25_rec", 0.2), ("V2oa25_rec3", 1.0 / 3.0), ("V2oa25_rec5", 0.5)]
        {
            let cfg = variant(name).unwrap();
            assert_eq!(cfg.entry_mode, EntryMode::Recursive { base_frac: frac }, "{name}");
            let normalized = OrganicConfig { entry_mode: EntryMode::Full, ..cfg };
            assert_eq!(format!("{normalized:?}"), format!("{base:?}"), "{name}");
        }
    }

    #[test]
    fn v2oa25f1_inherits_default_gate_plus_fractal_dual_gate() {
        let cfg = variant("V2oa25F1").unwrap();
        assert_eq!(
            cfg.theta_mode,
            ThetaMode::AdaptiveQuantile { q: 0.25, window: 50, min_obs: 10 }
        );
        assert!(cfg.rev_l41_gate);
        assert_eq!(cfg.rev_sub_depth, 1);
        assert_eq!(cfg.sub_mode, SubMode::Fractal);
        assert!(cfg.sub_cost_gate && cfg.sub_l41_gate);
    }
}
