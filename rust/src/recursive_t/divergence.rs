//! T 步骤 c)：走势完美判定（背驰），三模式入口保留（[`PerfectionMode`]，见下）。
//!
//! 设计文档 §6：背驰是 T 中唯一的度量剩余。★#1243（2026-08-26）起力度分量**只保留
//! ForceL 一套**（L(段)=速度净增量，#873 经 #989 段→笔反查），与生产默认判据
//! （theta_v0 `div_cand` ForceL 档，#990 I-2）同口径：
//! - **结构滤网 F**：第37课趋势背驰5条件的结构判据（条件2/3/5，`inner_zhongshu_count`）；
//! - **力度衰减 S**：`L(c) < L(a)`（ForceL，#873 教义正本）。
//!
//! **M（MACD 面积）已撤出判据**（#1243 裁定一：`leg_strength`/`leg_macd_force` 从判据
//! 消费面全退役——#991 I-3 已撤其教义地位，留判据里 = 用已退役量判定）。三模式入口
//! （`Structural`/`And`/`Or`，编排者 2026-06-18 裁决「三条路实测」的历史遗留）现在
//! **同口径 = `G∧(F∧S)`**：`And`/`Or` 不再引入 M，仅保留 API 兼容。几何门 G（≥2 中枢
//! + c 创新高/新低）不可绕过——BSP 价格/bar 锚定 c 段趋势极值，无创新高则无从定位一类买卖点。
//!
//! 第24课：「用均线或MACD看背驰都是辅助性的……配合上中枢，那是 100% 绝对的，
//! 因为这可以用纯数学的推理逻辑地证明。」
//!
//! ## 第37课趋势背驰5条件（`judge_trend_divergence` 全实装）
//!
//! 趋势 a+A+b+B+c 形式，A、B 锚定**最后两个同级别中枢**（A=倒数第二中枢=背驰背景，
//! B=末中枢；第24课「A 之前已有一个中枢，B 是另一个中枢」，≥3 中枢趋势同此）：
//! 1. A、B 是同级别中枢 —— `zhongshus.len() ≥ 2`；
//! 2. c 含对 B 的第三类买卖点 —— c 段回抽不破 B 的 ZG(上)/ZD(下)（守沿分量；「先离开」
//!    由 4+5 回补，「第一次回抽」序数未单独编码——有效域边界）；
//! 3. b 在级别上不大于 c —— b 段（A↔B 连接段）嵌套深度 ≤ c 段；
//! 4. 上涨 c 创新高 / 下跌 c 创新低 —— c 段极值超越此前全部极值；
//! 5. c 段含 ≥2 个次级别中枢 —— c 段嵌套深度和 ≥2（只验中枢计数，未验依次同向构成趋势）。
//! + 第24课力度：c 段力度 < a 段（ForceL 速度净增量衰减 `L(c) < L(a)`，#1243）。
//!
//! 条件 2/3/5 是 79% 伪背驰（buysellpoint.rs:460-465 v3 raw type1「下跌途中连续刷新低
//! 的伪底背驰」）的结构滤网。基底层（a₀，无中枢嵌套）按第64课退化为类背驰（仅 1+4+力度）。
//! 中枢锚定/守沿/中枢计数的有效域边界由对抗核验（6 验证者）确立，见 §6.7 决策记录。
//!
//! ## 力度度量（#873 教义正本，ForceL）
//!
//! 背驰 = 走势末段相对前一同向段「力度衰减」。★#1243 起力度只用一套 ForceL：
//! `L(段) = v(末笔) − v(首笔)`（速度净增量，物理上即单位质量冲量 `m·Δv`），速度在
//! 结构给定的最小单元（笔 = 本走势的 `Unit` 切片单元）上量：`v = (high−low) ÷ bar 数`
//! （沿走势方向速率，恒正）。同口径对拍锁见 [`unit_velocity`] 测试。
//!
//! 旧「level=0 用振幅、level≥1 用嵌套深度」两档力度（`leg_strength`）已随 #1243 退役
//! 删除；`inner_zhongshu_count` 现只供结构滤网 F（条件2/3/5）使用，不再参与力度比较。
//!
//! GUARD-ROLE: standalone-t-loadbearing-for-t-engine
//!
//! ★#1243 收编标注（2026-08-26，G2 #978 裁定二/四 + #991 I-3 后续）：`leg_strength`/
//! `leg_macd_force` **已从判据消费面删除**（#991 I-3 撤其教义地位，「中枢个数律」引证被
//! #874 推翻）。力度分量统一 ForceL（#873 经 #989 段→笔反查），与生产默认判据同口径。
//! `leg_*` 的历史读数探针保留在测试/诊断侧（如 `t_engine_run` 的本地复刻），不进任何判定。
//!
//! ## 名分（`docs/agents/generation-constitution.md` §1 名分五态，#761 C7-E2 执行票订正）
//!
//! - **名分**：**现役**——本文件的 `trend_diverging_segment`/`d_top`/
//!   `CONSOL_DOWN_DIAG` 在 `rec_stream.rs:313,353,375,1408` 生产路径被直接调用，
//!   是同目录 recursive_t/(b) T 引擎（12062 行，E3 处置范围）的编译期硬依赖，
//!   详细证据/理由见 `center.rs` 头部同名 GUARD-ROLE 块（避免重复，此处不复述）。
//! - **禁回灌**：本次仅加标记，未删除/未移动任何代码。

use super::types::{BSPKind, Direction, PerfectionMode, TrendKind, TrendType, Unit, Zhongshu, BSP};

// 诊断（编排者 2026-06-21）：盘整背驰 Down 失败原因计数（查 L4 ConsolDown type1_buy=0 根因）。
// [level][0=no_new_low(离开不创新低), 1=no_force_decay(力度不衰减), 2=produced(产 type1_buy),
//         3=no_leave(无离开段=走势未离开中枢=未完成)]。
thread_local! {
    pub static CONSOL_DOWN_DIAG: std::cell::RefCell<[[u64; 4]; 9]> =
        const { std::cell::RefCell::new([[0; 4]; 9]) };
    /// d_top 链贯通诊断（[cc][0=非趋势,1=无背驰,2=极性不符,3=无c段窗,4=向下链断,5=链贯通)）。
    pub static D_TOP_DIAG: std::cell::RefCell<[[u64; 6]; 9]> =
        const { std::cell::RefCell::new([[0; 6]; 9]) };
}

/// 单元（笔）速度（#873 教义正本）：沿走势方向速率，恒正 = 涨跌幅度 ÷ bar 数。
///
/// 与生产 `theta_v0::parser::segment::stroke_velocity` 同口径（同输入同输出对拍锁见
/// tests）：向上笔 `(high−low)/bars`、向下笔同样 `(high−low)/bars`——速率非带号速度。
fn unit_velocity(u: &Unit) -> f64 {
    let bars = (u.end_bar - u.start_bar + 1) as f64;
    (u.high - u.low) / bars
}

/// 力度 `L(段) = v(末笔) − v(首笔)`（#873 教义正本；#989 段→笔反查）。
///
/// 段的「笔」= 本走势的单元切片（`units[a..=b]`），首/末笔即切片首/末单元；bar 闭区间
/// `[首单元.start_bar, 末单元.end_bar]` 反查笔区间 = `[首, 末]`（切片连续，无空间隙）。
/// 调用方保证切片非空（judge_* 边界防御先滤空段）。
fn force_l(units: &[Unit]) -> f64 {
    unit_velocity(units.last().unwrap()) - unit_velocity(units.first().unwrap())
}

/// 一段单元的极值（最高 high / 最低 low）。
fn leg_extreme(units: &[Unit], dir: Direction) -> (f64, i64) {
    match dir {
        Direction::Up => {
            // 取最高 high；该极值 bar：上行单元高点在 end_bar，下行单元高点在 start_bar。
            let mut best = f64::MIN;
            let mut bar = 0i64;
            for u in units {
                if u.high > best {
                    best = u.high;
                    bar = if u.direction == Direction::Up {
                        u.end_bar
                    } else {
                        u.start_bar
                    };
                }
            }
            (best, bar)
        }
        Direction::Down => {
            let mut best = f64::MAX;
            let mut bar = 0i64;
            for u in units {
                if u.low < best {
                    best = u.low;
                    bar = if u.direction == Direction::Down {
                        u.end_bar
                    } else {
                        u.start_bar
                    };
                }
            }
            (best, bar)
        }
    }
}

/// 步骤 c)：判定走势是否终完美（背驰），若是则产生 type1 买卖点（`mode` 保留 API 兼容）。
///
/// 趋势背驰（第37课，需 ≥2 同级别中枢 A、B）：
/// 1. `a` 段 = 进入段（首中枢之前 + 累积至倒数第二中枢 A，作为力度基准）；
/// 2. `c` 段 = 末中枢 B 之后的单元（离开段）；
/// 3. 几何门 G：≥2 中枢 + 条件4（`c` 创新高/新低，超越此前全部极值）；
/// 4. 结构滤网 F∧S：第37课条件2·3·5（has_nest 时）+ ForceL 力度衰减（#1243）。
///
/// 盘整背驰（单中枢）：进入段 vs 离开段同样比较，离开创新极值；F 为空（盘整无条件
/// 2·3·5），故结构判据仅含力度衰减 S=ForceL。
pub fn judge_divergence(t: &TrendType, mode: PerfectionMode) -> Option<BSP> {
    match t.kind {
        TrendKind::UpTrend | TrendKind::DownTrend => judge_trend_divergence(t, mode),
        TrendKind::Consolidation => judge_consolidation_divergence(t, mode),
    }
}

fn judge_trend_divergence(t: &TrendType, _mode: PerfectionMode) -> Option<BSP> {
    let n_centers = t.zhongshus.len();
    if n_centers < 2 {
        return None; // 几何门 G·条件1：趋势背驰需 ≥2 同级别中枢。
    }
    // A = 倒数第二中枢（背驰背景，第24课「A 之前已有一个中枢，B 是这个大趋势的另一个
    // 中枢」），B = 末中枢。对 ≥3 中枢趋势，背驰段必须锚定**最后两个中枢**（与已验证
    // bit-exact 的 crate::divergence 的 indices[-2]/[-1] 一致）；旧首/末读法会把中间中枢
    // 折进 b 段使其膨胀、系统性过严错杀（对抗核验 major）。2 中枢时 n-2=0，与旧行为一致。
    let prev_center = &t.zhongshus[n_centers - 2];
    let last_center = &t.zhongshus[n_centers - 1];

    // pub fn 边界防御：中枢按第17课须 ≥3 单元（find_centers 保证），但 pub API 不能因
    // 外部构造空 units 的 Zhongshu 而 panic——返回 None 使「Option 永不 panic」契约诚实。
    if prev_center.units.is_empty() || last_center.units.is_empty() {
        return None;
    }

    // a 段：切片起点到倒数第二中枢末单元（进入段 + 累积至 A 的背驰背景，力度基准）。
    let a_end = *prev_center.units.last().unwrap();
    let a_leg = &t.units[0..=a_end];

    // c 段：末中枢末单元之后。
    let c_start = *last_center.units.last().unwrap() + 1;
    if c_start >= t.units.len() {
        return None; // 末中枢之后无离开段，走势尚未离开 → 未完成。
    }
    let c_leg = &t.units[c_start..];

    let (c_ext, c_bar) = leg_extreme(c_leg, t.direction);

    // 几何门 G·条件4：c 创新高/新低（超越 c 段之前的全部极值）。G 不可绕过
    // ——BSP 价格/bar 锚定 c 段趋势极值，无创新高则无从定位一类买卖点（设计文档 §6.6）。
    let prior = &t.units[0..c_start];
    let (prior_ext, _) = leg_extreme(prior, t.direction);
    let new_extreme = match t.direction {
        Direction::Up => c_ext > prior_ext,
        Direction::Down => c_ext < prior_ext,
    };
    if !new_extreme {
        return None;
    }

    // 结构滤网 F∧S（第37课条件2·3·5 + ForceL 力度衰减）。#1243：M 撤出判据，三模式同口径。
    let structural = trend_structural_filter(t, a_leg, c_leg, a_end, last_center);
    if !structural {
        return None;
    }

    let kind = match t.direction {
        Direction::Up => BSPKind::Type1Sell,
        Direction::Down => BSPKind::Type1Buy,
    };
    Some(BSP {
        kind,
        bar: c_bar,
        price: c_ext,
        level: t.level,
    })
}

/// **区间套 candidate（#1243 后 = 结构滤网，不含几何门 G）**：背驰候选 = `trend_structural_filter`
/// （F∧S 条件2/3/5 + ForceL 力度衰减）。**不含几何门 G（c 创新高）**——那是定位 BSP fire 的
/// 最终确认，candidate 是"c 段进行中接近顶"的候选状态，顶部进行中尚未创新高确认。
///
/// 历史「AND 双确认」（结构 ∧ MACD）已随 M 撤出判据（#1243）退役；现与
/// `trend_diverging_segment(t, Structural)` 同口径。
pub fn trend_candidate(t: &TrendType) -> bool {
    if !matches!(t.kind, TrendKind::UpTrend | TrendKind::DownTrend) {
        return false; // 仅趋势（盘整无 a/c 段力度对比）
    }
    let n_centers = t.zhongshus.len();
    if n_centers < 2 {
        return false; // 需 ≥2 中枢（a/c 段背景）
    }
    let prev_center = &t.zhongshus[n_centers - 2];
    let last_center = &t.zhongshus[n_centers - 1];
    if prev_center.units.is_empty() || last_center.units.is_empty() {
        return false;
    }
    let a_end = *prev_center.units.last().unwrap();
    let a_leg = &t.units[0..=a_end];
    let c_start = *last_center.units.last().unwrap() + 1;
    if c_start >= t.units.len() {
        return false; // c 段不存在（走势尚未离开末中枢）
    }
    let c_leg = &t.units[c_start..];
    // #1243：M 撤出判据 → candidate = 结构滤网 F∧S（ForceL 力度衰减）。
    trend_structural_filter(t, a_leg, c_leg, a_end, last_center)
}

/// **命题4 读法乙——背驰段判定（src-prop13 第27课区间套前提）**。
///
/// 背驰段 = **走势完美判据（[`judge_divergence`]）去掉几何门 G 的「c 创新高」分量**——即趋势已进入
/// 末段（≥2 中枢 + c 段存在 + ForceL 力度衰减），但**尚未创出最终新高/新低**（未走势完成）。
/// 源头审计 src-prop13：区间套的操作前提是「大级别**背驰段**」（未完成趋势的最后段），**严格⊊「走势
/// 完成」**——两者唯一差 = 几何门 G 的创新高确认。本函数即此差的形式化：与 [`judge_trend_divergence`]
/// **同 F∧S**，仅省去 `new_extreme` 检查 ⇒ 背驰段 ⊇ 走势完成（type1）。
///
/// **mode 一致性**（#1243）：三模式同口径 = 结构滤网 F∧S（ForceL），`_mode` 仅保留 API 兼容。
pub fn trend_diverging_segment(t: &TrendType, _mode: PerfectionMode) -> bool {
    match t.kind {
        TrendKind::UpTrend | TrendKind::DownTrend => {
            let n_centers = t.zhongshus.len();
            if n_centers < 2 {
                return false; // 需 ≥2 中枢（a/c 段背景）
            }
            let prev_center = &t.zhongshus[n_centers - 2];
            let last_center = &t.zhongshus[n_centers - 1];
            if prev_center.units.is_empty() || last_center.units.is_empty() {
                return false;
            }
            let a_end = *prev_center.units.last().unwrap();
            let a_leg = &t.units[0..=a_end];
            let c_start = *last_center.units.last().unwrap() + 1;
            if c_start >= t.units.len() {
                return false; // c 段不存在（走势尚未离开末中枢）
            }
            let c_leg = &t.units[c_start..];
            // 与 judge_trend_divergence 同 F∧S，**省去 new_extreme（创新高）门** = 背驰段 ⊋ 完成。
            trend_structural_filter(t, a_leg, c_leg, a_end, last_center)
        }
        TrendKind::Consolidation => {
            // 盘整背驰段（= judge_consolidation_divergence 去掉 new_extreme 门）：单中枢 + 离开段存在 +
            // ForceL 力度衰减。涌现塔最高级别绝大多数是盘整（首形成形态），其 type1 由盘整背驰产生
            // ⇒ 背驰段判据必须覆盖盘整，否则与 type1（含盘整背驰）口径失配（W 假性为 0）。
            if t.zhongshus.len() != 1 {
                return false;
            }
            let center = match t.zhongshus.first() {
                Some(c) if !c.units.is_empty() => c,
                _ => return false,
            };
            let enter_end = *center.units.last().unwrap();
            if enter_end + 1 >= t.units.len() {
                return false; // 无离开段（走势未离开中枢）
            }
            let enter_leg = &t.units[0..=enter_end];
            let leave_leg = &t.units[enter_end + 1..];
            force_l(leave_leg) < force_l(enter_leg)
        }
    }
}

/// 趋势背驰结构滤网 F∧S（几何门 G 已通过后）：第37课条件2·3·5（has_nest 时激活）+
/// ForceL 力度衰减（#1243）。返回 `true` ⟺ 结构完美。
///
/// 第64课：有中枢嵌套信息 → 真背驰（第37课5条件全检）；无 → 类背驰。
/// 「线段以下是没有中枢的……是类中枢/类背驰」（第64课）。a₀ 基底层单元
/// `inner_zhongshu_count=0`（无真次级别中枢），条件 2/3/5 不适用，退化为条件 1+4+力度
/// （类背驰）。level≥1 封装单元携带真中枢数 → 条件 2/3/5 激活（真背驰）。力度比较
/// 一律 ForceL（`force_l(c) < force_l(a)`），`inner_zhongshu_count` 只供 F 使用。
///
/// 真背驰是 79% 伪背驰（buysellpoint.rs:460-465「下跌途中连续刷新低的伪底背驰」）的
/// 结构滤网：单凭力度衰减（条件4+力度）= 背驰候选；加 2/3/5 = 走势完美。
fn trend_structural_filter(
    t: &TrendType,
    a_leg: &[Unit],
    c_leg: &[Unit],
    a_end: usize,
    last_center: &Zhongshu,
) -> bool {
    let has_nest = t.units.iter().any(|u| u.inner_zhongshu_count > 0);
    if has_nest {
        // 条件2：c 含对 B（末中枢）的第三类买卖点。完整 type3 = 先离开中枢、再回抽不破
        // ZG/ZD。本条只校验「回抽守住中枢边沿」这一**守沿分量**（∃ 单元不破 ZG/ZD）；
        // 「先离开」由条件4（c 创新高/新低）+ 条件5（c 含≥2 次级别中枢的离开走势）结构性
        // 回补，「第一次回抽」严格序数（第38课）未单独编码——对抗核验 minor，记为有效域边界。
        let c_holds_center_edge = c_leg.iter().any(|u| match t.direction {
            Direction::Up => u.low > last_center.high, // 回抽不破中枢上沿 ZG
            Direction::Down => u.high < last_center.low, // 回抽不破中枢下沿 ZD
        });
        if !c_holds_center_edge {
            return false;
        }
        // 条件5：c 段含 ≥2 个次级别(k-1)中枢（嵌套深度和 ≥2）。只校验中枢总数，不校验这
        // 2 个中枢「依次同向构成单一次级别趋势」（需对 c 段重跑 find_centers，当前未实装）
        // ——对抗核验 minor，避免声明膨胀（不声称「c 是次级别趋势」）。
        let c_nest: usize = c_leg.iter().map(|u| u.inner_zhongshu_count).sum();
        if c_nest < 2 {
            return false;
        }
        // 条件3：b 在级别上不大于 c——b 段（倒数第二中枢 A ↔ 末中枢 B 之间的连接段）嵌套
        // 深度 ≤ c 段。a_end 已锚定倒数第二中枢，故 ≥3 中枢时 b 段不再吞中间中枢。
        let b_start = a_end + 1;
        let b_endx = *last_center.units.first().unwrap(); // 末中枢首单元（不含）
        let b_nest: usize = if b_start < b_endx {
            t.units[b_start..b_endx]
                .iter()
                .map(|u| u.inner_zhongshu_count)
                .sum()
        } else {
            0 // A、B 紧邻，b 段为空（级别 0 ≤ c，自然满足）
        };
        if b_nest > c_nest {
            return false;
        }
    }

    // 力度衰减（#873 ForceL，#1243）：L(c) < L(a)——速度净增量衰减，与生产 theta_v0 ForceL 同口径。
    force_l(c_leg) < force_l(a_leg)
}

fn judge_consolidation_divergence(t: &TrendType, _mode: PerfectionMode) -> Option<BSP> {
    let diag_down = t.direction == Direction::Down && t.level < 9;
    if t.zhongshus.len() != 1 {
        return None;
    }
    let center = &t.zhongshus[0];
    if center.units.is_empty() {
        return None; // pub fn 边界防御（同 judge_trend_divergence）。
    }
    let enter_end = *center.units.last().unwrap();
    // 进入段 = 切片起点到中枢末单元；离开段 = 其后。
    if enter_end + 1 >= t.units.len() {
        if diag_down {
            CONSOL_DOWN_DIAG.with(|d| d.borrow_mut()[t.level][3] += 1); // 无离开段（走势未离开中枢）
        }
        return None;
    }
    let enter_leg = &t.units[0..=enter_end];
    let leave_leg = &t.units[enter_end + 1..];

    let (leave_ext, leave_bar) = leg_extreme(leave_leg, t.direction);
    let (enter_ext, _) = leg_extreme(enter_leg, t.direction);
    // 几何门 G：离开段创新极值（盘整背驰离开段须突破进入段极值）。G 不可绕过（定位 BSP）。
    let new_extreme = match t.direction {
        Direction::Up => leave_ext > enter_ext,
        Direction::Down => leave_ext < enter_ext,
    };
    if !new_extreme {
        if diag_down {
            CONSOL_DOWN_DIAG.with(|d| d.borrow_mut()[t.level][0] += 1); // 离开不创新低
        }
        return None;
    }

    // 盘整 F 为空（无条件2·3·5），结构判据仅力度衰减 S=ForceL（#1243：M 撤出判据）。
    let structural = force_l(leave_leg) < force_l(enter_leg);
    if !structural {
        if diag_down {
            CONSOL_DOWN_DIAG.with(|d| d.borrow_mut()[t.level][1] += 1); // 力度不衰减
        }
        return None;
    }
    if diag_down {
        CONSOL_DOWN_DIAG.with(|d| d.borrow_mut()[t.level][2] += 1); // 产 type1_buy
    }

    let kind = match t.direction {
        Direction::Up => BSPKind::Type1Sell,
        Direction::Down => BSPKind::Type1Buy,
    };
    Some(BSP {
        kind,
        bar: leave_bar,
        price: leave_ext,
        level: t.level,
    })
}

// ════════════════════════════ d_top 区间套链贯通（555 c段钻取，每级别独立腿触发器）════════════════════════════
//
// 任务18 编排者修正：读法乙 = 每级别自相似递归多重赋格，每级别独立腿消费**该级别 d_top**（区间套链贯通）。
// 唯一核心改动 = 触发器从「走势完成（judge_divergence，创新高）」换成「背驰段（trend_diverging_segment，去创新高）」。
// 故链函数参数化 `use_diverge`：false=走势完成链（556 读法B 基线）/ true=背驰段链（读法乙递归）。结构（链贯通）不变。

/// 级别信号：该级别走势是否产「真顶/真底」信号 + 极性（is_buy）。
/// `use_diverge=false`（走势完成）：`judge_divergence`（创新高确认）；`true`（背驰段）：`trend_diverging_segment`（去创新高）。
fn level_signal_is_buy(t: &TrendType, mode: PerfectionMode, use_diverge: bool) -> Option<bool> {
    if use_diverge {
        if trend_diverging_segment(t, mode) {
            Some(matches!(t.direction, Direction::Down)) // 底背驰段=买（下跌将反转）/ 顶背驰段=卖
        } else {
            None
        }
    } else {
        judge_divergence(t, mode).map(|b| b.kind.is_buy())
    }
}

/// 走势的 c 段（离开段）时间窗 `(start_bar, end_bar)`——区间套链的逐层范围。背驰/背驰段共用（c 段几何同）。
fn divergence_window(t: &TrendType) -> Option<(i64, i64)> {
    match t.kind {
        TrendKind::UpTrend | TrendKind::DownTrend => {
            let n = t.zhongshus.len();
            if n < 2 {
                return None;
            }
            let last_center = &t.zhongshus[n - 1];
            if last_center.units.is_empty() {
                return None;
            }
            let c_start = *last_center.units.last().unwrap() + 1;
            if c_start >= t.units.len() {
                return None;
            }
            let c_leg = &t.units[c_start..];
            let lo = c_leg.first()?.start_bar;
            let hi = c_leg.last()?.end_bar;
            Some((lo, hi))
        }
        TrendKind::Consolidation => {
            if t.zhongshus.len() != 1 {
                return None;
            }
            let center = &t.zhongshus[0];
            if center.units.is_empty() {
                return None;
            }
            let enter_end = *center.units.last().unwrap();
            if enter_end + 1 >= t.units.len() {
                return None;
            }
            let leave = &t.units[enter_end + 1..];
            let lo = leave.first()?.start_bar;
            let hi = leave.last()?.end_bar;
            Some((lo, hi))
        }
    }
}

/// 在父 c 段窗内选相应子背驰（段）：构造性嵌套（子窗 ⊊ 父窗，取末端最接近顶部者）。
fn select_child_in_window(
    candidates: &[TrendType],
    parent_win: (i64, i64),
    core_dir: Direction,
    want_buy: bool,
    mode: PerfectionMode,
    use_diverge: bool,
) -> Option<(i64, i64)> {
    let (plo, phi) = parent_win;
    let mut best: Option<(i64, i64)> = None;
    for t in candidates {
        if t.direction != core_dir {
            continue;
        }
        let w = match divergence_window(t) {
            Some(w) => w,
            None => continue,
        };
        let (lo, hi) = w;
        if !(plo <= lo && hi <= phi) {
            continue; // 子窗须内含于父窗
        }
        if (lo, hi) == (plo, phi) {
            continue; // 链坍缩到同一段不算钻取一层（no-patch：宁断不放松）
        }
        match level_signal_is_buy(t, mode, use_diverge) {
            Some(is_buy) if is_buy == want_buy => {}
            _ => continue, // 背驰(段)独立成立 + 极性匹配
        }
        match best {
            Some((blo, _)) if lo <= blo => {}
            _ => best = Some((lo, hi)),
        }
    }
    best
}

/// 区间套链贯通：cc 层背驰(段)成立 ∧ 向下逐层 c 段钻取，各层皆有相应子背驰(段)，直到下钻停止级 `min_level`。
///
/// 下钻停止判据（#817 N-2 裁定一/二，判据 + 参数分离）：成本门 ∧ 塔底，两道谁先到算谁——
/// `min_level = max(成本门级, 塔底级)`，向下钻到该级即止（本级仍须有子背驰(段)，更低级不要求）。
/// 塔底 = a₀ = 0（`recursive_t` 塔以 a₀ 为底，`T_A0` 只切 segment/stroke 不换级别号）；成本门据
/// 探针 #907「在现有塔 L0-L4 上不咬任何一级」⟹ 先到的是塔底 ⟹ 阶段一 `min_level = 0`。
/// 级别是参数、不许钉死（原 `(0..cc).rev()` 的常数 0 即被判违规的「必须降到 a0」，#817 N-2 说法③）。
pub fn nest_chain_complete(
    cc: usize,
    cc_trend: &TrendType,
    level_trends_all: &[&[TrendType]],
    core_dir: Direction,
    mode: PerfectionMode,
    use_diverge: bool,
    min_level: usize,
) -> bool {
    let want_buy = matches!(core_dir, Direction::Down);
    if cc_trend.direction != core_dir {
        return false;
    }
    match level_signal_is_buy(cc_trend, mode, use_diverge) {
        Some(is_buy) if is_buy == want_buy => {}
        _ => return false, // cc 层不背驰(段) / 极性不符
    }
    let mut parent_win = match divergence_window(cc_trend) {
        Some(w) => w,
        None => return false,
    };
    for k in (min_level..cc).rev() {
        let candidates = match level_trends_all.get(k) {
            Some(c) => *c,
            None => return false,
        };
        let child_win = match select_child_in_window(
            candidates,
            parent_win,
            core_dir,
            want_buy,
            mode,
            use_diverge,
        ) {
            Some(w) => w,
            None => return false, // 父窗内无子背驰(段) → 链断
        };
        parent_win = child_win;
    }
    true
}

/// **d_top(cc) 区间套链贯通真顶/真底**（每级别独立腿触发器，558/编排者修正）。
/// `use_diverge=false`：走势完成链（556 读法B 基线）；`true`：背驰段链（读法乙递归，触发更频繁可能解冻顶层）。
pub fn d_top(
    cc: usize,
    cc_trend: &TrendType,
    level_trends_all: &[&[TrendType]],
    core_dir: Direction,
    mode: PerfectionMode,
    use_diverge: bool,
    min_level: usize,
) -> bool {
    let rec = |i: usize| {
        if cc < 9 {
            D_TOP_DIAG.with(|d| d.borrow_mut()[cc][i] += 1);
        }
    };
    if !matches!(cc_trend.kind, TrendKind::UpTrend | TrendKind::DownTrend) {
        rec(0);
        return false;
    }
    let want_buy = matches!(core_dir, Direction::Down);
    if cc_trend.direction != core_dir {
        rec(2);
        return false;
    }
    match level_signal_is_buy(cc_trend, mode, use_diverge) {
        Some(is_buy) if is_buy == want_buy => {}
        Some(_) => {
            rec(2);
            return false;
        }
        None => {
            rec(1);
            return false;
        }
    }
    if divergence_window(cc_trend).is_none() {
        rec(3);
        return false;
    }
    let ok = nest_chain_complete(
        cc,
        cc_trend,
        level_trends_all,
        core_dir,
        mode,
        use_diverge,
        min_level,
    );
    if ok {
        rec(5);
    } else {
        rec(4);
    }
    ok
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recursive_t::types::Zhongshu;

    /// 构造单元（笔）：bar 跨度 1 ⟹ 速度 `v = high − low`（`unit_velocity` 的分母 = 1），
    /// 便于手工验算 ForceL。
    fn mk(low: f64, high: f64, bar: i64, d: Direction, nest: usize) -> Unit {
        Unit {
            high,
            low,
            start_bar: bar,
            end_bar: bar,
            direction: d,
            level: 0,
            inner_zhongshu_count: nest,
            area_pos: 0.0,
            area_neg: 0.0,
        }
    }

    /// 标准 2 中枢上涨趋势：z1=units[0..=2]、z2=units[4..=6]，尾部 `c_leg` 传入。
    ///
    /// 前缀笔速度：u0=20、u1=10、u2=10 ⟹ `L(a) = v(u2) − v(u0) = −10`。
    /// `prefix_nest=0` → 类背驰（基底层，无 2/3/5）；`>0` → 真背驰（has_nest）。
    /// z2.ZG=50：条件2 要求 c 段 ∃ 单元 low > 50（守沿）。
    fn 两中枢上涨趋势(prefix_nest: usize, c_leg: Vec<Unit>) -> TrendType {
        let mut units = vec![
            mk(10.0, 30.0, 0, Direction::Up, prefix_nest),   // v=20
            mk(12.0, 22.0, 1, Direction::Down, prefix_nest), // v=10
            mk(11.0, 21.0, 2, Direction::Up, prefix_nest),   // v=10
            mk(21.0, 41.0, 3, Direction::Up, prefix_nest),   // b 段 v=20
            mk(38.0, 48.0, 4, Direction::Down, prefix_nest), // v=10
            mk(40.0, 50.0, 5, Direction::Up, prefix_nest),   // v=10
            mk(39.0, 49.0, 6, Direction::Down, prefix_nest), // v=10
        ];
        units.extend(c_leg);
        let z1 = Zhongshu {
            high: 22.0,
            low: 12.0,
            gg: 30.0,
            dd: 10.0,
            units: vec![0, 1, 2],
            level: 1,
        };
        let z2 = Zhongshu {
            high: 50.0,
            low: 40.0,
            gg: 60.0,
            dd: 38.0,
            units: vec![4, 5, 6],
            level: 1,
        };
        TrendType {
            kind: TrendKind::UpTrend,
            zhongshus: vec![z1, z2],
            units,
            level: if prefix_nest > 0 { 2 } else { 0 },
            direction: Direction::Up,
            completed: false,
            bsp: None,
        }
    }

    /// 类背驰（基底层）：c 段 `L(c) = v(末)−v(首) = 1−12 = −11 < L(a) = −10` → 力度衰减背驰。
    #[test]
    fn 上涨趋势末段力度衰减创新高_产生一卖() {
        let t = 两中枢上涨趋势(
            0,
            vec![
                mk(48.0, 60.0, 7, Direction::Up, 0), // v=12
                mk(59.0, 60.0, 8, Direction::Up, 0), // v=1
            ],
        );
        let bsp = judge_divergence(&t, PerfectionMode::Structural).expect("应产生背驰买卖点");
        assert_eq!(bsp.kind, BSPKind::Type1Sell);
        assert_eq!(bsp.price, 60.0); // c 段新高
        assert_eq!(bsp.bar, 7);
        assert_eq!(bsp.level, 0);
    }

    /// c 段加速（`L(c)=12−1=11 > L(a)=−10`）→ 力度不衰减 → 无背驰。
    #[test]
    fn 末段力度不衰减则无背驰() {
        let t = 两中枢上涨趋势(
            0,
            vec![
                mk(48.0, 49.0, 7, Direction::Up, 0), // v=1
                mk(49.0, 61.0, 8, Direction::Up, 0), // v=12
            ],
        );
        assert!(judge_divergence(&t, PerfectionMode::Structural).is_none());
    }

    /// c 段不创新高（60 未超过此前 60 之外…此处 c 极值 50 == 此前极值 50）→ 几何门 G 拒绝。
    #[test]
    fn 末段不创新高则无背驰() {
        let t = 两中枢上涨趋势(
            0,
            vec![
                mk(48.0, 49.0, 7, Direction::Up, 0), // v=1
                mk(49.0, 50.0, 8, Direction::Up, 0), // v=1（高点 50 == 此前极值 50）
            ],
        );
        assert!(judge_divergence(&t, PerfectionMode::Structural).is_none());
    }

    /// 真背驰（level≥1）：结构滤网 F（条件2/3/5，用 `inner_zhongshu_count`）+ 力度 S=ForceL。
    ///
    /// ★判别性：a 段嵌套和 = 3、c 段嵌套和 = 4——若力度仍用旧 `leg_strength`（嵌套深度），
    /// `4 < 3` 为假 → 拒绝；换 ForceL 后 `L(c)=−11 < L(a)=−10` → 背驰成立。锁定「力度 = 速度
    /// 净增量，不是嵌套深度」（#1243）。
    #[test]
    fn 高级别背驰_结构滤网条件235加force_l() {
        let t = 两中枢上涨趋势(
            1,
            vec![
                mk(48.0, 60.0, 7, Direction::Up, 2), // v=12，nest 2
                mk(59.0, 60.0, 8, Direction::Up, 2), // v=1，nest 2（low 59 > ZG 50 守沿）
            ],
        );
        // 条件2：c 段 u8.low=59 > ZG=50 ✓；条件5：c_nest=2+2=4≥2 ✓；条件3：b_nest=1≤4 ✓。
        let bsp = judge_divergence(&t, PerfectionMode::Structural).expect("高级别 ForceL 背驰");
        assert_eq!(bsp.kind, BSPKind::Type1Sell);
        assert_eq!(bsp.price, 60.0);
        assert_eq!(bsp.level, 2);
    }

    /// 真背驰条件5不足：c 段次级别中枢数 <2（c 非次级别趋势）→ 拒绝（滤掉 79% 伪背驰）。
    #[test]
    fn 真背驰_条件5_c段中枢不足2_拒绝() {
        let t = 两中枢上涨趋势(
            2,
            vec![
                mk(59.0, 60.0, 7, Direction::Up, 1), // c_nest=1 <2 → 条件5 不满足（low 59>50 守沿已过条件2）
            ],
        );
        assert!(
            judge_divergence(&t, PerfectionMode::Structural).is_none(),
            "c 段非次级别趋势（中枢<2）应拒绝"
        );
    }

    /// 真背驰条件2不满足：c 段所有单元 low 均 < B.ZG=50（回抽破中枢上沿）→ 拒绝。
    #[test]
    fn 真背驰_条件2_回抽破中枢上沿_拒绝() {
        let t = 两中枢上涨趋势(
            2,
            vec![
                mk(47.0, 60.0, 7, Direction::Up, 2), // low 47 < ZG 50
                mk(49.0, 60.0, 8, Direction::Up, 2), // low 49 < ZG 50
            ],
        );
        assert!(
            judge_divergence(&t, PerfectionMode::Structural).is_none(),
            "c 段回抽破 ZG（无类三买）应拒绝"
        );
    }

    /// ≥3 中枢趋势：A/B 与 b 段必须锚定**最后两个中枢**（A=倒数第二、B=末），不得吞中间
    /// 中枢（对抗核验 major 回归）。中间中枢 z2 嵌套深（3×3=9）：旧首/末读法 b 段=z2 →
    /// b_nest=9 > c_nest=4 → 系统性过严错杀；修复后 b 段=z2↔z3 连接段（空）→ b_nest=0 ≤ 4。
    /// 力度分量用 ForceL：`L(c)=v(u10)−v(u9)=1−12=−11 < L(a)=v(u5)−v(u0)=11−14=−3`。
    #[test]
    fn 三中枢趋势_b段锚最后两中枢_不吞中间中枢() {
        let units = vec![
            mk(8.0, 22.0, 0, Direction::Up, 2),    // z1 v=14
            mk(12.0, 18.0, 1, Direction::Down, 2), // v=6
            mk(10.0, 20.0, 2, Direction::Up, 2),   // v=10
            mk(28.0, 42.0, 3, Direction::Up, 3),   // z2（中间中枢，嵌套深 3×3=9）v=14
            mk(32.0, 40.0, 4, Direction::Down, 3), // v=8
            mk(30.0, 41.0, 5, Direction::Up, 3),   // v=11
            mk(48.0, 62.0, 6, Direction::Up, 2),   // z3（末中枢 B）v=14
            mk(52.0, 60.0, 7, Direction::Down, 2), // v=8
            mk(50.0, 61.0, 8, Direction::Up, 2),   // v=11
            mk(63.0, 75.0, 9, Direction::Up, 2),   // c 段：离开 z3 向上，low=63 > ZG=60（守沿）v=12
            mk(70.0, 71.0, 10, Direction::Up, 2),  // c 末笔 v=1
        ];
        let z1 = Zhongshu {
            high: 18.0,
            low: 12.0,
            gg: 22.0,
            dd: 8.0,
            units: vec![0, 1, 2],
            level: 2,
        };
        let z2 = Zhongshu {
            high: 40.0,
            low: 32.0,
            gg: 42.0,
            dd: 28.0,
            units: vec![3, 4, 5],
            level: 2,
        };
        let z3 = Zhongshu {
            high: 60.0,
            low: 52.0,
            gg: 62.0,
            dd: 48.0,
            units: vec![6, 7, 8],
            level: 2,
        };
        let t = TrendType {
            kind: TrendKind::UpTrend,
            zhongshus: vec![z1, z2, z3],
            units,
            level: 2,
            direction: Direction::Up,
            completed: false,
            bsp: None,
        };
        // a 段=units[0..=5]；b 段=z2↔z3 间(空) b_nest=0≤c_nest=4；c 创新高 75、守 ZG=60、
        // ForceL −11 < −3 → 5 条件全过。
        let bsp = judge_divergence(&t, PerfectionMode::Structural)
            .expect("3 中枢趋势 b 段锚最后两中枢应产一卖");
        assert_eq!(bsp.kind, BSPKind::Type1Sell);
        assert_eq!(bsp.price, 75.0);
        assert_eq!(bsp.bar, 9);
    }

    /// 盘整背驰（单中枢）：进入段 vs 离开段 ForceL 力度衰减 + 离开创新高 → 产一卖。
    #[test]
    fn 盘整背驰_force_l力度衰减创新高_产点() {
        let units = vec![
            mk(10.0, 30.0, 0, Direction::Up, 0),   // 进入段 v=20
            mk(12.0, 22.0, 1, Direction::Down, 0), // v=10
            mk(11.0, 21.0, 2, Direction::Up, 0),   // v=10
            mk(21.0, 33.0, 3, Direction::Up, 0),   // 离开段 v=12（创新高 33）
            mk(32.0, 33.0, 4, Direction::Up, 0),   // 离开段末 v=1
        ];
        let center = Zhongshu {
            high: 22.0,
            low: 12.0,
            gg: 30.0,
            dd: 10.0,
            units: vec![0, 1, 2],
            level: 0,
        };
        let t = TrendType {
            kind: TrendKind::Consolidation,
            zhongshus: vec![center],
            units,
            level: 0,
            direction: Direction::Up,
            completed: false,
            bsp: None,
        };
        // L(进入)=10−20=−10；L(离开)=1−12=−11 < −10 → 背驰。
        let bsp = judge_divergence(&t, PerfectionMode::Structural).expect("盘整背驰应产一卖");
        assert_eq!(bsp.kind, BSPKind::Type1Sell);
        assert_eq!(bsp.price, 33.0);
        assert_eq!(bsp.level, 0);
    }

    /// ★#1243：M 撤出判据 ⟹ `Structural`/`And`/`Or` 三变体同口径（同输入同输出）。
    #[test]
    fn 模式三变体同口径_force_l背驰产点一致() {
        let t = 两中枢上涨趋势(
            0,
            vec![
                mk(48.0, 60.0, 7, Direction::Up, 0),
                mk(59.0, 60.0, 8, Direction::Up, 0),
            ],
        );
        let s = judge_divergence(&t, PerfectionMode::Structural).expect("背驰");
        let a = judge_divergence(&t, PerfectionMode::And).expect("And 同口径");
        let o = judge_divergence(&t, PerfectionMode::Or).expect("Or 同口径");
        assert_eq!(s, a);
        assert_eq!(s, o);
    }

    /// ★#1243：非背驰输入下三变体一致拒绝（无 MACD 兜底放行）。
    #[test]
    fn 模式三变体同口径_非背驰一致拒绝() {
        let t = 两中枢上涨趋势(
            0,
            vec![
                mk(48.0, 49.0, 7, Direction::Up, 0), // L(c)=12−1=11 > L(a)=−10
                mk(49.0, 61.0, 8, Direction::Up, 0),
            ],
        );
        assert!(judge_divergence(&t, PerfectionMode::Structural).is_none());
        assert!(judge_divergence(&t, PerfectionMode::And).is_none());
        assert!(judge_divergence(&t, PerfectionMode::Or).is_none());
    }

    /// ★对拍锁（#804 测试锁形状 + #1266 AC）：递归线 `force_l`（#873 经 #989 反查）与
    /// 生产 `theta_v0::parser::segment::segment_force_l` 同输入同输出——同一「段」的笔切片
    /// 映射为 theta_v0 Stroke 后，两实现 L(段) 逐位一致。
    #[test]
    fn force_l_与theta_v0_segment_force_l_同输入同输出() {
        use crate::theta_v0::parser::segment::segment_force_l;
        use crate::theta_v0::types::{Direction as V0Dir, Stroke};

        fn to_stroke(u: &Unit) -> Stroke {
            let (sp, ep) = match u.direction {
                Direction::Up => (u.low as i64, u.high as i64),
                Direction::Down => (u.high as i64, u.low as i64),
            };
            Stroke {
                direction: match u.direction {
                    Direction::Up => V0Dir::Up,
                    Direction::Down => V0Dir::Down,
                },
                start_index: u.start_bar as usize,
                end_index: u.end_bar as usize,
                start_price: sp,
                end_price: ep,
            }
        }

        // 上涨腿：v 依次 20 / 10 / 10 ⟹ L = 10 − 20 = −10。
        let up_leg = vec![
            mk(10.0, 30.0, 0, Direction::Up, 0),
            mk(12.0, 22.0, 1, Direction::Down, 0),
            mk(11.0, 21.0, 2, Direction::Up, 0),
        ];
        // 下跌腿：v 依次 10 / 5 / 5 ⟹ L = 5 − 10 = −5。
        let down_leg = vec![
            mk(40.0, 50.0, 3, Direction::Down, 0),
            mk(43.0, 48.0, 4, Direction::Up, 0),
            mk(44.0, 49.0, 5, Direction::Down, 0),
        ];
        for leg in [&up_leg, &down_leg] {
            let strokes: Vec<Stroke> = leg.iter().map(to_stroke).collect();
            let seg_start = leg.first().unwrap().start_bar as usize;
            let seg_end = leg.last().unwrap().end_bar as usize;
            let expected = force_l(leg);
            let actual =
                segment_force_l(&strokes, seg_start, seg_end).expect("段区间应覆盖至少一笔");
            assert!(
                (actual - expected).abs() < 1e-9,
                "force_l={expected} ≠ segment_force_l={actual}"
            );
        }
    }
}
