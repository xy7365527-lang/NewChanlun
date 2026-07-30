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
    AdaptiveQuantile {
        q: f64,
        window: usize,
        min_obs: usize,
    },
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

/// master 出场模式（2026-06-11 任务：master 出场事件驱动→状态驱动）。
///
/// 范畴背景：BTC 全历史 P5 基线 −45% vs BH +1380%——master 见 entry 级
/// type1 卖即清仓，在超强单边标的上反复丢掉趋势主体段（递归建仓双标的
/// 否证已确认"级别确认是状态范畴非事件范畴"；本轴把同一区分应用到出场侧）。
///
/// Signal = 在册行为（O0≡P5 零接触面）：entry_ladder 的 confirmed sell1
/// 即出（事件驱动）。
/// HoldTrend = 状态驱动（49课"利润最大化"持币不动 + 41课"大级别走势没有
/// 衰竭时不做反向"）：本级别 sell1 只是必要条件，还需直接父级别
/// （entry_ladder+1）上行趋势衰竭确认（TrendExhaustion::up_unexhausted
/// 为 false——相邻同向段创新高 ∧ 无盘整背驰的正面延续证据不成立）才出；
/// 趋势未衰竭时 master 持仓，sell1 的短差机会由 voice 承载（V2oa25 在册
/// 逻辑零接触）。证据缺失（段对不可定义）⇒ 不拦截出场——门只在趋势
/// 明确延续时关，与 rev_l41_gate"证据缺失不拒开"同一保守语义。
/// HighestOnly = 最激进持仓：只在当前最高涌现层（max_ladder）的 sell1
/// 出场（31课"历史性大顶"的级别相对化读数——最高级别卖点才是大顶）。
/// Emergent = 出场级别跟随持仓走势自身的涌现级别（2026-06-11 编排者
/// 纠正后的形态；第一版 Climb 用全局 max_ladder 做爬梯目标 = 预设外部
/// 参数，违反"级别从下到上涌现"原则，已删除——设计缺陷与否证记录见
/// analysis/master_exit_climb_results.md）。
/// 判据（无状态，每 bar 重读，零额外机制——引擎递归塔的自我复制本身
/// 就是级别涌现）：出场级别 = 从 entry_ladder 向上、父级别方向行连续
/// Up 的最高层。父级别当前段向上 = 持仓走势已被父级别同级别分解识别
/// 为其向上段的组成部分 = 走势归属升一层；逐层递归。与入场区间套对称：
/// 入场看 buy1 涌现在哪个级别，出场看当前走势涌现到哪个级别。高层
/// 翻转/无结构时归属即时回落——保留出场能力（≠HighestOnly 伪装
/// buy-hold）。需 dir_flips 磁带行。hold_trend=true 组合 HoldTrend
/// 语义：sell1 @ 归属级别 ∧ 其直接父级别（动态）上行趋势衰竭才出。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitMode {
    Signal,
    HoldTrend,
    HighestOnly,
    Emergent { hold_trend: bool },
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

/// voice 消费方式（2026-06-11 并发赋格最小可验证实验；deep_think Part II
/// §17 开放问题3"控制状态剥离"的最小化形态）。
///
/// Fsm = 在册行为（O0≡P5 零接触面）：main 腿（带门带锚）+ VoiceUnit
/// 状态机（RIDE/REV/OSC 相位、配对、θ门、41课门）。
/// Ledger = 裸账本（Part II §15"voice 不是机器，是一个有门的账本"——
/// 最小实验连门都不留）：voice@k 的唯一状态 = 腿槽占用（资源状态），
/// 每个 confirmed BSP 事件直接饱和执行——Sell@k 且槽空 → 卖出 frac_k
/// （清 slice），Buy@k 且槽开 → 买回（填 slice）。无相位过滤、无锚比较、
/// 无门谓词、无配对记忆。master 满仓持有/出场逻辑零接触——递归建仓
/// 双标的否证的暴露塌缩死因（slice 占用时间 ≪ 趋势长度）由 master 满仓
/// 兜底规避，本轴只隔离 voice 消费方式的因果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceMode {
    Fsm,
    Ledger,
}

/// osc 域腿操作域（osc 操作对象定义严格化，2026-06-11 任务）。
///
/// 这不是外加门——是 osc 操作对象定义的严格化：38课中枢震荡操作的隐含
/// 前提是确立的中枢（价格在中枢内反复震荡 = 盘整走势）。趋势走势
/// （≥2 同向中枢，17课定义）里的中枢不是38课震荡操作的对象——
/// 49课"中枢向上移动时就应该满仓"（趋势走势不做逆向短差）；
/// 26课"单边上扬走势，短线最好别做"。
///
/// 与 osc_l41_gate 的范畴区分：41课门是**点态条件**（父级别衰竭与否的
/// bar 级读数，已否证——门拦截是腿延迟非腿消灭，"未衰竭⇒不回ZD"传导链
/// 断裂）；本轴是**对象域定义**（锚中枢所在走势的 kind，状态范畴）——
/// 趋势走势中 osc 触发点从定义上不存在，而非被逐点拦截。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OscDomain {
    /// 在册 P5 行为：任何存活中枢都可开 osc 腿（O0≡P5 零接触面）。
    Any,
    /// 38课严格域：仅锚中枢所在走势（本层尾 move）kind==Consolidation
    /// 时开腿；kind==Trend ⇒ 不开。需要 trend_flips 磁带行（runner guard）。
    ConsolidationOnly,
    /// H3 级别上移（26课行183"最好别按1分钟弄，5分钟甚至更长都可以"，
    /// osc 白名单消除任务 2026-06-12）：盘整态 = ConsolidationOnly 同语义
    /// （k 层域内正常开）；趋势态 = **重路由而非删除**——osc 操作级别整体
    /// 上移到 k+1（触发判据 c≥ZG(k+1)∧sub_sell(k)、锚中枢、ZD 边界、死亡
    /// 出口全部按 k+1 层中枢运行；腿仍占 k 层 osc 槽、用 frac_of(k)——
    /// 量是物理归属，上移改变操作节奏不改资金归属，44课禁令对象=响应量
    /// 错配非物理归属在册）。趋势态下 k 层永不开（重路由不是 fallback）；
    /// k+1 无 alive 中枢/无触发 ⇒ 自然不开（机制预测：负域浅回调在 k+1
    /// 无信号）。单级上移（预注册判据内定理，不递归）；上移路径不查 k+1
    /// 层 trend_row（预注册无此判据——边界条件在结果文档声明）。
    /// 需要 trend_flips 磁带行（runner guard）。
    TrendUpshift,
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
    /// 49课严格形式（操作判据严格形式审计，2026-06-11）："一旦出现第三类
    /// 卖点，就不能回补了"——osc 腿在锚中枢死亡（三卖终结）时不强制回补，
    /// 保持卖出状态直到边界触线（更低处）或 master 清算。false = 在册 P5
    /// 行为（死亡即市价回补，O0≡P5 零接触面）。
    pub osc_sell3_no_recover: bool,
    /// 中枢上移出口（僵尸腿消灭任务 2026-06-12）：新中枢形成且新 ZD > 锚中枢
    /// ZG（中枢向上移动）⇒ osc 空腿立即回补。原文依据：49课"中枢向上移动时
    /// 就应该满仓"——震荡短差赌的是回 ZD，中枢整体上移 = 该赌注结构性失败，
    /// 继续持有即僵尸腿（在册诊断：type3 confirmed 在单边趋势中回抽不发生 ⇒
    /// 出口2不触发，僵尸腿最终归宿 = master 强平，占总亏 62-82%）。本出口
    /// 补的正是 type3 出口失效的情况——这是原文要求的结构性出口，不是外加门。
    /// 方向对称声明（声明=能力）：osc 腿构造性 short-only（开腿条件
    /// c ≥ ZG ∧ sub_sell），"下降趋势 osc 多腿"在类型层不存在，对称分支
    /// （中枢下移→平多）无承载对象。
    /// false = 在册 P5 行为（O0≡P5 零接触面——默认值锚定 P5 逐字，与
    /// osc_sell3_no_recover 同先例：原文严格形式在变体层开启）。
    pub osc_shift_close: bool,
    /// 41课门（域腿形态，osc 加门任务 2026-06-11）：osc 开腿前检查直接父级别
    /// （ladder+1）向上走势衰竭状态——相邻同向（Up）段创新高 ∧ 当前段窗口内
    /// 无盘整背驰 = 上涨趋势未完 = 拒开逆向短差。原文依据：41课"大级别走势
    /// 没有任何衰竭迹象时参与反向小级别买卖点是刀口舔血"（"一切"包括域腿）；
    /// 49课"中枢向上移动时就应该满仓"（满仓 = 不做逆向短差）。与 rev_l41_gate
    /// （REV 主腿）同判据同追踪器（up_unexhausted(k+1)），消费点不同——REV 门
    /// 挡段终结反向腿，本门挡中枢震荡 c≥ZG 卖出腿（强趋势中价格不回 ZD →
    /// 中枢死亡 → 高位强制买回的结构性亏损路径）。n_osc_l41_rejects 可观测。
    /// false = 在册 P5 行为（O0≡P5 零接触面）。需要 D3 dir_flips 磁带行。
    pub osc_l41_gate: bool,
    /// osc 操作域（操作对象定义严格化，2026-06-11 任务）：ConsolidationOnly
    /// = 仅盘整走势的中枢里做震荡短差（38课域内），趋势走势的中枢不开
    /// （49课满仓要求）。判据 = 锚中枢所在层（k）尾 move kind（trend_row[k]，
    /// trend_flips 磁带行直接消费——17课趋势定义 ≥2 同向中枢的引擎读数）。
    /// Any = 在册 P5 行为（O0≡P5 零接触面）。
    pub osc_domain: OscDomain,
    /// H1 candidate 冻结（osc 白名单消除任务 2026-06-12）：candidate 离开段
    /// 未决期间 osc 不开腿。原文依据：49课行52"中枢完成后的向上移动时的
    /// 差价是不能做的……前提是中枢震荡依旧"；行68 给出当下判据——次级别
    /// 走势**离开**中枢即启动"向上移动"语义（candidate type3，不等回抽
    /// 确认）。在册缺口：is_frozen 挂 confirmed Buy3，单边趋势中回抽不发生
    /// ⇒ candidate 永不确认 ⇒ 禁令窗口内开腿门恒开（僵尸腿全在此窗口开出，
    /// 在册诊断：尾部强平腿占总亏 62-82%）。解冻 = 价格回边界内（买侧
    /// c<ZG / 卖侧 c>ZD——38课答疑"能回到中枢就不是第三类买点"）、中枢
    /// 死亡或新中枢形成。**只挡开腿，永不挡闭腿**（僵尸腿教训）。纯结构
    /// 零参数零标的依赖——白名单消除候选 H1。
    /// false = 在册 P5 行为（O0≡P5 零接触面）。
    pub osc_candidate_freeze: bool,
    /// H2 力度收敛门（osc 白名单消除任务 2026-06-12，设计预注册
    /// analysis/h2_strength_convergence_design.md）：osc 开腿前比较锚中枢
    /// 最近两次**向上离开段力度**（excursion = H1 窗口内 max(c) − 当时 ZG
    /// ——力度=价格振幅在册口径的直读，复用 H1 窗口机器零 surfacing）。
    /// 原文依据：49课行38"中枢震荡都是逐步收敛的……如果继续是中枢震荡，
    /// 后面的向下离开力度一定比前一个小。当然，还有些特殊的中枢震荡，会
    /// 出现扩张的情况……最终形成第三类卖点"；行52"用中枢震荡力度判断的
    /// 方法，完全可以避开"。判据（零参数，开腿时刻只读历史）：(1) 向上
    /// 力度历史 <2 条 ⇒ 拒（新生保守默认：无"震荡依旧"证据）；(2) 最近
    /// > 前次 ⇒ 拒（扩张 ⇒ 三类点预警）；(3) 收敛 ⇒ 放行。判据时点 =
    /// 开腿时刻本身——覆盖 H1 candidate 窗口的"窗口前"盲区（533号 BRN
    /// 裁决：负域僵尸主体开在 candidate 出现之前）。只挡开腿，闭腿零接触
    /// （僵尸腿教训）。false = 在册 P5 行为（O0≡P5 零接触面）。
    pub osc_strength_gate: bool,
    /// H4 滚动振幅准入（osc 白名单消除任务 2026-06-12，fallback 方向）：
    /// osc 开腿前提 = 锚层典型中枢相对振幅 θ_q（DepthRef 因果滚动中位数，
    /// window=DEPTH_REF_WINDOW 个中枢/min_obs=SUB_COST_MIN_OBS/q=SUB_COST_Q，
    /// 零前瞻）≥ theta_cost_k × friction_rt。原文依据：38课行32"选择一组
    /// 历史上某级别平均震荡幅度最大的股票，不断操作下去，这样的效果更好"
    /// ——震荡幅度是事前可读的结构量选股条款；35课行30"级别越小，平均的
    /// 买卖点间波幅也越小……不足以让交易成本、交易误差等相对买卖点间波幅
    /// 足够小，这样的操作，从长期的角度看，是没有意义的"——准入下限 =
    /// k 倍往返摩擦。与 θ 深度门（theta_mode，逐腿锚振幅判深度）不同：
    /// 本门判"该标的该层该时段适不适合做 osc 短差"（级别×时段可操作性，
    /// sub_cost_gate/ledger_cost_gate 的 osc 准入形态——同判据同常数）。
    /// 判据从回测盈亏符号（白名单，不可在线不可证伪）换成结构量×物理
    /// 摩擦（事前可读）。参照不可定义（warm-up）⇒ 保守拒绝独立计数
    /// （"不静默放行"先例）。只挡开腿，闭腿零接触（僵尸腿教训）。
    /// false = 在册 P5 行为（O0≡P5 零接触面）。
    pub osc_amp_gate: bool,
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
    /// Seq38n：38课位置分支入主 REV 腿闭腿集（reason=11；严格形式审计 §4e
    /// 唯一缺失项的移植，2026-06-11）。原文 38课:36（向下段镜像）："1、不
    /// 跌破第一段低点，重新买入"；答疑:296"这个'不跌破'是靠次级别判断吗？
    /// ——对，需要该段内部结构的确认"⇒ 判据 = low_since_open > seg1_low
    /// ∧ sub_confirm(k, Buy)（非纯几何触线）。seg1_low = 开腿时冻结的
    /// rev_run_low（上次配对闭腿以来 close 运行最低——第一段起点低点的
    /// close 分辨率读数，Sequence38 子腿 seg1_low 的主腿同构）。
    /// 优先序：事件证据（T7/T6/T5/T2c）之后、几何触线（ZG/ZD）之前——
    /// 结构确认强于几何弱于事件本体。仅 rev_paired 消费（runner guard）。
    pub rev_seq_nobreak: bool,
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
    /// 结论有效域限近似实现本身。
    /// Sequence38 = 段间盘整背驰严格形式（见 SubMode docstring）。
    /// 仅 rev_sub_depth > 0 时消费。
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
    /// master 出场模式（见 ExitMode docstring）。Signal = 在册 sell1 即出。
    pub exit_mode: ExitMode,
    /// master 入场级别下限（2026-06-11 入场侧调研：BTC 死因 = segment 级
    /// 出场→重入循环 1254 次每次期望为负——exit_reasons 直方图显示 86% 入场
    /// 落在 ladder=2）。FLAT 态只对 ≥ 本值的 confirmed buy1 布防（ARMED 升级
    /// 与区间套次级别确认零改动——次级别确认本来就在 arm_ladder 之下）。
    /// 出场判据级别 = entry_ladder（不变式），故本轴是**对称**升级：入场出场
    /// 同级上移，不违反 Climb 否证的"出场判据级别不可高于入场级别"定理。
    /// 0 = 无约束（在册行为，O0≡P5 零接触面：扫描下界 = FIRST_BSP_LADDER）。
    /// 语义映射：3 = move(L1) 级、4 = recL2 级。高层 buy1 未涌现期不布防
    /// （首笔入场 bar 合法变晚——级别涌现是数据性质，不提供低层代理降级）。
    pub entry_min_ladder: usize,
    /// entry 级完整声部（嵌套递归并发赋格 E 轴，2026-06-12 任务）：
    /// active_levels 从 [floor, entry) 扩为 [floor, entry]——entry 级与其余
    /// 声部全同构（main/osc/rev 三类腿；"每一层的结构完全同构"）。
    /// 44课级别门的嵌套形态：master 满仓入场保暴露（positional 扁平分层
    /// 否证的死因矫正——入场侧不拆），entry 级 sell1 在 master 被出场门
    /// （HoldTrend/Emergent 趋势延续证据）拦截持仓时，由 voice@entry 承载
    /// 为配额反向腿（44课:50"先出一部分……如果没有出现上一段所说的情况，
    /// 就可以回补，权当弄了一个短差"的逐字形态）。master 真实出场时
    /// close_position 强制清腿在册逻辑自动覆盖（义务闭腿）。
    /// 要求 exit_mode ≠ Signal（Signal 下 master 在 voice 之前清仓，本轴
    /// 是死配置——runner guard 拒绝）。false = 在册行为（O0≡P5 零接触）。
    /// **L2 判决（2026-06-12，nested_recursive_fugue_results.md）：双标的
    /// 否证**——OKLO −513pp / BTC −410pp（entry 级 main/rev 腿在强趋势中
    /// 反向段不回撤 ⇒ 短差失血，与 osc 僵尸同构）。保留为否证在案配置位。
    pub entry_voice: bool,
    /// voice 消费方式（见 VoiceMode docstring）。Fsm = 在册行为。
    /// Ledger ⇒ main 腿/FSM 机制位必须全关（runner guard 强制——账本路径
    /// 不消费这些位，保留非关闭值即声明膨胀）。
    pub voice_mode: VoiceMode,
    /// 账本 voice 卖侧词汇收缩：仅 confirmed Sell1 开腿（递归建仓 s1 臂
    /// 教训的镜像——Sell2/Sell3 是结构内确认/中枢离开点，在强趋势中开腿
    /// 是纯打断）。仅 voice_mode=Ledger 路径消费（guard 强制）。
    pub ledger_sell_t1_only: bool,
    /// 有门的账本——对象域门（2026-06-12 并发赋格门层任务；VL 实验开放轴2，
    /// deep_think §15 门层补全）。开腿前提 = 该层尾 move kind==Consolidation
    /// （trend_row[k]==false）：38课震荡短差的操作对象是确立中枢内的反复震荡，
    /// 趋势走势中该对象从定义上不存在（osc_domain=ConsolidationOnly 同范畴
    /// ——对象域状态范畴，判决：对象域胜点态门第五例）。VL 裸账本 OKLO
    /// −1146pp 的死因（强趋势中噪声短差失血）的直接修复位。闭腿不受门约束
    /// （卖了必须买回——僵尸腿教训）。需要 trend_flips 磁带行（runner guard）。
    /// 仅 voice_mode=Ledger 路径消费。
    pub ledger_domain_gate: bool,
    /// 有门的账本——成本门（35课："交易成本+交易误差相对波幅不够小的级别，
    /// 长期操作没有意义"）。开腿前提 = 该层典型中枢振幅 θ_q（DepthRef 因果
    /// 滚动中位数，零前瞻）≥ theta_cost_k × friction_rt——振幅不足的级别
    /// 配额自动归零（设计 §5.4"比值不足者配额为 0（声部不开）"的逐 bar
    /// 实现 = 声部的经济生命周期）。参照不可定义 ⇒ 保守拒绝并独立计数
    /// （sub_cost_gate 同构）。仅 voice_mode=Ledger 路径消费。
    pub ledger_cost_gate: bool,
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
    /// 笔级分型锚（38课程式的近似实现，非严格形式——严格形式 =
    /// Sequence38）：方向行翻转为进出点，存在论下限 = bi 级。
    Fractal,
    /// 段间盘整背驰锚（38课程式的**严格形式**，2026-06-11 任务）。
    /// 操作对象 = REV 窗口内本级别的段序列（38课:36 同级别分解程式逐条）：
    /// - **开腿（先卖）**：本级别或次级别的盘整背驰卖点
    ///   （DivKind::Consolidation × side()==Sell）——"向上的第一段走势类型，
    ///   根据其内部结构可以判断其背驰或盘整背驰结束点，先卖出"；
    ///   开腿时冻结 seg1_low = 节点激活/上次买回以来的 close 运行最低
    ///   （第一段起点低点的 close 分辨率读数）。
    /// - **闭腿（买回）三岔**：
    ///   (1) 段间盘整背驰买点（DivKind::Consolidation × side()==Buy @ 本级别）
    ///       ——"跌破第一段低点，如果与第一段前的向下段形成盘整背驰，也重新
    ///       买入"；不跌破时同一事件同样是第二段完成证据，无条件入闭腿集；
    ///   (2) 不跌破第一段低点 ∧ 次级别结构确认第二段完成（38课答疑:296
    ///       "这个'不跌破'是靠次级别判断吗？——对，需要该段内部结构的确认"
    ///       ⇒ sub_confirm(k, Buy)，非纯几何触线）；
    ///   (3) 新的下跌背驰（DivKind::Trend × Buy ∨ confirmed Buy1 @ 本级别）
    ///       ——"否则继续观望，直到出现新的下跌背驰"（观望出口：新下跌背驰
    ///       = 新一轮程式的买点本体）。
    /// 方向恒 Short（"向下段的运作刚好相反，是先卖后买"——同 Fractal 论据）；
    /// 买回后 seg1_low 复位重新累计（38课中间循环：程式在 REV 窗口内反复）。
    /// 与 Fractal 的判据差异 = 近似 vs 严格的唯一轴：分型翻转（笔级方向行）
    /// vs 段间盘整背驰（div 事件流 + 次级别结构确认）。
    /// 存在论下限 = FIRST_BSP_LADDER（div 事件流的承载下界，同 Zhongshu）。
    Sequence38,
    /// 反向线段腿（嵌套递归并发赋格的**严格形式**，2026-06-12 任务；
    /// 设计 `analysis/nested_recursive_fugue_design.md` §2.3/§2.5）。
    /// 操作对象 = 父腿反向走势窗口内、本级别的反向走势类型（用户结构第3层：
    /// "反向线段内部的反向线段，又在次次级别是一个直接的多单"），进出恒用
    /// **本级别自己的买卖点**（"每一层都通过买卖点操作"）：
    /// - **开腿** = 本级别反向段起点买卖点：奇数深度（Long）confirmed Buy1 ∨
    ///   盘背买；偶数深度（Short）镜像——符号交替塔 (−1)ⁿ；
    /// - **闭腿** = 本级别反向段终点买卖点（开腿的镜像侧）：confirmed
    ///   Sell1/Buy1 ∨ 盘背卖/买 ∨ confirmed type3（中枢离开终结段）。
    ///   恒 confirmed——candidate 不是原文操作点（not6 先例）。
    /// 与三个在册模式的差异（各自否证死因的逐点拆除）：
    /// - vs Zhongshu（O_sub0 两票否证）：**无"存活中枢"前置、无 ZG/ZD 边界
    ///   与振幅域门**——父腿前提（反向段运行）不再否定子腿前提；
    /// - vs Fractal（OKLO −73.8pp，近似实现）：信号源 = 本级别 BSP/盘背
    ///   事件流，非笔级方向行翻转（a0 近似死因矫正）；
    /// - vs Sequence38：方向遵守符号交替塔（38课程式的方向镜像），非恒 Short。
    /// 递归终止三范畴：深度预算 ∧ 存在论下限 FIRST_BSP_LADDER（笔 = a0，
    /// 62/77/78课）∧ 35课成本门（sub_cost_gate：θ_q ≥ sub_cost_k×friction）。
    CounterSeg,
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
            osc_sell3_no_recover: false,
            osc_shift_close: false,
            osc_l41_gate: false,
            osc_domain: OscDomain::Any,
            osc_candidate_freeze: false,
            osc_strength_gate: false,
            osc_amp_gate: false,
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
            rev_seq_nobreak: false,
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
            exit_mode: ExitMode::Signal,
            entry_min_ladder: 0,
            entry_voice: false,
            voice_mode: VoiceMode::Fsm,
            ledger_sell_t1_only: false,
            ledger_domain_gate: false,
            ledger_cost_gate: false,
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
        "V1" => Some(OrganicConfig {
            rev_mode: true,
            ..base
        }),
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
            theta_mode: ThetaMode::AdaptiveQuantile {
                q: 0.25,
                window: 50,
                min_obs: 10,
            },
            rev_l41_gate: true,
            ..variant("V2of").expect("V2of 在上方注册")
        }),
        "V2oa50" => Some(OrganicConfig {
            theta_mode: ThetaMode::AdaptiveQuantile {
                q: 0.50,
                window: 50,
                min_obs: 10,
            },
            ..variant("V2of").expect("V2of 在上方注册")
        }),
        // ── 并发赋格最小可验证实验（2026-06-11；deep_think Part II §17
        // 开放问题3 的最小化形态）。VL = 裸账本 voice：每个 confirmed BSP
        // 直接饱和执行（Sell 开=卖出 frac_k、Buy 闭=买回），无相位/无锚/
        // 无门/无配对。master Full 入场 + Signal 出场在册路径零接触——
        // 与 V2oa25（FSM voice）同 master 同磁带可比，差额 = FSM 控制状态
        // + 门谓词的联合信息含量。main 腿不消费的位全部显式关闭
        // （声明=能力；runner guard 强制）。──
        "VL" => Some(OrganicConfig {
            voice_mode: VoiceMode::Ledger,
            open_kinds: vec![],
            hard_type3: false,
            pre_type3: false,
            center_gate: false,
            same_center_close: false,
            osc_mode: false,
            ..base
        }),
        // VLs1：卖侧词汇收缩消融（仅 confirmed Sell1 开）——递归建仓 s1 臂
        // 的镜像：Sell2/Sell3 是结构内确认/中枢离开点，非趋势终结信号。
        // VL−VLs1 差额 = 卖侧词汇宽度的独立因果。
        "VLs1" => Some(OrganicConfig {
            ledger_sell_t1_only: true,
            ..variant("VL").expect("VL 在上方注册")
        }),
        // ── 有门的账本消融矩阵（2026-06-12 并发赋格门层任务；基线 = VL 裸
        // 账本（00 格在册：OKLO −1146pp / BRN +24pp）。门 = 无消费记忆的
        // 结构谓词（deep_think §13 判别标准），只挡开腿不挡闭腿。
        // 预注册判据（OKLO）：VLg > VL ⇒ 门层携带信息（修复量 = Δ）；
        // VLg ≥ V2oa25 ⇒ "voice=有门的账本"形态确认（FSM 相位是冗余缓存）；
        // VL < VLg < V2oa25 ⇒ 信息主要在门，残差 = 相位的不可消除部分。──
        // VLd：仅对象域门（趋势态不开短差——OKLO 死因的单因修复位）。
        "VLd" => Some(OrganicConfig {
            ledger_domain_gate: true,
            ..variant("VL").expect("VL 在上方注册")
        }),
        // VLc：仅成本门（35课振幅/摩擦——级别配额归零的经济生命周期）。
        "VLc" => Some(OrganicConfig {
            ledger_cost_gate: true,
            ..variant("VL").expect("VL 在上方注册")
        }),
        // VLg：双门（有门的账本完整形态）。
        "VLg" => Some(OrganicConfig {
            ledger_domain_gate: true,
            ledger_cost_gate: true,
            ..variant("VL").expect("VL 在上方注册")
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
        // ── 38课严格形式（2026-06-11；基线 = V2of / 近似对照 = V2ofF1）──
        // Seq1 = REV 窗口内 k−1 段间盘整背驰短差腿（sub_mode=Sequence38，
        // 唯一差异轴 vs V2ofF1 = 进出判据：分型翻转 → 段间盘背 + 不跌破
        // 第一段低点 + 新下跌背驰观望出口）。分型模式的否证结论（OKLO 负/
        // BRN 正）有效域限近似实现——本变体是38课本体的直接判决位。
        "V2ofSeq1" => Some(OrganicConfig {
            rev_sub_depth: 1,
            sub_mode: SubMode::Sequence38,
            ..variant("V2of").expect("V2of 在上方注册")
        }),
        // ── master 入场侧递归建仓（2026-06-11；基线 = V2oa25，单轴 entry_mode）──
        // base_frac 三档：0.2（深递归，最多 3 档追加）/ 1/3 / 0.5（浅递归）。
        // voice（V2oa25 REV+域腿）逐位不碰——唯一差异轴是 master 入场过程。
        "V2oa25_rec" => Some(OrganicConfig {
            entry_mode: EntryMode::Recursive { base_frac: 0.2 },
            ..variant("V2oa25").expect("V2oa25 在上方注册")
        }),
        "V2oa25_rec3" => Some(OrganicConfig {
            entry_mode: EntryMode::Recursive {
                base_frac: 1.0 / 3.0,
            },
            ..variant("V2oa25").expect("V2oa25 在上方注册")
        }),
        "V2oa25_rec5" => Some(OrganicConfig {
            entry_mode: EntryMode::Recursive { base_frac: 0.5 },
            ..variant("V2oa25").expect("V2oa25 在上方注册")
        }),
        // ── master 出场状态驱动消融（2026-06-11；基线 = V2oa25，单轴 exit_mode）──
        // ht = HoldTrend（sell1 ∧ 父级别趋势衰竭才出）；ho = HighestOnly
        // （只认 max_ladder 的 sell1）。voice 短差（V2oa25 REV+域腿）逐位不碰。
        "V2oa25_ht" => Some(OrganicConfig {
            exit_mode: ExitMode::HoldTrend,
            ..variant("V2oa25").expect("V2oa25 在上方注册")
        }),
        // ── 操作判据严格形式替换（2026-06-11 审计任务；基线 = V2oa25_ht）──
        // not6：去 candidate Buy3 预动作（pre_type3=false，T6 预回补 + main 腿
        // pre 闭腿一并关闭）。原文无 candidate 操作点：三买 = 回抽完成且不回
        // 中枢的确认（38课答疑:160/180"能回到中枢就不是第三类买点……第二次
        // 没回，那才是第三类买点"）——回抽进行中的预判不是原文判据；在册
        // 诊断 t6 预逃逸槽胜率 ≈30% 唯一大负槽（organic fugue 逐笔诊断）。
        "V2oa25_ht_not6" => Some(OrganicConfig {
            pre_type3: false,
            ..variant("V2oa25_ht").expect("V2oa25_ht 在上方注册")
        }),
        // o3s：49课严格形式——"在围绕中枢差价时……前提是中枢震荡依旧，一旦
        // 出现第三类卖点，就不能回补了"。在册 P5 行为（锚中枢死亡即市价强制
        // 回补）与原文直接冲突；严格形式 = 死亡不回补，保持卖出状态直到
        // 旧边界触线（更低处兑现）或 master 清算（n_osc_dead_holds 可观测）。
        "V2oa25_ht_o3s" => Some(OrganicConfig {
            osc_sell3_no_recover: true,
            ..variant("V2oa25_ht").expect("V2oa25_ht 在上方注册")
        }),
        // sc：中枢上移出口（僵尸腿消灭任务 2026-06-12）。osc 闭腿在册只有
        // ZD 触线（正常兑现）与 master 强平（僵尸腿最终归宿）；type3 强闭
        // （出口2）在单边趋势中回抽不发生 ⇒ 不触发。本出口 = 49课"中枢向上
        // 移动时就应该满仓"——新中枢 ZD > 锚中枢 ZG ⇒ 旧中枢的 osc 空腿
        // 立即回补（中枢向上移动 = 回 ZD 赌注结构性失败）。预注册判据：
        // BTC/GC/ES 僵尸腿被消灭 → osc 亏损减少；OKLO/BRN 正常腿不受影响
        // （中枢不移动 = 出口不触发）；十标的不需要白名单——同一逻辑在
        // 所有 regime 自适应。
        "V2oa25_ht_sc" => Some(OrganicConfig {
            osc_shift_close: true,
            ..variant("V2oa25_ht").expect("V2oa25_ht 在上方注册")
        }),
        // o41：osc 域腿加 41课门（跑输 BH 根因任务 2026-06-11：BTC/GC/ES 凶手
        // = osc 腿在强趋势中逆向卖出 → 价格不回 ZD → 中枢死亡 → 高位强制买回；
        // 41课门在册只挂 REV 腿，osc 开腿无门）。判据 = rev_l41_gate 同形
        // （up_unexhausted(k+1)），消费点 = step_osc 开腿分支。预注册判据：
        // BTC/GC/ES 的 osc 亏损大幅减少；OKLO/BRN 正 osc 贡献近不变
        // （趋势衰竭时门放行，震荡 regime 中正常开腿）。
        "V2oa25_ht_o41" => Some(OrganicConfig {
            osc_l41_gate: true,
            ..variant("V2oa25_ht").expect("V2oa25_ht 在上方注册")
        }),
        // co：osc 操作域严格化（2026-06-11 任务；o41 点态门否证后的状态范畴
        // 形态——门拦截是腿延迟非腿消灭，对象域定义才是腿消灭）。只在盘整
        // 走势的中枢里做震荡短差（38课域内），趋势走势的中枢不开（49课
        // 满仓 / 26课单边不做短线）。判据 = trend_row[k]（锚中枢所在层尾
        // move kind，trend_flips 磁带行）。预注册判据：BTC/GC/ES osc 亏损
        // 消除或大幅减少（趋势中从定义上不开）；OKLO/BRN osc 正贡献保持
        // （盘整中正常开）；十标的全部 ≥ 基线。
        "V2oa25_ht_co" => Some(OrganicConfig {
            osc_domain: OscDomain::ConsolidationOnly,
            ..variant("V2oa25_ht").expect("V2oa25_ht 在上方注册")
        }),
        // scco：sc×co 双轴组合（2026-06-12 任务；sc/co 单轴判决后的正交性
        // 验证位）。sc 与 co 作用面正交：co 限制开腿对象域（趋势走势的中枢
        // 从定义上不开 osc），sc 收紧闭腿出口（盘整域内开出的腿在中枢上移
        // 时立即回补——僵尸残腿消灭）。预注册判据：BTC/GC/ES 上 scco ≥ sc
        // （co 额外消除趋势域开腿）；BRN 上 scco > co（sc 对盘整域内僵尸
        // 残腿也有效）；理想情况十标的全正不需要白名单。
        "V2oa25_ht_scco" => Some(OrganicConfig {
            osc_shift_close: true,
            osc_domain: OscDomain::ConsolidationOnly,
            ..variant("V2oa25_ht").expect("V2oa25_ht 在上方注册")
        }),
        // h1：candidate 冻结（osc 白名单消除任务 2026-06-12——零参数内生
        // 判据消除 per-asset 白名单）。49课行52/60/68：禁令窗口从次级别
        // 走势离开中枢（candidate type3）开始，不等 confirmed；窗口内 osc
        // 不开腿；价格回边界内（回试跌回 = 仍是中枢震荡）解冻。预注册判据
        // （任一不满足即 H1 否证）：(1) OKLO Δosc ≈ +105K（僵尸尾部 3 腿
        // 占总亏 62% 被窗口拦截）；(2) 正域 OKLO/BRN 不恶化（对照：co 在
        // OKLO 恶化）；(3) 负域 BTC/CL/ES osc 亏损大幅缩减。机制差异 vs
        // co：co 删整个趋势态域（深回调盈利腿陪葬），h1 只删离开段未决
        // 窗口（震荡标的窗口被回试快速否定 ⇒ 盈利腿保留）。
        "V2oa25_ht_h1" => Some(OrganicConfig {
            osc_candidate_freeze: true,
            ..variant("V2oa25_ht").expect("V2oa25_ht 在上方注册")
        }),
        // h2：力度收敛门（H1 判决后的下一步——533号 BRN 裁决：负域僵尸主体
        // 开在 candidate 窗口出现**之前**，H1 结构性覆盖不到；H2 判据时点
        // = 开腿时刻本身，只读已完成的历史离开段力度，恰好覆盖"窗口前"
        // 盲区）。预注册判据（h2_strength_convergence_design.md §5，任一
        // 不满足按对应轴否证）：(1) 时序靶：基线僵尸尾部腿开腿 bar 被 H2
        // 拒绝率 ≥50%；(2) 负域 BTC/CL/ES Δosc 缩减 ≥50%；(3) OKLO 保护
        // （h1h2 组合）Δosc(h1h2 vs h1) ≥ −10K；(4) 腿消灭 vs 延迟双指标
        // （腿数 + 尾部亏损等量缩减）。
        "V2oa25_ht_h2" => Some(OrganicConfig {
            osc_strength_gate: true,
            ..variant("V2oa25_ht").expect("V2oa25_ht 在上方注册")
        }),
        // h1h2：组合（"H1 管离开后、H2 管离开前"——研究 §5 预注册执行序；
        // 判据3 否证时预案 = H1(正域)/H2(负域) 对偶单边轴并集）。
        "V2oa25_ht_h1h2" => Some(OrganicConfig {
            osc_candidate_freeze: true,
            osc_strength_gate: true,
            ..variant("V2oa25_ht").expect("V2oa25_ht 在上方注册")
        }),
        // h4：滚动振幅准入（H2 否证关闭后的 fallback 轴——研究 §4 H4，
        // 38课行32+35课行30）。判据 = 锚层典型中枢相对振幅 θ_q（DepthRef
        // 因果滚动 P50，50中枢窗/min_obs=10，零前瞻）≥ theta_cost_k ×
        // friction_rt（默认 2×10bps=0.2%）——结构量×物理摩擦替代回测盈亏
        // 符号白名单。预注册判据（任一不满足按对应轴否证）：(1) 负域
        // BTC/CL/ES osc 亏损缩减 ≥50%；(2) 正域 OKLO 不恶化（OKLO 振幅
        // P50≈0.93% ≫ 0.2% 门槛，门应近零拦截）；(3) 零摩擦口径下 BRN
        // （振幅 P50≈0.10% < 门槛）osc 腿大减是判据的诚实后果而非否证——
        // BRN 零摩擦 osc 盈利在 10bps 真实摩擦下是否存活由腿均盈利 vs
        // 摩擦水平裁决（35课语义：振幅不足的级别长期操作没有意义）；
        // (4) 双拒因（noref/thin）逐事件可分离（G3）。
        "V2oa25_ht_h4" => Some(OrganicConfig {
            osc_amp_gate: true,
            ..variant("V2oa25_ht").expect("V2oa25_ht 在上方注册")
        }),
        // h3：级别上移（osc 白名单消除任务 2026-06-12——26课行183"最好别
        // 按1分钟弄，5分钟甚至更长都可以"）。趋势态下 osc 重路由到 k+1 层
        // 中枢（删除→重路由：co 的趋势态判据 + 26课的级别上移响应）。
        // 机制差异 vs co：co 在趋势态删腿（深回调盈利腿陪葬，OKLO 恶化
        // 在册）；h3 在趋势态换床位——正域深回调在 k+1 级别中枢仍触发
        // （盈利腿保留且单腿振幅更大），负域浅回调在 k+1 无信号（自然
        // 不开）。预注册判据（研究 §4 H3，任一不满足按对应轴否证）：
        // (1) 正域 OKLO/BRN 不恶化（h3 ≥ co 的正域表现——盈利腿经 k+1
        // 通道保留）；(2) 负域 BTC/CL/ES osc 亏损大幅缩减（量级对照 co）；
        // (3) 风险预注册：k+1 中枢稀疏 ⇒ 腿数大减（n_osc_upshift_open
        // 可观测）——若正域收益被频率损失吃掉（OKLO h3 < h1）按增强轴
        // 否证；(4) G1 全域判据：五标的全部 ≥ 基线（不需要白名单）。
        "V2oa25_ht_h3" => Some(OrganicConfig {
            osc_domain: OscDomain::TrendUpshift,
            ..variant("V2oa25_ht").expect("V2oa25_ht 在上方注册")
        }),
        // ── 38课位置分支移植（2026-06-11 任务；审计 §4e 唯一缺失项；
        //    基线 = V2oa25_ht。Sequence38 子腿 L2 全正（OKLO+10.2/BRN+4.1pp）
        //    后的主腿判决位——三臂分解组合的两条轴 ──
        // nb：主 REV 腿闭腿集加入"不跌破第一段低点 × 次级别确认"（单轴）。
        "V2oa25_ht_nb" => Some(OrganicConfig {
            rev_seq_nobreak: true,
            ..variant("V2oa25_ht").expect("V2oa25_ht 在上方注册")
        }),
        // seq1：Seq1 子腿（REV 窗口内 k−1 段间盘背短差，sub_mode=Sequence38）
        // 移上 ht 基线（V2ofSeq1 的判决基线是 V2of——ht 出场下子腿增量待判）。
        "V2oa25_ht_seq1" => Some(OrganicConfig {
            rev_sub_depth: 1,
            sub_mode: SubMode::Sequence38,
            ..variant("V2oa25_ht").expect("V2oa25_ht 在上方注册")
        }),
        // nbseq1：全组合（主腿位置分支 + Seq1 子腿——38课程式两个层位同开）。
        "V2oa25_ht_nbseq1" => Some(OrganicConfig {
            rev_seq_nobreak: true,
            ..variant("V2oa25_ht_seq1").expect("V2oa25_ht_seq1 在上方注册")
        }),
        "V2oa25_ho" => Some(OrganicConfig {
            exit_mode: ExitMode::HighestOnly,
            ..variant("V2oa25").expect("V2oa25 在上方注册")
        }),
        // em = Emergent（出场级别 = 持仓走势的涌现级别归属，方向行逐层
        // 向上递归）；emht = Emergent + HoldTrend 组合（归属级别 sell1 ∧
        // 动态父级别趋势衰竭确认）。voice 短差（V2oa25 REV+域腿）逐位不碰。
        "V2oa25_em" => Some(OrganicConfig {
            exit_mode: ExitMode::Emergent { hold_trend: false },
            ..variant("V2oa25").expect("V2oa25 在上方注册")
        }),
        "V2oa25_emht" => Some(OrganicConfig {
            exit_mode: ExitMode::Emergent { hold_trend: true },
            ..variant("V2oa25").expect("V2oa25 在上方注册")
        }),
        // ── 嵌套递归并发多重赋格（NRF，2026-06-12 任务；设计
        // analysis/nested_recursive_fugue_design.md + positional 否证矫正）。
        // 三条正交轴长在 V2oa25_emht（在册最优出场基座）上。
        // **L2 判决（nested_recursive_fugue_results.md）**：E 轴双标的否证
        // （OKLO −513/BTC −410pp）；S 轴 regime 函数（OKLO +110pp/BTC −12pp，
        // 深度2 在 BTC 真实涌现 288 腿）；Q 轴第六例 regime 分裂
        // （OKLO +2301% 新高 / BTC −105% 爆仓）。保留为否证在案配置位。──
        "NRF1" => Some(OrganicConfig {
            entry_voice: true,
            ..variant("V2oa25_emht").expect("V2oa25_emht 在上方注册")
        }),
        // S 轴（NRF2 = NRF1 + 反向线段子腿严格形式）：深度2 = 数据界
        // （设计 §4.3），符号交替塔 (−1)ⁿ，开闭恒用本级别买卖点，
        // 35课成本门 = 递归经济终止（sub_cost_gate）。
        "NRF2" => Some(OrganicConfig {
            rev_sub_depth: 2,
            sub_mode: SubMode::CounterSeg,
            sub_cost_gate: true,
            ..variant("NRF1").expect("NRF1 在上方注册")
        }),
        // Q 轴（NRF2q = NRF2 + 配额涌现候选 B）：40课结构规模——配额 ∝
        // 该层存活中枢相对振幅占比（26课"级别=买卖量"的连续化；与"绝对
        // 不加仓"无冲突：配额只定反向腿卖出规模，不对多头仓位再平衡）。
        "NRF2q" => Some(OrganicConfig {
            sizing: Sizing::Structure,
            ..variant("NRF2").expect("NRF2 在上方注册")
        }),
        // Q 轴单因隔离臂（NRF2q 是 E×S×Q 合取——本臂隔离 Sizing::Structure
        // 在无 entry 声部/无子腿时的独立贡献）。
        "V2oa25_emht_q" => Some(OrganicConfig {
            sizing: Sizing::Structure,
            ..variant("V2oa25_emht").expect("V2oa25_emht 在上方注册")
        }),
        // BTC 域 scco 合成臂（osc 白名单在册：BTC 负域须 sc+co，否则 osc
        // 失血掩盖出场轴效应；emht×scco 合测 = btc_vs_bh 解法假设3，
        // L2 实测仅 +7.5pp 近零）。
        "V2oa25_emht_scco" => Some(OrganicConfig {
            osc_shift_close: true,
            osc_domain: OscDomain::ConsolidationOnly,
            ..variant("V2oa25_emht").expect("V2oa25_emht 在上方注册")
        }),
        "NRF1_scco" => Some(OrganicConfig {
            osc_shift_close: true,
            osc_domain: OscDomain::ConsolidationOnly,
            ..variant("NRF1").expect("NRF1 在上方注册")
        }),
        "NRF2_scco" => Some(OrganicConfig {
            osc_shift_close: true,
            osc_domain: OscDomain::ConsolidationOnly,
            ..variant("NRF2").expect("NRF2 在上方注册")
        }),
        "NRF2q_scco" => Some(OrganicConfig {
            osc_shift_close: true,
            osc_domain: OscDomain::ConsolidationOnly,
            ..variant("NRF2q").expect("NRF2q 在上方注册")
        }),
        // ── master 入场级别下限（2026-06-11；基线 = V2oa25，单轴 entry_min_ladder）──
        // e3 = move(L1) 级入场（BTC 在册分布 188 笔）/ e4 = recL2 级（17 笔，
        // 稀疏对照臂）；ht 组合 = 与新默认候选 HoldTrend 的合取。出场级别
        // 自动 = 入场级别（对称升级）。voice（V2oa25 REV+域腿）逐位不碰。
        "V2oa25_e3" => Some(OrganicConfig {
            entry_min_ladder: 3,
            ..variant("V2oa25").expect("V2oa25 在上方注册")
        }),
        "V2oa25_e3ht" => Some(OrganicConfig {
            entry_min_ladder: 3,
            exit_mode: ExitMode::HoldTrend,
            ..variant("V2oa25").expect("V2oa25 在上方注册")
        }),
        "V2oa25_e4" => Some(OrganicConfig {
            entry_min_ladder: 4,
            ..variant("V2oa25").expect("V2oa25 在上方注册")
        }),
        "V2oa25_e4ht" => Some(OrganicConfig {
            entry_min_ladder: 4,
            exit_mode: ExitMode::HoldTrend,
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
                ThetaMode::AdaptiveQuantile {
                    q,
                    window: 50,
                    min_obs: 10
                }
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
            ThetaMode::AdaptiveQuantile {
                q: 0.25,
                window: 50,
                min_obs: 10
            }
        );
        assert!(cfg.rev_mode && cfg.rev_paired && !cfg.rev_escape_open);
        assert_eq!(cfg.theta_cost_k, 2.0);
        assert_eq!(cfg.friction_rt, 0.001);
        assert_eq!(cfg.rev_sub_depth, 0);
    }

    #[test]
    fn cycle38_close_ablation_variants_single_axis() {
        // 默认/在册：rev_cycle_close=SubAny（C38base 零接触）
        assert_eq!(
            OrganicConfig::default().rev_cycle_close,
            RevCycleClose::SubAny
        );
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
            let normalized = OrganicConfig {
                rev_cycle_close: RevCycleClose::SubAny,
                ..cfg
            };
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
        for (name, frac) in [
            ("V2oa25_rec", 0.2),
            ("V2oa25_rec3", 1.0 / 3.0),
            ("V2oa25_rec5", 0.5),
        ] {
            let cfg = variant(name).unwrap();
            assert_eq!(
                cfg.entry_mode,
                EntryMode::Recursive { base_frac: frac },
                "{name}"
            );
            let normalized = OrganicConfig {
                entry_mode: EntryMode::Full,
                ..cfg
            };
            assert_eq!(format!("{normalized:?}"), format!("{base:?}"), "{name}");
        }
    }

    #[test]
    fn exit_mode_variants_single_axis() {
        // 默认/在册全变体 exit_mode=Signal（O0≡P5 零接触面）
        assert_eq!(OrganicConfig::default().exit_mode, ExitMode::Signal);
        let base = variant("V2oa25").unwrap();
        assert_eq!(base.exit_mode, ExitMode::Signal);
        // 四臂只动 exit_mode 一轴——其余字段与 V2oa25 逐位相同
        for (name, want) in [
            ("V2oa25_ht", ExitMode::HoldTrend),
            ("V2oa25_ho", ExitMode::HighestOnly),
            ("V2oa25_em", ExitMode::Emergent { hold_trend: false }),
            ("V2oa25_emht", ExitMode::Emergent { hold_trend: true }),
        ] {
            let cfg = variant(name).unwrap();
            assert_eq!(cfg.exit_mode, want, "{name}");
            let normalized = OrganicConfig {
                exit_mode: ExitMode::Signal,
                ..cfg
            };
            assert_eq!(format!("{normalized:?}"), format!("{base:?}"), "{name}");
        }
        // HoldTrend 的数据基础前提：V2oa25 的 rev_l41_gate 已要求 D3 行 +
        // 实例化 TrendExhaustion——_ht 继承后两者共用同一追踪器
        assert!(variant("V2oa25_ht").unwrap().rev_l41_gate);
    }

    #[test]
    fn o41_variant_single_axis_on_ht() {
        // 默认/在册基线 osc_l41_gate=false（O0≡P5 零接触面）
        assert!(!OrganicConfig::default().osc_l41_gate);
        let base = variant("V2oa25_ht").unwrap();
        assert!(!base.osc_l41_gate);
        // o41 只动 osc_l41_gate 一轴——其余字段与 V2oa25_ht 逐位相同
        let cfg = variant("V2oa25_ht_o41").unwrap();
        assert!(cfg.osc_l41_gate && cfg.osc_mode && cfg.rev_l41_gate);
        assert_eq!(cfg.exit_mode, ExitMode::HoldTrend);
        let normalized = OrganicConfig {
            osc_l41_gate: false,
            ..cfg
        };
        assert_eq!(format!("{normalized:?}"), format!("{base:?}"));
    }

    #[test]
    fn co_variant_single_axis_on_ht() {
        // 默认/在册基线 osc_domain=Any（O0≡P5 零接触面）
        assert_eq!(OrganicConfig::default().osc_domain, OscDomain::Any);
        let base = variant("V2oa25_ht").unwrap();
        assert_eq!(base.osc_domain, OscDomain::Any);
        // co 只动 osc_domain 一轴——其余字段与 V2oa25_ht 逐位相同
        let cfg = variant("V2oa25_ht_co").unwrap();
        assert_eq!(cfg.osc_domain, OscDomain::ConsolidationOnly);
        assert!(cfg.osc_mode && !cfg.osc_l41_gate);
        assert_eq!(cfg.exit_mode, ExitMode::HoldTrend);
        let normalized = OrganicConfig {
            osc_domain: OscDomain::Any,
            ..cfg
        };
        assert_eq!(format!("{normalized:?}"), format!("{base:?}"));
    }

    #[test]
    fn sc_variant_single_axis_on_ht() {
        // 默认/在册基线 osc_shift_close=false（O0≡P5 零接触面——原文严格
        // 形式在变体层开启，osc_sell3_no_recover 同先例）
        assert!(!OrganicConfig::default().osc_shift_close);
        let base = variant("V2oa25_ht").unwrap();
        assert!(!base.osc_shift_close);
        // sc 只动 osc_shift_close 一轴——其余字段与 V2oa25_ht 逐位相同
        let cfg = variant("V2oa25_ht_sc").unwrap();
        assert!(cfg.osc_shift_close && cfg.osc_mode);
        assert_eq!(cfg.exit_mode, ExitMode::HoldTrend);
        let normalized = OrganicConfig {
            osc_shift_close: false,
            ..cfg
        };
        assert_eq!(format!("{normalized:?}"), format!("{base:?}"));
    }

    #[test]
    fn scco_variant_dual_axis_on_ht() {
        // scco 恰动两轴（sc + co）——归一化两轴后与 V2oa25_ht 逐位相同
        let base = variant("V2oa25_ht").unwrap();
        let cfg = variant("V2oa25_ht_scco").unwrap();
        assert!(cfg.osc_shift_close && cfg.osc_mode);
        assert_eq!(cfg.osc_domain, OscDomain::ConsolidationOnly);
        assert_eq!(cfg.exit_mode, ExitMode::HoldTrend);
        let normalized = OrganicConfig {
            osc_shift_close: false,
            osc_domain: OscDomain::Any,
            ..cfg.clone()
        };
        assert_eq!(format!("{normalized:?}"), format!("{base:?}"));
        // 与两个单轴变体的关系：scco = sc ∪ co（各自归一化另一轴后相等）
        let sc = variant("V2oa25_ht_sc").unwrap();
        let co = variant("V2oa25_ht_co").unwrap();
        let as_sc = OrganicConfig {
            osc_domain: OscDomain::Any,
            ..cfg.clone()
        };
        assert_eq!(format!("{as_sc:?}"), format!("{sc:?}"));
        let as_co = OrganicConfig {
            osc_shift_close: false,
            ..cfg
        };
        assert_eq!(format!("{as_co:?}"), format!("{co:?}"));
    }

    #[test]
    fn h1_variant_single_axis_on_ht() {
        // 默认/在册基线 osc_candidate_freeze=false（O0≡P5 零接触面——
        // 49课禁令窗口严格形式在变体层开启，sc/o3s 同先例）
        assert!(!OrganicConfig::default().osc_candidate_freeze);
        let base = variant("V2oa25_ht").unwrap();
        assert!(!base.osc_candidate_freeze);
        // h1 只动 osc_candidate_freeze 一轴——其余字段与 V2oa25_ht 逐位相同
        let cfg = variant("V2oa25_ht_h1").unwrap();
        assert!(cfg.osc_candidate_freeze && cfg.osc_mode);
        assert_eq!(cfg.exit_mode, ExitMode::HoldTrend);
        let normalized = OrganicConfig {
            osc_candidate_freeze: false,
            ..cfg
        };
        assert_eq!(format!("{normalized:?}"), format!("{base:?}"));
    }

    #[test]
    fn h2_variants_single_axis_on_ht() {
        // 默认/在册基线 osc_strength_gate=false（O0≡P5 零接触面——
        // 49课行38/52 力度收敛门严格形式在变体层开启，h1 同先例）
        assert!(!OrganicConfig::default().osc_strength_gate);
        let base = variant("V2oa25_ht").unwrap();
        assert!(!base.osc_strength_gate);
        // h2 只动 osc_strength_gate 一轴——其余字段与 V2oa25_ht 逐位相同
        let cfg = variant("V2oa25_ht_h2").unwrap();
        assert!(cfg.osc_strength_gate && cfg.osc_mode && !cfg.osc_candidate_freeze);
        assert_eq!(cfg.exit_mode, ExitMode::HoldTrend);
        let normalized = OrganicConfig {
            osc_strength_gate: false,
            ..cfg
        };
        assert_eq!(format!("{normalized:?}"), format!("{base:?}"));
        // h1h2 = h1 ∪ h2（各自归一化另一轴后与单轴变体逐位相同）
        let combo = variant("V2oa25_ht_h1h2").unwrap();
        assert!(combo.osc_candidate_freeze && combo.osc_strength_gate);
        let as_h1 = OrganicConfig {
            osc_strength_gate: false,
            ..combo.clone()
        };
        assert_eq!(
            format!("{as_h1:?}"),
            format!("{:?}", variant("V2oa25_ht_h1").unwrap())
        );
        let as_h2 = OrganicConfig {
            osc_candidate_freeze: false,
            ..combo
        };
        assert_eq!(
            format!("{as_h2:?}"),
            format!("{:?}", variant("V2oa25_ht_h2").unwrap())
        );
    }

    #[test]
    fn h4_variant_single_axis_on_ht() {
        // 默认/在册基线 osc_amp_gate=false（O0≡P5 零接触面——38课行32
        // 滚动振幅准入在变体层开启，h1/h2 同先例）
        assert!(!OrganicConfig::default().osc_amp_gate);
        let base = variant("V2oa25_ht").unwrap();
        assert!(!base.osc_amp_gate);
        // h4 只动 osc_amp_gate 一轴——其余字段与 V2oa25_ht 逐位相同
        let cfg = variant("V2oa25_ht_h4").unwrap();
        assert!(cfg.osc_amp_gate && cfg.osc_mode);
        assert!(!cfg.osc_candidate_freeze && !cfg.osc_strength_gate);
        // 门槛常数沿用在册预注册值（theta_cost_k=2.0 × friction_rt=10bps
        // = 0.2% 相对振幅下限——ledger_cost_gate 同对常数）
        assert_eq!(cfg.theta_cost_k, 2.0);
        assert_eq!(cfg.friction_rt, 0.001);
        assert_eq!(cfg.exit_mode, ExitMode::HoldTrend);
        let normalized = OrganicConfig {
            osc_amp_gate: false,
            ..cfg
        };
        assert_eq!(format!("{normalized:?}"), format!("{base:?}"));
    }

    #[test]
    fn entry_min_ladder_variants_single_axis() {
        // 默认/在册全变体 entry_min_ladder=0（O0≡P5 零接触面）
        assert_eq!(OrganicConfig::default().entry_min_ladder, 0);
        let base = variant("V2oa25").unwrap();
        assert_eq!(base.entry_min_ladder, 0);
        // 四臂只动 entry_min_ladder（+_ht 臂的 exit_mode）——其余逐位同 V2oa25
        for (name, min_lad, exit) in [
            ("V2oa25_e3", 3usize, ExitMode::Signal),
            ("V2oa25_e3ht", 3, ExitMode::HoldTrend),
            ("V2oa25_e4", 4, ExitMode::Signal),
            ("V2oa25_e4ht", 4, ExitMode::HoldTrend),
        ] {
            let cfg = variant(name).unwrap();
            assert_eq!(cfg.entry_min_ladder, min_lad, "{name}");
            assert_eq!(cfg.exit_mode, exit, "{name}");
            let normalized = OrganicConfig {
                entry_min_ladder: 0,
                exit_mode: ExitMode::Signal,
                ..cfg
            };
            assert_eq!(format!("{normalized:?}"), format!("{base:?}"), "{name}");
        }
    }

    #[test]
    fn nrf_variants_axis_decomposition() {
        // 在册基线 entry_voice=false（O0≡P5 零接触面）
        assert!(!OrganicConfig::default().entry_voice);
        assert!(!variant("V2oa25_emht").unwrap().entry_voice);
        // NRF1 = V2oa25_emht + entry_voice 单轴
        let emht = variant("V2oa25_emht").unwrap();
        let nrf1 = variant("NRF1").unwrap();
        assert!(nrf1.entry_voice);
        assert_eq!(nrf1.exit_mode, ExitMode::Emergent { hold_trend: true });
        let normalized = OrganicConfig {
            entry_voice: false,
            ..nrf1.clone()
        };
        assert_eq!(format!("{normalized:?}"), format!("{emht:?}"));
        // NRF2 = NRF1 + {rev_sub_depth=2, CounterSeg, sub_cost_gate} 三位
        let nrf2 = variant("NRF2").unwrap();
        assert_eq!(nrf2.rev_sub_depth, 2);
        assert_eq!(nrf2.sub_mode, SubMode::CounterSeg);
        assert!(nrf2.sub_cost_gate && !nrf2.sub_l41_gate);
        let normalized = OrganicConfig {
            rev_sub_depth: 0,
            sub_mode: SubMode::Zhongshu,
            sub_cost_gate: false,
            ..nrf2.clone()
        };
        assert_eq!(format!("{normalized:?}"), format!("{nrf1:?}"));
        // NRF2q = NRF2 + sizing Structure 单轴；emht_q = emht + 同单轴
        let nrf2q = variant("NRF2q").unwrap();
        assert_eq!(nrf2q.sizing, Sizing::Structure);
        let normalized = OrganicConfig {
            sizing: Sizing::Equal,
            ..nrf2q
        };
        assert_eq!(format!("{normalized:?}"), format!("{nrf2:?}"));
        let emht_q = variant("V2oa25_emht_q").unwrap();
        let normalized = OrganicConfig {
            sizing: Sizing::Equal,
            ..emht_q
        };
        assert_eq!(format!("{normalized:?}"), format!("{emht:?}"));
        // scco 合成臂 = 各自基臂 + {sc, co} 两位
        for (name, base) in [
            ("V2oa25_emht_scco", "V2oa25_emht"),
            ("NRF1_scco", "NRF1"),
            ("NRF2_scco", "NRF2"),
            ("NRF2q_scco", "NRF2q"),
        ] {
            let cfg = variant(name).unwrap();
            assert!(cfg.osc_shift_close, "{name}");
            assert_eq!(cfg.osc_domain, OscDomain::ConsolidationOnly, "{name}");
            let normalized = OrganicConfig {
                osc_shift_close: false,
                osc_domain: OscDomain::Any,
                ..cfg
            };
            assert_eq!(
                format!("{normalized:?}"),
                format!("{:?}", variant(base).unwrap()),
                "{name}"
            );
        }
    }

    #[test]
    fn v2oa25f1_inherits_default_gate_plus_fractal_dual_gate() {
        let cfg = variant("V2oa25F1").unwrap();
        assert_eq!(
            cfg.theta_mode,
            ThetaMode::AdaptiveQuantile {
                q: 0.25,
                window: 50,
                min_obs: 10
            }
        );
        assert!(cfg.rev_l41_gate);
        assert_eq!(cfg.rev_sub_depth, 1);
        assert_eq!(cfg.sub_mode, SubMode::Fractal);
        assert!(cfg.sub_cost_gate && cfg.sub_l41_gate);
    }
}
