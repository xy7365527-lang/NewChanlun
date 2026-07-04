//! 背驰度量（reference-theta-v0.md:34,37）——MACD 辅助 + **A/B/C 趋势/盘整背驰框架**。
//!
//! ## 两层结构（★no-patch：力度原语 + A/B/C 框架层）
//!
//! 本文件分两层，对应缠论背驰的两个语义层（第24课:22-24 + beichi.md v1.1 已结算）：
//!
//! - **力度原语层**（§A）：MACD 段面积比较（`segment_macd_area` / `is_divergence` /
//!   `segments_diverge`）——「后段面积 `<` 前段面积」的纯算术，对齐契约锚
//!   `Origin.Divergence.IsDivergence`（`d.forceC.area < d.forceA.area`，Divergence.lean:83）+
//!   `Origin.ForceInterface.ForceMeasure`（[`super::force_conformance`] 实例化）。这是**正确的
//!   力度比较原语**（C段 < A段），被 force_conformance + 次级别背驰（mod.rs）复用，**保留**。
//!
//! - **A/B/C 框架层**（§B，本次新增，消解 grammar-audit 头号缺口）：把力度原语组装为缠论
//!   **趋势背驰 vs 盘整背驰**判定。退化的旧实现只有力度原语，`judge_first` 直接用「任意前
//!   同向段」喂原语——**无 A/B/C 走势分解、无趋势/盘整区分、无 τ 门控**，产假背驰=假买卖点。
//!   本层补上 A/B/C 的精确结构（第24课:22-24）：
//!   · **A/B/C 是走势级别三段**：A=前一离开段、B=中间中枢、C=后一离开段（**不是**次级别 a-b-c，
//!     也不是力度段）。第24课原文:22「同向趋势之间一定有一个盘整或反向趋势连接，把这三段分别
//!     称为 A、B、C 段」。
//!   · **趋势背驰**（[`trend_divergence`]）：走势类型 τ=Trend（≥2 依次同向中枢，reference:34
//!     「同级别趋势≥两同向中枢后」+ maimai.md:105「≥2个依次同向的同级别中枢」）。A=相邻**前
//!     一中枢**的离开段，C=相邻**后一中枢**的离开段（破中枢段）。C段面积 < A段面积 ⟹ 趋势背驰。
//!   · **盘整背驰**（[`consolidation_divergence`]）：τ=Consolidation（恰 1 中枢，zoushi.md:107）。
//!     A、C 是**同一中枢**的两次同向离开段（第24课:34-36 + beichi.md:113）。C < A ⟹ 盘整背驰。
//!   · **τ 门控**：已迁 decompose.rs（task #143 局部趋势门，复用 `center::classify_relation`）——
//!     ≥2 全链同向 → Trend；1 中枢 → Consolidation；mixed/扩张 → 非趋势非盘整（不产第一类）。
//!     第一类买卖点**只由趋势背驰产生**（盘整背驰不产第一类，beichi.md #4 + maimai.md:56 已结算）。
//!
//! ## bit-exact 注意点（★浮点域隔离）
//!
//! MACD 是 v0 的**辅助**度量（结构前提优先）。MACD 浮点运算**隔离在本文件**，按固定
//! 约简顺序计算，**不漏入整数 tick 域**（types.rs 的结构判定全在 i64）。背驰输出是
//! bool（严格变小），bool 无浮点歧义——浮点只在内部面积比较时出现，且用严格 `<`。
//! A/B/C 框架层的中枢关系判定（趋势门控，decompose.rs）在**整数 tick 域**（`center::classify_relation`），
//! 浮点只在力度原语层（面积比较）出现——两域不混。
//!
//! ## MACD(12,26,9) 算法（reference-theta-v0.md:37，固定约简顺序）
//!
//! - `EMA[0] = close[0]`（★首值取首 close，非 SMA 预热）。
//! - `EMA[i] = α*close[i] + (1-α)*EMA[i-1]`，`α = 2/(period+1)`。
//! - `DIF = EMA_fast - EMA_slow`（逐 bar）。
//! - `DEA = EMA(DIF, signal)`，DEA 首值取首 DIF。
//! - `hist = DIF - DEA`（逐 bar）。
//!
//! ## 背驰判据（reference-theta-v0.md:34,37；契约锚 `Origin.Divergence` + `Origin.ForceInterface`）
//!
//! 力度原语「同向段面积**严格**变小才成立，等值不成立」。段面积 = 该段 bar 区间内 `|hist|` 之和。
//! 后一同向段面积 `<` 前一同向段面积 ⟹ 力度衰减（严格 `<`，等值返回 false）。契约锚
//! `Origin.Divergence.IsDivergence`（`d.forceC.area < d.forceA.area`，Divergence.lean:83）。
//! A/B/C 框架层把该原语限定到正确的 A/C 段配对（趋势=跨相邻中枢 / 盘整=同中枢两次离开）。
//!
//! ## 认识论（formalization-validity-domain）
//!
//! MACD 常数从 config 读（fast/slow/signal）。EMA 首值规则 + 面积规则标 `[L3经验待标定]`
//! （reference-theta-v0.md:37）——v0 默认占位，L2/L3 标定不归本工位。本文件证的是
//! 「给定 MACD 参数后 A/B/C 段配对 + 面积比较确定」（**L1** 管线正确性），**不**证「MACD 趋势
//! 背驰预测在真实行情有效」（**L2/L3**，否证检验，不在本工位；alpha 影响须统一 L3 验证）。
//! 趋势/盘整门控判据（中枢同向关系）是 **L0**（纯整数几何，不依赖经验数据）。

use super::super::config::MacdConfig;
use super::super::types::{Center, Direction, Segment, Tick};

/// MACD 逐 bar 输出（DIF/DEA/hist，浮点域，隔离在本结构）。
#[derive(Debug, Clone, PartialEq)]
pub struct MacdSeries {
    pub dif: Vec<f64>,
    pub dea: Vec<f64>,
    pub hist: Vec<f64>,
}

/// EMA（首值取首元素，reference-theta-v0.md:37）。
///
/// `out[0]=src[0]`；`out[i]=α*src[i]+(1-α)*out[i-1]`，`α=2/(period+1)`。
/// bit-exact：α 与递推按固定顺序计算（先 `α*src[i]`，再 `(1-α)*prev`，最后相加）。
/// 边界条件：`src` 为空 ⟹ 空序列；`period=0` 在 config 校验层拒绝（此处假设 >=1）。
fn ema(src: &[f64], period: u32) -> Vec<f64> {
    let mut out = Vec::with_capacity(src.len());
    if src.is_empty() {
        return out;
    }
    let alpha = 2.0 / (period as f64 + 1.0);
    out.push(src[0]); // 首值取首元素（非 SMA 预热）。
    for i in 1..src.len() {
        // 固定约简顺序：α*src[i] + (1-α)*prev。
        let prev = out[i - 1];
        let v = alpha * src[i] + (1.0 - alpha) * prev;
        out.push(v);
    }
    out
}

/// 计算 MACD 序列（reference-theta-v0.md:37，固定约简顺序）。
///
/// `closes` 是逐 bar 收盘价（已量化的 tick 转 f64——MACD 在浮点域，但输入来自整数 tick
/// 的确定转换，无额外浮点引入）。返回 DIF/DEA/hist 三序列（等长 = closes 长度）。
pub fn compute_macd(closes: &[f64], cfg: &MacdConfig) -> MacdSeries {
    let ema_fast = ema(closes, cfg.fast);
    let ema_slow = ema(closes, cfg.slow);
    // DIF = EMA_fast - EMA_slow（逐 bar，等长）。
    let dif: Vec<f64> = ema_fast
        .iter()
        .zip(ema_slow.iter())
        .map(|(f, s)| f - s)
        .collect();
    // DEA = EMA(DIF, signal)，首值取首 DIF。
    let dea = ema(&dif, cfg.signal);
    // hist = DIF - DEA。
    let hist: Vec<f64> = dif.iter().zip(dea.iter()).map(|(d, e)| d - e).collect();
    MacdSeries { dif, dea, hist }
}

/// 单 bar MACD 输出（增量 API 用，与 [`MacdSeries`] 同浮点域）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MacdPoint {
    pub dif: f64,
    pub dea: f64,
    pub hist: f64,
}

/// MACD 增量递推状态（per-bar substrate O(n²) 根因解法，231号纯性能非 alpha）。
///
/// 携带 EMA_fast / EMA_slow / DEA 三条 EMA 的末值与已处理 bar 数，使每 bar
/// 以 O(1) 延伸而非 O(merged) 全量重算。bit-exact 镜像 [`compute_macd`]：
/// - α = `2.0/(period+1.0)`（与 [`compute_macd`] 内 `ema` 同公式）。
/// - EMA 递推顺序 `alpha*x + (1-alpha)*prev`（与 `ema` 同约简顺序）。
/// - 首值规则：EMA_fast[0]=EMA_slow[0]=close[0]；DIF[0]=0；DEA[0]=首 DIF=0（`ema` 首值取首元素）。
///
/// 边界条件：`bars_processed == 0` 时本状态对应首 bar（init 契约）；
/// `cfg` 的 fast/slow/signal 须 >=1（config 校验层拒绝 0，此处假设）。
/// 认识论：L1（管线 bit-exact 等价于全量版），不携带 alpha 信息——增量只是把同一
/// 浮点约简从「全量重算」改成「逐 bar 延伸」，数值不变（231号：纯性能）。
#[derive(Debug, Clone, PartialEq)]
pub struct MacdState {
    bars_processed: u64,
    ema_fast: f64,
    ema_slow: f64,
    dea: f64,
    alpha_fast: f64,
    alpha_slow: f64,
    alpha_signal: f64,
}

impl MacdState {
    /// 从首 bar 初始化增量状态（镜像 [`compute_macd`] 对 `closes[0..=0]` 的输出）。
    ///
    /// 等价于 `compute_macd(&[first_close], cfg)` 后取末状态。首 bar 输出：
    /// DIF = first_close - first_close = 0；DEA = EMA(DIF, signal) 首值 = 首 DIF = 0；
    /// hist = DIF - DEA = 0。
    pub fn init(first_close: f64, cfg: &MacdConfig) -> Self {
        Self {
            bars_processed: 1,
            ema_fast: first_close,
            ema_slow: first_close,
            dea: 0.0,
            alpha_fast: 2.0 / (cfg.fast as f64 + 1.0),
            alpha_slow: 2.0 / (cfg.slow as f64 + 1.0),
            alpha_signal: 2.0 / (cfg.signal as f64 + 1.0),
        }
    }

    /// 已处理的 bar 数（init 后 =1，每 append +1）。
    pub fn bars_processed(&self) -> u64 {
        self.bars_processed
    }

    /// 当前 bar 的 MACD 输出（由当前状态直接派生，O(1)）。
    ///
    /// DIF = ema_fast - ema_slow；hist = DIF - DEA。与全量版对应位置逐位相等。
    pub fn current_point(&self) -> MacdPoint {
        let dif = self.ema_fast - self.ema_slow;
        MacdPoint { dif, dea: self.dea, hist: dif - self.dea }
    }

    /// 增量延伸一个 bar（O(1)），返回新状态。
    ///
    /// 递推顺序严格镜像全量版：先更新 EMA_fast/slow（得到新 DIF），再用新 DIF
    /// 递推 DEA（signal EMA 的输入是 DIF 序列）。这是 bit-exact 关键——DEA 必须用
    /// **本 bar 的新 DIF** 而非上一 bar 的旧 DIF（全量版 `dea = ema(&dif, signal)` 中
    /// dif/dea 同索引对齐，即 DEA[i] 用 DIF[i] 递推）。
    pub fn append(&self, new_close: f64) -> Self {
        // EMA 递推：alpha*x + (1-alpha)*prev（与 `ema` 同约简顺序）。
        let new_ema_fast = self.alpha_fast * new_close + (1.0 - self.alpha_fast) * self.ema_fast;
        let new_ema_slow = self.alpha_slow * new_close + (1.0 - self.alpha_slow) * self.ema_slow;
        let new_dif = new_ema_fast - new_ema_slow;
        // DEA = EMA(DIF, signal)，本 bar 用新 DIF 递推（全量版同索引对齐）。
        let new_dea = self.alpha_signal * new_dif + (1.0 - self.alpha_signal) * self.dea;
        Self {
            bars_processed: self.bars_processed + 1,
            ema_fast: new_ema_fast,
            ema_slow: new_ema_slow,
            dea: new_dea,
            alpha_fast: self.alpha_fast,
            alpha_slow: self.alpha_slow,
            alpha_signal: self.alpha_signal,
        }
    }
}

/// 增量 MACD API：基于上一状态延伸一个 bar（O(1)），返回新状态。
///
/// `prev` 由 [`MacdState::init`]（首 bar）或上一次本函数的返回值（后续 bar）提供。
/// 每次调用对应一个新 close，返回状态的 [`MacdState::current_point`] 即该 bar 输出。
/// bit-exact：逐 bar append 的输出 == [`compute_macd`](closes) 对应位置（见测试）。
///
/// 这是 per-bar substrate O(n²) 根因之一的全量 MACD 重算的 O(n) 替代——
/// 把每 bar `compute_macd(merged)` 的 O(merged) 降为 O(1) 延伸（231号纯性能）。
pub fn compute_macd_append(prev: &MacdState, new_close: f64) -> MacdState {
    prev.append(new_close)
}

/// 由完整 closes 用增量 API 构造末状态（供 bit-exact 对照与下游接入验证）。
///
/// 从 `MacdState::init(closes[0])` 起，逐 bar `append`，返回末状态。
/// 等价于 `compute_macd(closes)` 的末状态 + 全序列 `current_point` 轨迹。
pub fn macd_state_from_closes(closes: &[f64], cfg: &MacdConfig) -> MacdState {
    let mut state = MacdState::init(closes[0], cfg);
    for &c in &closes[1..] {
        state = state.append(c);
    }
    state
}

/// 同向段 MACD 面积（reference-theta-v0.md:37）= 段 bar 区间 `[start,end]` 内 `|hist|` 之和。
///
/// `[start, end]` 是闭区间 bar 索引。越界（end>=len 或 start>end）⟹ 0.0（空段无面积）。
/// bit-exact：求和按 bar 索引升序累加（固定顺序），用 `f64::abs`。
pub fn segment_macd_area(hist: &[f64], start: usize, end: usize) -> f64 {
    if start > end || end >= hist.len() {
        return 0.0;
    }
    let mut area = 0.0;
    for &h in &hist[start..=end] {
        area += h.abs();
    }
    area
}

/// 背驰判定（reference-theta-v0.md:37）：后一同向段面积**严格**小于前一同向段。
///
/// `prev_area`/`curr_area` 是两个同向段的 MACD 面积。`curr < prev`（严格 `<`）⟹ 背驰
/// 成立；等值（`curr == prev`）或变大 ⟹ 不成立。等值不成立是 reference-theta-v0.md:37
/// 明确规则（不是浮点容差问题——段面积相等表示力度未衰减，非背驰）。
pub fn is_divergence(prev_area: f64, curr_area: f64) -> bool {
    curr_area < prev_area
}

/// 两段背驰判定（端到端）：从 hist 序列取两同向段面积并比较。
///
/// `prev_seg`/`curr_seg` 是 `(start,end)` 闭区间 bar 索引对。后段面积严格小于前段 ⟹ 力度衰减
/// （力度原语，A/B/C 框架层 §B 用它比较已定位的 A/C 段）。
///
/// ## ★诚实 gap（缠师第17课，编排者 2026-07-01 坐实——识别非实装）
///
/// 缠师第17课原文（017-第17课.md:248/250）：「用均线或 MACD 看背驰都是**辅助性**的，都不是最
/// 重要的」「没有 MACD 就判断不了背驰？显然不是。**那只是辅助**」。背驰的**定义**是「两相邻同向
/// 趋势间后者比前者的**走势力度**减弱」——**走势力度**才是判据，MACD 面积只是一个 proxy。
///
/// 本实装的背驰**仅由 MACD 段面积**判定（`segment_macd_area` = Σ|hist|），**无独立的走势力度
/// 判据**（价格振幅/速度/成交量力度等）。`force_conformance.rs` 的 `ForceMeasure.strength` 亦只
/// 包 MACD area（见其模块头 L2 未验证声明）。这是比 P2 veto **更根本的简化**：辅助指标（MACD）
/// 被当成了背驰的**唯一判据**，与缠师原文相悖（同构：区间套同一性证书零调用、简化被当完整）。
///
/// ponytail: MACD-area-only 力度判据，真走势力度（振幅/速度/量能，第17课定义）留待后续大工程；
/// 本 gap 已诚实标注（编排者裁定：识别并标注，不顺手实装）。升级路径 = ForceMeasure 增非-MACD
/// strength 实例（价格振幅/速度），MACD area 降为多 proxy 之一，与 P2「MACD 降 feature」同精神。
pub fn segments_diverge(
    hist: &[f64],
    prev_seg: (usize, usize),
    curr_seg: (usize, usize),
) -> bool {
    let prev_area = segment_macd_area(hist, prev_seg.0, prev_seg.1);
    let curr_area = segment_macd_area(hist, curr_seg.0, curr_seg.1);
    is_divergence(prev_area, curr_area)
}

// ═══════════════════════════════════════════════════════════════════════════
// § A2. 多力度原语 + Weak_Θ 词典序（P2 方案A §4/§7 改点5；第17课「黄白线主，面积次」+ 第34课）
//
// ★存在论位置（p2-plan §4 + codex-review-20260701-2251 护栏3/6）：这些是**力度原语层**——
// 供 **selector 层** Weak_Θ 力度门消费的多 proxy（DIF/价格振幅/速度），**不下沉 buy1 定义层**
// （buy1 判据仍 `segments_diverge`=MACD 面积，class_index 语义冻结，护栏3）。DIF 逻辑移植 rust
// 顶层旧引擎 `src/divergence.rs`（dif_peak/T6），坐标系改为 theta_v0 的 `dif` 序列 + 段闭区间。
//
// ★认识论（formalization-validity-domain 231号，强制）：
// - **L1**：原语接口正确性（给定 dif/closes 求峰值/振幅/速度是确定性算术，逐例可验）。
// - **L2/L3**：「哪个 mode 有 alpha / 词典序优于单 MACD」需三套 OOS（MACD/Force/LEX），
//   **本层不声明**，留 W-VERIFY（#23）。weak_theta 只提供判定纯函数，不声明择时有效性。
//
// ★有效域边界更新（A6 #159 后）：R2 时代「Candidate 无段 close 序列 ⟹ 算不了振幅/速度 ⟹ 不造
// 死字段」的前提已被两步解除——#115 在 `BspPoint.force` 收进 signal 抽取层算好的 A/C 段 5 proxy
// （单一来源），A6（#159）令 `Candidate.force` 纯透传该值进 gamma 组装 ⟹ z 装配点
// （`selector::z_of_candidate`）读 `c.force` 填 `force_state` 第 8 维（统计层与生产 π fill loop
// 同经此路，无死字段）。**仍未接**：`filter_gamma` 的 Weak_Θ 力度门（`weak_theta` 词典序判定进
// χ 判据）——那是判据变更非透传，留 A3（#164）独立工位。诚实声明：原语 ✓，透传 ✓（A6），
// Weak_Θ 门接入 ✗（A3 gap）。
// ═══════════════════════════════════════════════════════════════════════════

/// 段力度多 proxy（P2 §6 selector 状态 z 的力度分量：MACD 面积 / DIF 峰值 / 价格振幅 / 速度）。
///
/// 三 proxy 对应第17课「黄白线（DIF）最重要，面积次之」+ 价格振幅/速度（走势力度的直接度量，
/// 非 MACD proxy）。由 [`force_features`] 从段区间算出，供 [`weak_theta`] 词典序比较。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ForceFeatures {
    /// MACD 段面积 = Σ|hist|（力度原语，`segment_macd_area`）。
    pub macd_area: f64,
    /// DIF 段峰值绝对值（黄白线主判据，第17课「黄白线最重要」；移植旧引擎 dif_peak）。
    pub dif_peak: f64,
    /// 价格振幅 |端价差|（L0 整数 tick，走势力度直接度量，非 MACD proxy）。
    pub price_amplitude: i64,
    /// 价格速度 |端价差|/Δbar（f64，单位时间价格变动）。
    pub price_speed: f64,
    /// 段全变差 TV = Σ|P_{t+1}−P_t|（原文 §5 p6 𝒜_ℓ 成员；整数 tick 域，路径长度 ≥ |端价差|）。
    pub tv: i64,
}

/// A/C 段力度 proxy 对（趋势背驰的两段并置——`Weak_Θ(seg_a, seg_c)` 用它比较）。
///
/// 一类候选（趋势背驰）由 A 段（倒数第二中枢离开）+ C 段（破最后中枢）配对，每段一个
/// [`ForceFeatures`]。这是 selector Weak_Θ 力度门 / 离线多 proxy 交叉验证（W-VERIFY #13）的输入。
/// **认识论 L1**：段坐标→proxy 是确定性算术；「哪个 proxy 有 alpha」是 L2/L3（本层不声明）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ForceProxies {
    /// A 段（前趋势离开段）力度。
    pub seg_a: ForceFeatures,
    /// C 段（破最后中枢段）力度。
    pub seg_c: ForceFeatures,
}

/// 力度支配态（`关于背驰.pdf` §9.1 `ForceState`，5-proxy 近似 = `ForceStateA5`）。
///
/// 支配序四态（§5-6 p6 `s ≺_𝒜 s'`）：C 段（后离开段）相对 A 段（前离开段）在允许力度族
/// 𝒜₅={macd_area, dif_peak, price_amplitude, price_speed, tv} 上的支配关系。**背驰 = C 力度衰减 =
/// `Dominated`**（C 在全部 5 proxy 上 ≤ A 且至少一个 <）。
///
/// **命名（beta-bucket-design v2 §2.3 / codex-beta ② / q3 D-1 残余）**：`A5` 后缀诚实标注现有
/// 5 proxy——TV（全变差）已并入本口径族；原文完整 𝒜_ℓ 仍缺递归次级别力度 SubMovePower
/// （Σ_{次级别同向段} m_{ℓ-1}(u)），其数据源未达 signal 抽取层（signal.rs 只有 hist/dif/closes，
/// 无塔次级别段），登记诚实缺口——裁定见 codex-a4-force-20260704.md。补齐后可升名 `ForceState`。
/// 单调性：补维只会把 `Dominated`/`Dominates`/`Tie` 变 `Incomparable`（新增口径可能冲突），反向
/// 不会 ⟹ `ForceStateA5` 的 `Dominated` 是完整支配序 `Dominated` 的**超集（宽判背驰）**。
///
/// **认识论 L1**：给定 A/C ForceFeatures 求支配态是确定性算术（管线正确性）。「哪个态有 alpha」=L2/L3。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ForceStateA5 {
    /// C 在全部 5 proxy 上 ≤ A 且至少一个 <——**确定背驰**（C 力度衰减）。
    Dominated,
    /// C 在全部 5 proxy 上 ≥ A 且至少一个 >——确定力度延续（非背驰）。
    Dominates,
    /// C 与 A 全部 5 proxy 相等。
    Tie,
    /// 口径冲突（部分 proxy C<A、部分 C>A）——**不作背驰确认**（codex-beta ②）。
    Incomparable,
}

impl ForceProxies {
    /// 𝒜₅ 支配序比较（唯一支配序原语，beta-bucket-design v2 §6 路由 ⑤——不在别处重算）。
    ///
    /// 逐 proxy 比较 C(`seg_c`) vs A(`seg_a`)：任一 proxy `c<a` 记 weaker，`c>a` 记 stronger。
    /// 全 ≤ 且有 weaker ⟹ `Dominated`；全 ≥ 且有 stronger ⟹ `Dominates`；全等 ⟹ `Tie`；
    /// 既有 weaker 又有 stronger ⟹ `Incomparable`。price_amplitude/tv 是 i64，其余 f64（严格
    /// `<`/`>`，与 `is_divergence` 同「等值不算衰减」口径）。
    pub fn force_state(&self) -> ForceStateA5 {
        let (a, c) = (&self.seg_a, &self.seg_c);
        // 五 proxy 的 (c<a, c>a) 布尔对。amplitude/tv 为 i64，比较前统一到同类型无损。
        let cmps = [
            (c.macd_area < a.macd_area, c.macd_area > a.macd_area),
            (c.dif_peak < a.dif_peak, c.dif_peak > a.dif_peak),
            (c.price_amplitude < a.price_amplitude, c.price_amplitude > a.price_amplitude),
            (c.price_speed < a.price_speed, c.price_speed > a.price_speed),
            (c.tv < a.tv, c.tv > a.tv),
        ];
        let weaker = cmps.iter().any(|&(lt, _)| lt);
        let stronger = cmps.iter().any(|&(_, gt)| gt);
        match (weaker, stronger) {
            (true, false) => ForceStateA5::Dominated,
            (false, true) => ForceStateA5::Dominates,
            (false, false) => ForceStateA5::Tie,
            (true, true) => ForceStateA5::Incomparable,
        }
    }
}

/// DIF 段峰值绝对值（黄白线主判据原语，第17课；移植旧引擎 `dif_peak_for_range` 到 theta_v0）。
///
/// `dif` 是 `compute_macd(...).dif` 序列（黄白线，与 hist 同坐标系）。`[start,end]` 闭区间 bar
/// 下标。向上段取区间内 `max(dif)`（正峰），向下段取 `min(dif)` 的绝对值（负峰）——与旧引擎
/// `dif_peak_for_range(up)` 语义一致（up→max 正值 / down→|min 负值|）。越界/空 ⟹ 0.0。
///
/// bit-exact：按 bar 升序扫描取极值（固定顺序），`f64` 比较用 `max`/`min`（无 NaN 前提，dif 由
/// EMA 差得，有限）。
pub fn segment_dif_peak(dif: &[f64], start: usize, end: usize, direction: Direction) -> f64 {
    if start > end || end >= dif.len() {
        return 0.0;
    }
    let slice = &dif[start..=end];
    match direction {
        // 向上段：黄白线正峰（max）。缠师顶背驰看黄白线新高与否。
        Direction::Up => slice.iter().copied().fold(f64::NEG_INFINITY, f64::max).max(0.0),
        // 向下段：黄白线负峰（|min|）。底背驰看黄白线新低。
        Direction::Down => slice.iter().copied().fold(f64::INFINITY, f64::min).min(0.0).abs(),
    }
}

/// 段价格振幅 |端价差|（L0 整数 tick，走势力度的直接度量，第17课「走势力度」非 MACD proxy）。
///
/// `closes` 段端点（下标 `start`/`end`）close 的有向差绝对值。越界 ⟹ 0（空段无振幅）。
/// **整数域**（无浮点）——close 已量化为 tick（types.rs），振幅是 tick 差。
pub fn segment_price_amplitude(closes: &[Tick], start: usize, end: usize) -> i64 {
    if start > end || end >= closes.len() {
        return 0;
    }
    (closes[end] - closes[start]).abs()
}

/// 段价格速度 |端价差|/Δbar（单位时间价格变动，f64）。
///
/// 振幅 / 段跨度（`end-start`，至少 1 避免除零）。越界 ⟹ 0.0。
pub fn segment_price_speed(closes: &[Tick], start: usize, end: usize) -> f64 {
    if start > end || end >= closes.len() {
        return 0.0;
    }
    let amp = (closes[end] - closes[start]).abs() as f64;
    let span = (end - start).max(1) as f64;
    amp / span
}

/// 段全变差 TV = Σ|closes[t+1]−closes[t]|，t∈[start,end)（原文 §5 p6 `TV=Σ|P_{t+1}−P_t|`）。
///
/// 与振幅的差：振幅只看端点差，TV 累积路径长度（TV ≥ |端价差|，段内折返越多 TV 越大）。
/// 越界/单点段 ⟹ 0。整数 tick 域（无浮点）。
pub fn segment_total_variation(closes: &[Tick], start: usize, end: usize) -> i64 {
    if start >= end || end >= closes.len() {
        return 0;
    }
    closes[start..=end].windows(2).map(|w| (w[1] - w[0]).abs()).sum()
}

/// 从段区间算全部力度 proxy（`ForceFeatures`）——MACD 面积 + DIF 峰值 + 价格振幅 + 速度 + TV。
///
/// `hist`/`dif` 是 `compute_macd` 的 hist/dif 序列；`closes` 是 close(tick) 序列；三者同坐标系
/// （bar 下标对齐）。`(start,end)` 闭区间，`direction` 段方向（DIF 峰值方向敏感）。
pub fn force_features(
    hist: &[f64],
    dif: &[f64],
    closes: &[Tick],
    start: usize,
    end: usize,
    direction: Direction,
) -> ForceFeatures {
    ForceFeatures {
        macd_area: segment_macd_area(hist, start, end),
        dif_peak: segment_dif_peak(dif, start, end, direction),
        price_amplitude: segment_price_amplitude(closes, start, end),
        price_speed: segment_price_speed(closes, start, end),
        tv: segment_total_variation(closes, start, end),
    }
}

/// Weak_Θ 力度比较模式（第34课 Θ=(level,DIF,area,amplitude,speed,…) 参数化；「三套 OOS」+ 默认 Lex）。
///
/// 每个 mode 定义「后段力度 `<` 前段力度」（背驰=力度衰减）的判据用哪个 proxy。`Lex` 是缠师第17课
/// 「黄白线主 ▷ 面积次」的词典序（DIF 可判用 DIF，否则退面积）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeakThetaMode {
    /// 纯 MACD 面积（`segments_diverge` 同判据，与 buy1 冻结判据一致——作对照基线）。
    MacdArea,
    /// 纯 DIF 峰值（黄白线主判据，第17课「黄白线最重要」）。
    Dif,
    /// 纯价格振幅（走势力度直接度量，非 MACD proxy）。
    PriceAmplitude,
    /// 词典序（第17课/第34课默认）：DIF 主 ▷ 面积次——DIF 严格可判则用 DIF，DIF 相等则退面积。
    Lex,
}

/// Weak_Θ 力度衰减判定（第34课参数化弱化关系，`Weak_Θ(A段,C段)` = C 段力度**严格小于** A 段）。
///
/// **词典序语义**（p2-plan §4 + 第17课「黄白线最重要，面积次之」+ swarm 待判点 B「非 AND/OR」）：
/// - `MacdArea`：`C.macd_area < A.macd_area`（与 buy1 冻结判据一致，对照基线）。
/// - `Dif`：`C.dif_peak < A.dif_peak`（黄白线主）。
/// - `PriceAmplitude`：`C.price_amplitude < A.price_amplitude`。
/// - `Lex`：DIF 主 ▷ 面积次——`C.dif < A.dif` ⟹ true（背驰）；`C.dif > A.dif` ⟹ false（力度延续）；
///   `C.dif == A.dif`（DIF 不可判）⟹ 退面积 `C.area < A.area`。**非 AND/OR**（027:30「只要其中
///   一个符合就可以」否证 AND；第34课「黄白线最重要」否证纯 OR）。
///
/// ★护栏3（codex 语义诚实）：本函数供 **selector 层** Weak_Θ 力度门，**不喂 buy1**（buy1 判据
/// 冻结为 `segments_diverge`=MACD 面积，class_index 语义不动）。level 元门（级别配套）不在此函数
/// （selector 已按 level 分桶，元门由 MuClass.level 承载）——本函数是同级别内的力度词典序。
///
/// ★认识论 L1（给定 A/C ForceFeatures 求 Weak 是确定性布尔，验证判定逻辑）——「哪个 mode 有
/// alpha」是 L2/L3（W-VERIFY，本函数不声明）。
pub fn weak_theta(mode: WeakThetaMode, seg_a: &ForceFeatures, seg_c: &ForceFeatures) -> bool {
    match mode {
        WeakThetaMode::MacdArea => seg_c.macd_area < seg_a.macd_area,
        WeakThetaMode::Dif => seg_c.dif_peak < seg_a.dif_peak,
        WeakThetaMode::PriceAmplitude => seg_c.price_amplitude < seg_a.price_amplitude,
        WeakThetaMode::Lex => {
            // 词典序：DIF 主判据。DIF 严格可判（≠）时用 DIF；DIF 相等则退面积（次判据）。
            if seg_c.dif_peak != seg_a.dif_peak {
                seg_c.dif_peak < seg_a.dif_peak
            } else {
                seg_c.macd_area < seg_a.macd_area
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// § B. A/B/C 趋势/盘整背驰框架层（第24课:22-24 + beichi.md v1.1 已结算）
// ═══════════════════════════════════════════════════════════════════════════

/// A/B/C 背驰段对（第24课:22-24 走势级别三段；契约锚 `Origin.Divergence.DivergencePair`）。
///
/// - `seg_a`：A 段（前一离开段）的 source_index 闭区间 `(start,end)`。
/// - `seg_c`：C 段（后一离开段/破中枢段）的 source_index 闭区间。
/// - `is_trend`：true=趋势背驰（A/C 跨相邻两中枢，B=中间中枢）；false=盘整背驰（A/C 同一中枢两次离开）。
///
/// B 段在结构上是 A 与 C 之间的中枢（趋势背驰=后一中枢；盘整背驰=唯一中枢），由调用方的中枢序列
/// 隐含定位——本结构只携 A/C 段坐标（力度比较的两端），B 的存在性由 τ 门控（≥1 中枢）保证。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbcDivergence {
    pub seg_a: (usize, usize),
    pub seg_c: (usize, usize),
    pub is_trend: bool,
}

impl AbcDivergence {
    /// 背驰判定（C段面积 < A段面积，力度原语）。`src_to_idx`：source_index→closes 下标的升序映射。
    /// 段无法映射到 closes 区间（越界/空）⟹ false（无面积=非背驰，与 judge_first 越界口径一致）。
    pub fn diverges(&self, hist: &[f64], a_idx: (usize, usize), c_idx: (usize, usize)) -> bool {
        segments_diverge(hist, a_idx, c_idx)
    }
}

/// 趋势背驰的 A **离开走势区间**定位（第24课:22-24 + Q5 裁决 task #145，趋势 τ=Trend）。
///
/// ★Q5 谱系（一类买卖点.pdf 裁决，2026-07 #145）：「A/C 应为次级别走势类型，不应默认是单一线段。
/// If a lower-level departure move consists of multiple segments, the MACD/force comparison should
/// cover the whole lower-level move type interval：I(A)=[λ_A,ρ_A]，Area(A)=Σ_{t∈I(A)}|hist_t|。」
/// A = 离开 C_prev 的次级别走势类型（同向 d）——**不是**其中最后一个同向段。
///
/// 在 ≥2 同向中枢的趋势中，C = **最后一个中枢**之后破中枢的离开走势（由 judge_first 定位）；
/// A = **倒数第二个中枢**之后的同向离开走势区间。窗口过滤与旧单段实现相同
/// （direction==trend_dir ∧ start_index ≥ prev_center.end_index ∧ start_index < last_center.end_index），
/// 区间 = **当前离开 episode**（首匹配段起点..末匹配段终点，含中间反向段 bar，Q5 面积口径）。
///
/// ★episode 定界（codex ac4 审查 #1 修复）：「失败离开→回中枢→重新离开」是两个独立 episode，
/// 不桥接。边界 = 窗口内**最后一个回中枢段**（[`departure_episode_start`]，与 judge_pan_div
/// `reenters` 同一判据——反向段端点回到 prev_center 核心 [zd,zg] 内侧）。无回中枢段 ⟹ 整窗口
/// 一个 episode（与修复前行为相同）；恰一个匹配段 ⟹ 与旧单段返回值 bit 相同（兼容）。
///
/// ★A/C 跨相邻中枢配对（消解退化的「任意前同向段」）：A 不是序列序任意前同向段，而是趋势中**相邻
/// 前一中枢**的离开走势——第24课:24「A 之前已有一个中枢，B 是这个大趋势的另一个中枢」。`segments`
/// 按 start_index 升序；`last_center`/`prev_center` 是趋势的最后两个相邻中枢。
///
/// 返回 A 区间 `(λ_A, ρ_A)`（source_index 闭区间）；无匹配段/最后回中枢段之后无同向段 ⟹ None。
/// 离开走势方向 = 趋势方向（向下趋势=向下离开=底背驰候选；向上趋势=顶背驰候选）。
pub fn locate_departure_move_a(
    segments: &[Segment],
    anchors: &[Option<Direction>],
    prev_center: &Center,
    last_center: &Center,
    trend_dir: Direction,
) -> Option<(usize, usize)> {
    let lo = segments.partition_point(|s| s.start_index < prev_center.end_index);
    let hi = segments.partition_point(|s| s.start_index < last_center.end_index);
    let win = &segments[lo..hi];
    let awin = &anchors[lo..hi];
    let lambda_a = episode_start_in(win, awin, prev_center, trend_dir)?;
    // ★Q7-#1 裁定C：A 段候选筛选用 anchor 方向——fallback 单元不作 A 段方向锚（仍是区间成员）。
    let mut it = win
        .iter()
        .zip(awin)
        .filter(|(s, a)| **a == Some(trend_dir) && s.start_index >= lambda_a)
        .map(|(s, _)| s);
    let first = it.next()?;
    let last = it.last().unwrap_or(first);
    // I(A) = [episode 首段起点, episode 末同向段终点]（Q5：多段时含中间反向段 bar）。
    Some((first.start_index, last.end_index))
}

/// 当前离开 episode 的起点 λ（Q5 + codex ac4 审查 #1/#2 修复的共享定界原语）。
///
/// `win` = 离开中枢 `c` 的候选段窗口（start_index 升序切片）；episode 边界 = 窗口内**最后一个
/// 回中枢段**（反向段端点回到 c 核心 [zd,zg] 内侧——Down 离开侧 end ≥ zd / Up 离开侧 end ≤ zg，
/// 与 judge_pan_div `reenters` 同一判据）。λ = 边界之后首个同向段的 start_index；无回中枢段 ⟹
/// 窗口首个同向段（整窗口一个 episode）；边界后无同向段 ⟹ None（episode 无同向体，诚实无定位）。
///
/// 生产（signal.rs extract 循环 λ_C）/漏斗探针/oracle/judge_pan_div 全部经本函数取 episode 起点
/// ——单一来源，无坐标 fork（675号）。
/// L0/测试便捷（Q7-#1 裁定C）：段方向即锚方向——L0 线段有内在缠论方向，锚资格 ≡ 结构方向。
pub fn self_anchors(segs: &[Segment]) -> Vec<Option<Direction>> {
    segs.iter().map(|s| Some(s.direction)).collect()
}

/// ★Q7-#1 裁定C（codex-q7-fallback-20260703）：`anchors` 与 `win` 平行——离开段（首同向段）
/// 选取用 anchor 方向（`anchors[j] == Some(dir)`），fallback 单元（None）不得作离开段方向锚。
/// 回中枢段边界（reenters）用结构方向（`s.direction`）——回中枢是几何角色，非裁决三锚之一。
pub fn episode_start_in(
    win: &[Segment],
    anchors: &[Option<Direction>],
    c: &Center,
    dir: Direction,
) -> Option<usize> {
    let reenters = |s: &Segment| {
        s.direction != dir
            && match dir {
                Direction::Down => s.end_price >= c.zd,
                Direction::Up => s.end_price <= c.zg,
            }
    };
    let boundary = win.iter().rev().find(|s| reenters(s)).map_or(0, |r| r.end_index);
    win.iter()
        .zip(anchors)
        .find(|(s, a)| **a == Some(dir) && s.start_index >= boundary)
        .map(|(s, _)| s.start_index)
}

/// λ_C（Q5 + codex ac4 审查 #2 修复）：离开中枢 `c` 的**当前** episode 首同向段起点，窗口截至
/// `until_start`（含——判破段 seg 自身在窗口内）。窗口 = start_index ∈ [c.end_index, until_start]
/// 的段；episode 定界经 [`episode_start_in`]（回中枢段边界，单一来源）。
///
/// 旧实现（c_start_cache 按 c_idx 缓存「中枢后第一个同向段起点」）无 reentry 检测——「失败离开→
/// 回中枢→重新离开触发 broke」场景下 λ_C 过早，污染 I(C) 面积（codex #2 致命）。本函数按
/// (c, dir, until_start) 逐段计算（λ_C 依赖 seg 前的回中枢段集合，不再可按 c_idx 缓存）。
pub fn departure_move_c_start(
    segments: &[Segment],
    anchors: &[Option<Direction>],
    c: &Center,
    dir: Direction,
    until_start: usize,
) -> Option<usize> {
    let lo = segments.partition_point(|s| s.start_index < c.end_index);
    let hi = segments.partition_point(|s| s.start_index <= until_start);
    episode_start_in(&segments[lo..hi], &anchors[lo..hi], c, dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctr(zd: Tick, zg: Tick, dd: Tick, gg: Tick, ei: usize) -> Center {
        Center { zd, zg, dd, gg, start_index: 0, end_index: ei }
    }
    use super::super::super::types::Tick;

    #[test]
    fn ema_first_value_is_first_close() {
        // EMA 首值取首 close（reference-theta-v0.md:37）。
        let src = vec![10.0, 20.0, 30.0];
        let out = ema(&src, 2);
        assert_eq!(out[0], 10.0); // 首值 = src[0]
        // α=2/3：out[1]=2/3*20+1/3*10=16.666...
        let alpha = 2.0 / 3.0;
        assert!((out[1] - (alpha * 20.0 + (1.0 - alpha) * 10.0)).abs() < 1e-12);
    }

    #[test]
    fn ema_empty_yields_empty() {
        assert_eq!(ema(&[], 12), Vec::<f64>::new());
    }

    #[test]
    fn macd_series_equal_length() {
        let closes: Vec<f64> = (0..50).map(|i| 100.0 + (i as f64)).collect();
        let cfg = MacdConfig::default();
        let m = compute_macd(&closes, &cfg);
        assert_eq!(m.dif.len(), closes.len());
        assert_eq!(m.dea.len(), closes.len());
        assert_eq!(m.hist.len(), closes.len());
    }

    #[test]
    fn macd_hist_equals_dif_minus_dea() {
        // hist = DIF - DEA（reference-theta-v0.md:37）逐 bar 验证。
        let closes: Vec<f64> = (0..40).map(|i| 100.0 + (i as f64 * 0.5)).collect();
        let cfg = MacdConfig::default();
        let m = compute_macd(&closes, &cfg);
        for i in 0..m.hist.len() {
            assert!((m.hist[i] - (m.dif[i] - m.dea[i])).abs() < 1e-12);
        }
    }

    #[test]
    fn segment_area_sums_abs_hist() {
        let hist = vec![1.0, -2.0, 3.0, -4.0];
        // [0,2]：|1|+|-2|+|3|=6。
        assert_eq!(segment_macd_area(&hist, 0, 2), 6.0);
        // [1,3]：2+3+4=9。
        assert_eq!(segment_macd_area(&hist, 1, 3), 9.0);
    }

    #[test]
    fn segment_area_out_of_bounds_zero() {
        let hist = vec![1.0, 2.0];
        assert_eq!(segment_macd_area(&hist, 0, 5), 0.0); // end 越界
        assert_eq!(segment_macd_area(&hist, 3, 1), 0.0); // start>end
    }

    #[test]
    fn divergence_strict_less_only() {
        // 严格变小 ⟹ 背驰（reference-theta-v0.md:37）。
        assert!(is_divergence(10.0, 5.0));
        // 等值 ⟹ 不成立（等值不成立的明确规则）。
        assert!(!is_divergence(10.0, 10.0));
        // 变大 ⟹ 不成立。
        assert!(!is_divergence(10.0, 15.0));
    }

    #[test]
    fn segments_diverge_end_to_end() {
        // 前段 [0,1] 面积大，后段 [2,3] 面积小 ⟹ 背驰。
        let hist = vec![5.0, -5.0, 1.0, -1.0];
        assert!(segments_diverge(&hist, (0, 1), (2, 3))); // 10 vs 2 → 严格小
        // 反向：后段面积大 ⟹ 不背驰。
        assert!(!segments_diverge(&hist, (2, 3), (0, 1)));
    }

    /// property：面积非负（|hist| 之和）。
    #[test]
    fn property_area_nonnegative() {
        let closes: Vec<f64> = (0..30).map(|i| 100.0 + ((i * 7 % 13) as f64)).collect();
        let cfg = MacdConfig::default();
        let m = compute_macd(&closes, &cfg);
        for start in 0..m.hist.len() {
            for end in start..m.hist.len() {
                assert!(segment_macd_area(&m.hist, start, end) >= 0.0);
            }
        }
    }

    // ── § B. A/B/C 框架层：趋势背驰 A 段定位（locate_trend_seg_a）─────────────────

    #[test]
    fn locate_departure_move_a_covers_whole_interval() {
        // Q5：趋势 τ=Trend(Down)，prev_center end=5，last_center end=12。多段离开走势——
        // I(A) = (首个匹配段起点, 末个匹配段终点) = (6, 11)，覆盖中间反向段 [8,10] 的 bar。
        let prev_c = ctr(300, 400, 290, 410, 5);
        let last_c = ctr(100, 200, 90, 210, 12);
        let segments = vec![
            // start_index 升序。向下段在 [5,12) 区间内（离开走势的组成段）。中间反向段端点 295
            // < prev_c.zd=300——趋势内部正常回撤，未回中枢核心 ⟹ 同一 episode（codex ac4 #1）。
            Segment { direction: Direction::Down, start_index: 6, end_index: 8, start_price: 380, end_price: 280 },
            Segment { direction: Direction::Up, start_index: 8, end_index: 10, start_price: 280, end_price: 295 },
            Segment { direction: Direction::Down, start_index: 10, end_index: 11, start_price: 295, end_price: 250 },
            Segment { direction: Direction::Down, start_index: 13, end_index: 15, start_price: 200, end_price: 80 }, // 在 last_center 之后（C 区，非 A）
        ];
        let seg_a = locate_departure_move_a(&segments, &self_anchors(&segments), &prev_c, &last_c, Direction::Down);
        assert_eq!(seg_a, Some((6, 11)), "Q5：A = 整个离开走势区间（首匹配段起点..末匹配段终点，含中间反向段）");
    }

    #[test]
    fn locate_departure_move_a_reentry_splits_episodes() {
        // codex ac4 #1 修复见证：失败离开（[6,8]）→ 回中枢段（[8,10] 端点 395 回到 prev core
        // [300,400] 内侧 ≥ zd=300）→ 重新离开（[10,11]）。两个独立 episode 不桥接——
        // A = 最后 episode [10,11]，非旧桥接区间 [6,11]。
        let prev_c = ctr(300, 400, 290, 410, 5);
        let last_c = ctr(100, 200, 90, 210, 12);
        let segments = vec![
            Segment { direction: Direction::Down, start_index: 6, end_index: 8, start_price: 380, end_price: 280 },
            Segment { direction: Direction::Up, start_index: 8, end_index: 10, start_price: 280, end_price: 395 }, // 回中枢：end 395 ≥ zd=300
            Segment { direction: Direction::Down, start_index: 10, end_index: 11, start_price: 395, end_price: 250 },
        ];
        let seg_a = locate_departure_move_a(&segments, &self_anchors(&segments), &prev_c, &last_c, Direction::Down);
        assert_eq!(seg_a, Some((10, 11)), "回中枢段切开两个 episode ⟹ A = 当前（最后）episode，不桥接");
        // 对照：中间反向段未回核心（end 280 < zd=300）⟹ 同一 episode ⟹ 全区间（covers_whole_interval 语义）。
        let no_reentry = vec![
            Segment { direction: Direction::Down, start_index: 6, end_index: 8, start_price: 380, end_price: 280 },
            Segment { direction: Direction::Up, start_index: 8, end_index: 10, start_price: 280, end_price: 295 },
            Segment { direction: Direction::Down, start_index: 10, end_index: 11, start_price: 295, end_price: 250 },
        ];
        let seg_a2 = locate_departure_move_a(&no_reentry, &self_anchors(&no_reentry), &prev_c, &last_c, Direction::Down);
        assert_eq!(seg_a2, Some((6, 11)), "反向段未回核心 ⟹ 同一 episode ⟹ 整区间");
    }

    #[test]
    fn departure_move_c_start_reentry_bounds_episode() {
        // codex ac4 #2 修复见证：中枢后失败离开 [6,8] → 回中枢段 [8,10]（end 395 ≥ zd=300）→
        // 重新离开 [10,11]（判破段）。λ_C = 10（当前 episode 起点），非旧口径 6（首个同向段）。
        let c = ctr(300, 400, 290, 410, 5);
        let segments = vec![
            Segment { direction: Direction::Down, start_index: 6, end_index: 8, start_price: 380, end_price: 280 },
            Segment { direction: Direction::Up, start_index: 8, end_index: 10, start_price: 280, end_price: 395 },
            Segment { direction: Direction::Down, start_index: 10, end_index: 11, start_price: 395, end_price: 250 },
        ];
        assert_eq!(departure_move_c_start(&segments, &self_anchors(&segments), &c, Direction::Down, 10), Some(10),
            "回中枢段之后重新离开 ⟹ λ_C = 当前 episode 首同向段起点");
        // 无回中枢段 ⟹ 整窗口一个 episode ⟹ λ_C = 首个同向段起点（单段兼容口径）。
        assert_eq!(departure_move_c_start(&segments[..1], &self_anchors(&segments[..1]), &c, Direction::Down, 6), Some(6),
            "无回中枢段 ⟹ λ_C = 中枢后首个同向段起点");
    }

    #[test]
    fn locate_departure_move_a_single_segment_bit_compatible() {
        // Q5 兼容性：恰一个匹配段 ⟹ 返回值与旧单段实现 bit 相同。
        let prev_c = ctr(300, 400, 290, 410, 5);
        let last_c = ctr(100, 200, 90, 210, 12);
        let segments = vec![
            Segment { direction: Direction::Down, start_index: 6, end_index: 8, start_price: 380, end_price: 280 },
            Segment { direction: Direction::Up, start_index: 8, end_index: 10, start_price: 280, end_price: 295 }, // 未回核心（<zd=300）
        ];
        let seg_a = locate_departure_move_a(&segments, &self_anchors(&segments), &prev_c, &last_c, Direction::Down);
        assert_eq!(seg_a, Some((6, 8)), "单匹配段 ⟹ 与旧单段返回值 bit 相同（兼容）");
    }

    #[test]
    fn locate_departure_move_a_none_when_no_prev_leave() {
        // prev_center 与 last_center 之间无同向离开段 ⟹ A 区间无法定位 ⟹ None（无背驰对照）。
        let prev_c = ctr(300, 400, 290, 410, 5);
        let last_c = ctr(100, 200, 90, 210, 12);
        let segments = vec![
            // [5,12) 内只有向上段，无向下离开段 ⟹ A（向下）无候选。
            Segment { direction: Direction::Up, start_index: 6, end_index: 8, start_price: 280, end_price: 380 },
        ];
        let seg_a = locate_departure_move_a(&segments, &self_anchors(&segments), &prev_c, &last_c, Direction::Down);
        assert_eq!(seg_a, None, "无 prev_center 同向离开段 ⟹ A 无法定位");
    }

    #[test]
    fn abc_divergence_trend_diverges_via_area() {
        // AbcDivergence 趋势背驰：A 段面积大、C 段面积小 ⟹ 背驰（C<A 力度原语）。
        let hist = vec![5.0, -5.0, 1.0, -1.0]; // A=[0,1] 面积10，C=[2,3] 面积2
        let abc = AbcDivergence { seg_a: (0, 1), seg_c: (2, 3), is_trend: true };
        assert!(abc.diverges(&hist, (0, 1), (2, 3)), "C段面积2 < A段面积10 ⟹ 趋势背驰");
        // 反向：C 段面积大 ⟹ 力度延续 ⟹ 非背驰。
        let abc_cont = AbcDivergence { seg_a: (2, 3), seg_c: (0, 1), is_trend: true };
        assert!(!abc_cont.diverges(&hist, (2, 3), (0, 1)), "C段面积10 ≥ A段面积2 ⟹ 力度延续=非背驰");
    }

    // ===== § A2. 多力度原语 + Weak_Θ 词典序（P2 §4/§7 改点5，L1 接口正确性）=====

    #[test]
    fn dif_peak_up_takes_positive_max_down_takes_abs_min() {
        // 向上段：黄白线正峰（max）。dif=[1,3,2] ⟹ 峰=3。
        let dif = vec![1.0, 3.0, 2.0, -1.0];
        assert_eq!(segment_dif_peak(&dif, 0, 2, Direction::Up), 3.0);
        // 向下段：黄白线负峰（|min|）。dif=[−1,−4,−2] ⟹ 峰=|−4|=4。
        let dif2 = vec![-1.0, -4.0, -2.0];
        assert_eq!(segment_dif_peak(&dif2, 0, 2, Direction::Down), 4.0);
        // 越界 ⟹ 0。
        assert_eq!(segment_dif_peak(&dif, 0, 10, Direction::Up), 0.0);
        // 向上段但区间全负 ⟹ 正峰 = 0（max(neg,0)=0，无正黄白线）。
        assert_eq!(segment_dif_peak(&dif2, 0, 2, Direction::Up), 0.0);
    }

    #[test]
    fn price_amplitude_and_speed_from_close_endpoints() {
        // 振幅 = |端价差|（整数 tick）。closes[0..=3]=[100,?,?,150] ⟹ |150−100|=50。
        let closes: Vec<Tick> = vec![100, 120, 90, 150];
        assert_eq!(segment_price_amplitude(&closes, 0, 3), 50);
        // 速度 = 振幅 / 跨度 = 50 / 3。
        assert!((segment_price_speed(&closes, 0, 3) - (50.0 / 3.0)).abs() < 1e-12);
        // 越界 ⟹ 0。
        assert_eq!(segment_price_amplitude(&closes, 0, 9), 0);
        assert_eq!(segment_price_speed(&closes, 0, 9), 0.0);
        // TV = 20+30+60 = 110（路径长度 > 端点差 50——折返段被计入）。
        assert_eq!(segment_total_variation(&closes, 0, 3), 110);
        assert_eq!(segment_total_variation(&closes, 0, 9), 0);
        assert_eq!(segment_total_variation(&closes, 2, 2), 0);
    }

    fn ff(area: f64, dif: f64, amp: i64, speed: f64) -> ForceFeatures {
        // tv 默认随振幅（单调一致，不给既有支配序测试引入额外冲突维）。
        ForceFeatures {
            macd_area: area,
            dif_peak: dif,
            price_amplitude: amp,
            price_speed: speed,
            tv: amp,
        }
    }

    #[test]
    fn weak_theta_each_mode_compares_correct_proxy() {
        // A 段力度大，C 段力度小 ⟹ 各 mode 都判 Weak（背驰=力度衰减）。
        let a = ff(10.0, 8.0, 100, 20.0);
        let c = ff(5.0, 4.0, 50, 10.0);
        assert!(weak_theta(WeakThetaMode::MacdArea, &a, &c), "C.area<A.area ⟹ Weak");
        assert!(weak_theta(WeakThetaMode::Dif, &a, &c), "C.dif<A.dif ⟹ Weak");
        assert!(weak_theta(WeakThetaMode::PriceAmplitude, &a, &c), "C.amp<A.amp ⟹ Weak");
        // C 力度 ≥ A ⟹ 各 mode 非 Weak（力度延续）。
        assert!(!weak_theta(WeakThetaMode::MacdArea, &c, &a));
        assert!(!weak_theta(WeakThetaMode::Dif, &c, &a));
    }

    #[test]
    fn force_state_five_dominance_cases() {
        let fp = |a, c| ForceProxies { seg_a: a, seg_c: c };
        let strong = ff(10.0, 8.0, 100, 20.0);
        let weak = ff(5.0, 4.0, 50, 10.0);
        // C 全弱于 A ⟹ Dominated（确定背驰）。
        assert_eq!(fp(strong, weak).force_state(), ForceStateA5::Dominated);
        // C 全强于 A ⟹ Dominates。
        assert_eq!(fp(weak, strong).force_state(), ForceStateA5::Dominates);
        // 全等 ⟹ Tie。
        assert_eq!(fp(strong, strong).force_state(), ForceStateA5::Tie);
        // 口径冲突：C.area 弱但 C.dif 强 ⟹ Incomparable。
        let mixed_a = ff(10.0, 4.0, 100, 20.0);
        let mixed_c = ff(5.0, 8.0, 50, 10.0);
        assert_eq!(fp(mixed_a, mixed_c).force_state(), ForceStateA5::Incomparable);
    }

    /// ★Lex 词典序核心（第17课「黄白线主 ▷ 面积次」，非 AND/OR）：DIF 可判用 DIF，DIF 相等退面积。
    #[test]
    fn weak_theta_lex_dif_dominates_area_secondary() {
        // (1) DIF 严格可判：C.dif<A.dif ⟹ Weak，**即使面积相反**（DIF 主判据压过面积）。
        let a1 = ff(5.0, 8.0, 0, 0.0);  // A: 面积小、DIF 大
        let c1 = ff(10.0, 4.0, 0, 0.0); // C: 面积大、DIF 小
        assert!(weak_theta(WeakThetaMode::Lex, &a1, &c1),
            "DIF 主：C.dif(4)<A.dif(8) ⟹ Weak，即使 C.area(10)>A.area(5)（黄白线压过面积）");
        // 对照：纯面积 mode 会判非 Weak（面积延续）——证明 Lex ≠ 纯面积。
        assert!(!weak_theta(WeakThetaMode::MacdArea, &a1, &c1),
            "纯面积：C.area(10)≥A.area(5) ⟹ 非 Weak（与 Lex 分歧，证 DIF 主判据生效）");

        // (2) DIF 相等（不可判）⟹ 退面积次判据：C.area<A.area ⟹ Weak。
        let a2 = ff(10.0, 5.0, 0, 0.0);
        let c2 = ff(4.0, 5.0, 0, 0.0); // DIF 相等（5==5）⟹ 退面积
        assert!(weak_theta(WeakThetaMode::Lex, &a2, &c2),
            "DIF 相等 ⟹ 退面积次判据：C.area(4)<A.area(10) ⟹ Weak");
        // DIF 相等 + 面积也延续 ⟹ 非 Weak。
        let c3 = ff(20.0, 5.0, 0, 0.0);
        assert!(!weak_theta(WeakThetaMode::Lex, &a2, &c3),
            "DIF 相等 + C.area(20)≥A.area(10) ⟹ 非 Weak");
    }

    #[test]
    fn force_features_assembles_all_proxies() {
        // hist=[3,-3,1,-1]（A[0,1] 面积6，C[2,3] 面积2），dif=[2,4,1,0.5]，closes=[100,110,105,102]。
        let hist = vec![3.0, -3.0, 1.0, -1.0];
        let dif = vec![2.0, 4.0, 1.0, 0.5];
        let closes: Vec<Tick> = vec![100, 110, 105, 102];
        let a = force_features(&hist, &dif, &closes, 0, 1, Direction::Up);
        assert_eq!(a.macd_area, 6.0);
        assert_eq!(a.dif_peak, 4.0); // up 段 max(dif[0..=1])=max(2,4)=4
        assert_eq!(a.price_amplitude, 10); // |110−100|
        assert_eq!(a.tv, 10); // Σ|Δ| = |110−100|（单跳段 TV=振幅）
        // Weak_Θ Lex：A=[0,1] vs C=[2,3]（C.dif_peak=max(1,0.5)=1<A.dif=4 ⟹ Weak）。
        let c = force_features(&hist, &dif, &closes, 2, 3, Direction::Up);
        assert!(weak_theta(WeakThetaMode::Lex, &a, &c), "C 段 DIF 峰(1)<A 段 DIF 峰(4) ⟹ Lex Weak");
    }

    // ===== 增量 MACD API（231号纯性能，bit-exact 对照全量版）=====

    /// bit-exact 核心：增量逐 bar 输出 == 全量版对应位置（合成数据）。
    #[test]
    fn incremental_macd_bit_exact_synthetic() {
        // 合成 closes：覆盖上升/震荡/下降，长度 > signal 周期使 EMA 充分递推。
        let closes: Vec<f64> = (0..80)
            .map(|i| 100.0 + 10.0 * ((i as f64) * 0.3).sin() + (i as f64) * 0.05)
            .collect();
        let cfg = MacdConfig::default();
        let full = compute_macd(&closes, &cfg);

        // 增量：init 首 bar，逐 bar append，逐位断言。
        let mut state = MacdState::init(closes[0], &cfg);
        // 首 bar 对照。
        let p0 = state.current_point();
        assert!(
            (p0.dif - full.dif[0]).abs() < 1e-12,
            "首 bar DIF 不匹配：增量={} 全量={}",
            p0.dif,
            full.dif[0]
        );
        assert!((p0.dea - full.dea[0]).abs() < 1e-12);
        assert!((p0.hist - full.hist[0]).abs() < 1e-12);

        for i in 1..closes.len() {
            state = compute_macd_append(&state, closes[i]);
            let p = state.current_point();
            assert!(
                (p.dif - full.dif[i]).abs() < 1e-12,
                "bar {} DIF 不匹配：增量={} 全量={}",
                i,
                p.dif,
                full.dif[i]
            );
            assert!(
                (p.dea - full.dea[i]).abs() < 1e-12,
                "bar {} DEA 不匹配：增量={} 全量={}",
                i,
                p.dea,
                full.dea[i]
            );
            assert!(
                (p.hist - full.hist[i]).abs() < 1e-12,
                "bar {} hist 不匹配：增量={} 全量={}",
                i,
                p.hist,
                full.hist[i]
            );
        }
        assert_eq!(state.bars_processed(), closes.len() as u64);
    }

    /// bit-exact 边界：首 bar 确定性（DIF=DEA=hist=0）。
    #[test]
    fn incremental_macd_first_bar_is_zero() {
        let cfg = MacdConfig::default();
        let state = MacdState::init(1234.5, &cfg);
        let p = state.current_point();
        assert_eq!(p.dif, 0.0, "首 bar DIF = close-close = 0");
        assert_eq!(p.dea, 0.0, "首 bar DEA = 首值取首 DIF = 0");
        assert_eq!(p.hist, 0.0, "首 bar hist = DIF-DEA = 0");
        assert_eq!(state.bars_processed(), 1);
    }

    /// bit-exact：macd_state_from_closes 末状态 == 逐 bar append 末状态。
    #[test]
    fn incremental_macd_from_closes_matches_append() {
        let closes: Vec<f64> = (0..30).map(|i| 50.0 + (i as f64)).collect();
        let cfg = MacdConfig::default();
        let from_helper = macd_state_from_closes(&closes, &cfg);
        // 手动逐 bar。
        let mut manual = MacdState::init(closes[0], &cfg);
        for &c in &closes[1..] {
            manual = manual.append(c);
        }
        assert_eq!(from_helper, manual);
    }

    /// bit-exact：跨不同 cfg 参数（fast/slow/signal 非默认）逐位相等。
    #[test]
    fn incremental_macd_bit_exact_custom_cfg() {
        let closes: Vec<f64> = (0..60)
            .map(|i| 200.0 + 5.0 * ((i as f64) * 0.7).cos())
            .collect();
        let cfg = MacdConfig { fast: 5, slow: 20, signal: 5 };
        let full = compute_macd(&closes, &cfg);
        let mut state = MacdState::init(closes[0], &cfg);
        for i in 1..closes.len() {
            state = compute_macd_append(&state, closes[i]);
            let p = state.current_point();
            assert!((p.dif - full.dif[i]).abs() < 1e-12, "custom cfg bar {} DIF", i);
            assert!((p.dea - full.dea[i]).abs() < 1e-12, "custom cfg bar {} DEA", i);
            assert!((p.hist - full.hist[i]).abs() < 1e-12, "custom cfg bar {} hist", i);
        }
    }

    /// 标度：增量 append 单次成本应近似常数（exp≈1），相对全量 compute_macd 的 O(n)。
    ///
    /// 用单 bar append 的固定耗时 vs 全量 compute_macd 在增大 n 下的线性增长比，
    /// 断言 append 耗时不随已处理 bar 数增长（O(1) per bar）。
    #[test]
    fn incremental_macd_append_is_o1_scaling() {
        let cfg = MacdConfig::default();
        // 在两个不同规模下测量「再 append 1 bar」的耗时——O(1) 则两者相近。
        let bench = |n_pre: usize| -> u128 {
            let closes_pre: Vec<f64> =
                (0..n_pre).map(|i| 100.0 + ((i as f64) * 0.1).sin()).collect();
            let state0 = macd_state_from_closes(&closes_pre, &cfg);
            let new_close = 105.0;
            // 重复 append（丢弃，仅测单次 append 在已处理 n_pre 后的耗时）。
            let start = std::time::Instant::now();
            let iters = 50_000;
            let mut s = state0.clone();
            for _ in 0..iters {
                s = s.append(new_close);
            }
            let _ = s.current_point();
            start.elapsed().as_nanos() / (iters as u128)
        };
        let per_bar_small = bench(100);
        let per_bar_large = bench(5_000);
        // O(1) 判据：大样本 per-bar 耗时不应显著高于小样本。
        // 容忍 3x 噪声（bench 环境抖动），真正 O(n) 会差 ~50x。
        let ratio = per_bar_large as f64 / per_bar_small.max(1) as f64;
        assert!(
            ratio < 3.0,
            "append 非 O(1)：小样本 {} ns/bar，大样本 {} ns/bar，比 {}",
            per_bar_small,
            per_bar_large,
            ratio
        );
    }
}
