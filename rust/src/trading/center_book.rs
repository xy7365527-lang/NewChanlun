//! CenterBook — 中枢生命周期账本（市场性质，跨 trade 持续）。
//!
//! `ingest` 逐字移植 Python `run_organic` 主循环的中枢账本段（last_center /
//! dead_centers / frozen / center_version，含 not-in 守卫的版本号语义）。
//!
//! v2 增量（C6/D2）：ingest 同时在**唯一 diff 点**派生 `CenterEvent` 三态流——
//! 多消费者各自 diff 是分歧温床（C6 候选 D 的否定论证）。数据源边界声明见
//! `types::CenterEvent` docstring（BSP 事件锚派生，非信号层 zhongshus diff）。
//!
//! **收窄（票 #637 尾部评审；方向语义已由票 #664 闭合）**：上述"唯一 diff 点"指 `ingest`；
//! [`CenterBook::consume_death_certificate`] 是第二条会改 `dead` 但**不经**该 diff 点的路径——
//! 它现在按证明自带的 `side`（票 #664）独立补发 `CenterEvent::Terminated`（首次登记 Broken 时
//! 一次，`AlreadyBroken` 幂等确认不重发，避免同一教义事件被两条通道各报一次）。`CenterEvent`
//! 流因此仍非单点 diff（两条通道各自派生各自的事件，非共享一个 diff 点），但方向不再沉默——
//! 细节见 [`CenterBook::consume_death_certificate`] doc 与 #664。
//!
//! # 两个死亡登记入口（票 #637 修复轮，2026-07-29 编排者裁定 1A/2A/4A）
//!
//! `ingest` 的 confirmed Type3 自诊断 kill 分支与 [`CenterBook::consume_death_certificate`]
//! 是**同一教义事件（18 课定理三充要条件）的两条观测通道**，不是两个对等的独立死亡入口：
//! 前者是本账自己从 BSP 事件锚 diff 出的判断，后者消费 `retrace_ledger` 独立判案给出的
//! 外部证明。两条通道撞车（同一锚先后被两边杀）是**幂等确认**，不是分歧——判定次序与
//! `KilledByOtherCause`（已删除该错误变体）见 [`CenterBook::consume_death_certificate`] doc。
//!
//! **方向盲区已闭合（票 #664；修复轮改口——先杀落位、后不覆写）**：`CenterDeathCertificate`
//! 现带 `side` 字段（三买/三卖），`consume_death_certificate` **仅在本次是真正首次死亡登记时**
//! 据此登记 `dead_down`（Sell）/ `frozen`（Buy，`hard_type3` 门控，与 `ingest` 同判据）/ 补发
//! `CenterEvent::Terminated`（direction 同 `ingest` 的 Buy→Up/Sell→Down 映射）——`is_dead_down`
//! 的生产消费方（`level_operating_unit.rs` `unified_osc.rs` `axiom_voice.rs`）对 cert 杀不再
//! 恒读 false。若锚已被另一通道先杀（`AlreadyBroken`），本次（cert）消费**不覆写**已落位的
//! 方向——但这条纪律**单侧成立**（影子评审 MEDIUM-1，如实登记非绝对断言）：`dead_down` 与
//! `CenterEvent::Terminated` 补发在两条通道间确实"先杀落位、后不覆写"（`ingest` 侧 `dead_down`
//! 写在 `!dead.contains(&cs)` 守卫内、cert 侧收窄进 `!already_dead`）；`frozen` 有一条 Python
//! parity 携带的例外——`ingest` 的 `frozen` 置位（`hard_type3 && side==Buy`）在该守卫**外**
//! （`#664` 之前即有的既有形状，非本票引入），故 cert 先以 `Sell` 杀锚（`frozen` 保持 `None`）
//! 后、`ingest` 收到同锚 confirmed hard `Buy3`，`frozen` 仍会被无条件翻成 `Some`（不推
//! `version`、不补发第二次 `Terminated`）——`frozen` 上"先杀为准"不成立，实际以 `ingest` 的
//! parity 行为为准。跨通道方向冲突的一般裁定仍是"先杀者为准，不设 fail-loud"，`frozen` 是
//! 其中的已知单侧例外，检测/收口归后续票。见 [`CenterBook::consume_death_certificate`] doc
//! 与 #664。

use std::collections::{HashMap, HashSet};

use super::types::*;
use crate::buysellpoint::{BspKind, Side};
use crate::stroke::Direction;
use crate::theta_v0::classifier::retrace_ledger::{CenterDeathCertificate, CenterFrame, RetraceSide};

/// H2 力度收敛门的三态判据读数（49课行38；`up_strength_verdict`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpStrengthVerdict {
    /// 向上离开段力度历史 <2 条——无"震荡依旧"证据，保守拒开。
    Newborn,
    /// 最近一次力度 > 前一次——扩张，三类点预警，拒开。
    Expanding,
    /// 最近 ≤ 前次——"中枢震荡逐步收敛"成立，放行。
    Converged,
}

/// 存活中枢快照（账本视角的"最后已知中枢"）。
#[derive(Debug, Clone, Copy)]
pub struct LiveCenter {
    pub seg_start: i64,
    pub zd: f64,
    pub zg: f64,
}

#[derive(Debug, Default)]
pub struct CenterBook {
    /// ladder → 最后已知中枢（Python `last_center: dict[int, tuple]`）。
    last: [Option<LiveCenter>; MAX_LADDER],
    /// ladder → 死亡中枢 seg_start 集（Python `dead_centers`）。
    dead: [Option<HashSet<i64>>; MAX_LADDER],
    /// ladder → 向下终结（confirmed Sell3 = 三卖）死亡中枢 seg_start 子集
    /// （49课严格形式 osc_sell3_no_recover 的方向判据；首杀方向）。
    dead_down: [Option<HashSet<i64>>; MAX_LADDER],
    /// ladder → 见过的中枢锚集合（票 #637 修复轮：`ingest` 凡见到带 `cs` 的事件即记录，
    /// kill 分支与 last 更新分支都记）。用途 = 区分"从未见过该锚"（[`DeathCertificateError::NoMatchingCenter`]）
    /// 与"见过但已被更替 / 已死"（迟到证明应照登记——补充十一「Superseded 非死，原框三类点
    /// 判据死后继续适用」、#583「迟到但都到」）。
    known: [Option<HashSet<i64>>; MAX_LADDER],
    /// ladder → 经 [`CenterBook::consume_death_certificate`] 登记 Broken 的中枢锚 → 该证明
    /// 存档的 [`CenterFrame`]（票 #637 修复轮：幂等键从「锚」收紧为「锚 + 框」——同锚不同框
    /// 是上游引擎改口，fail-loud 见 [`DeathCertificateError::ConflictingCertificate`]，不静默
    /// 吞掉）。存档框与 `dead` 的关系：本证明杀的必进 `dead`，但 `dead` 还含 `ingest` 的
    /// confirmed Type3 杀路径（两条通道观测同一教义事件，见模块头）。
    broken_by_certificate: [Option<HashMap<i64, CenterFrame>>; MAX_LADDER],
    /// ladder → 冻结中枢 seg_start（Python `frozen`；hard_type3 buy 置位）。
    frozen: [Option<i64>; MAX_LADDER],
    /// ladder → 未决离开段 (锚中枢 seg_start, 离开方向 side)——49课禁令窗口
    /// （H1 candidate 冻结，2026-06-12 任务）。
    ///
    /// 49课行52："中枢完成后的向上移动时的差价是不能做的"；行68 给出当下
    /// 判据——次级别走势**离开**中枢（candidate 三类买卖点出现）即启动
    /// "向上移动"语义，不需要等回抽确认（confirmed type3）。candidate
    /// type3 事件置位本窗口；窗口由价格回中枢否定（`negate_pending_departure`，
    /// 49课行52 前提"前提是中枢震荡依旧"的对称否定——回试跌回边界内 =
    /// 仍是中枢震荡）、中枢死亡（confirmed type3，现行 frozen/dead 语义
    /// 接管）或新中枢形成（cs 变化）解除。每个新离开段（新 seg_idx 的
    /// candidate 事件）重新置位——逐段窗口语义。
    pending_departure: [Option<(i64, Side)>; MAX_LADDER],
    /// H2（osc_strength_gate，49课行38）：未决**向上**离开段窗口内的运行
    /// max excursion（max(c) − 当时 ZG——力度 = 价格振幅在册口径对离开段
    /// 的直读，零 surfacing）。仅 Side::Buy 窗口有意义；窗口置位时重置为
    /// NEG_INFINITY。中枢延伸时 ZG 取当下值（已推入的历史记录不回溯修订）。
    pending_excursion: [f64; MAX_LADDER],
    /// H2：per-center 向上离开段力度历史 (锚中枢 seg_start, [前次, 最近], 条数)。
    /// 环形容量 2——49课行38"后面的向下离开力度一定比前一个小"是相邻两次
    /// 比较的字面（扩窗即引入参数，须回原文重审——设计预注册边界 (b)）。
    /// **仅价格否定的窗口推入**（回试跌回边界内 = "如果继续是中枢震荡"的
    /// 完成样本）；中枢死亡/新中枢解除的窗口不推入——该离开段终结了中枢，
    /// 不属于"继续是中枢震荡"的序列（行38 判据的参照系内生于震荡序列本身）。
    up_strength: [Option<(i64, [f64; 2], u8)>; MAX_LADDER],
    /// 禁令窗口置位次数（G3 可观测性：candidate 离开段事件数）。
    pub cf_windows: u64,
    /// 禁令窗口价格否定次数（回试跌回边界内解冻数；与 cf_windows 之差 =
    /// 由死亡/新中枢解除或持续到结束的窗口数）。
    pub cf_negations: u64,
    /// H2 力度历史推入总数（= Buy 侧价格否定数——每次推入即一条完成的
    /// 向上离开段样本；与 cf_negations 之差 = Sell 侧否定数）。
    pub sg_records: u64,
    /// H2 力度历史达到可比对（len 1→2 跃迁）的中枢数——判据参照系
    /// 非空性的直接读数：=0 ⇒ "同中枢两次完成向上离开段"在该磁带上
    /// 是空集，收敛/扩张分支结构性不可达。
    pub sg_pairs: u64,
    /// 中枢生死/边界事件版本号（SizeAllocator 重算门控）。
    pub version: u64,
}

impl CenterBook {
    pub fn new() -> Self {
        Self::default()
    }

    /// 消费一层的本 bar BSP 事件流。`events_out` 非 None 时收集本层 CenterEvent
    /// （v2 消费者：T4b 加码 / FatigueGate 清空路径(2) / 域腿解冻观测）。
    ///
    /// Python 逐字对应（含分支顺序与版本号自增点）：
    /// ```python
    /// kind, side, confirmed, cs = ev[0], ev[1], ev[3], ev[4]
    /// if cs is None: continue
    /// dead = dead_centers.setdefault(lad, set())
    /// if confirmed and kind == "type3":
    ///     if cs not in dead: dead.add(cs); center_version += 1
    ///     if hard_type3 and side == "buy": frozen[lad] = cs
    /// elif cs not in dead:
    ///     if last_center.get(lad) != (cs, ev[5], ev[6]): center_version += 1
    ///     last_center[lad] = (cs, ev[5], ev[6])
    ///     if lad in frozen and frozen[lad] != cs: del frozen[lad]
    /// ```
    pub fn ingest(
        &mut self,
        ladder: usize,
        evs: &[BspEvent],
        hard_type3: bool,
        mut events_out: Option<&mut Vec<CenterEvent>>,
    ) {
        for ev in evs {
            let Some(cs) = ev.cs else { continue };
            // 票 #637 修复轮：凡带 cs 的事件即记入"见过"锚集，不分 kill/last 分支——
            // 供 consume_death_certificate 区分"从未见过"与"见过但已被更替/已死"。
            self.known[ladder].get_or_insert_with(HashSet::new).insert(cs);
            let dead = self.dead[ladder].get_or_insert_with(HashSet::new);
            if ev.confirmed && ev.class.kind() == BspKind::Type3 {
                if !dead.contains(&cs) {
                    dead.insert(cs);
                    if ev.class.side() == Side::Sell {
                        // 三卖终结（向下离开）——49课"不能回补"的方向判据
                        self.dead_down[ladder]
                            .get_or_insert_with(HashSet::new)
                            .insert(cs);
                    }
                    self.version += 1;
                    if let Some(out) = events_out.as_deref_mut() {
                        // Buy3 = 向上离开后回抽不破 ZG → 中枢向上终结；Sell3 反之。
                        let direction = match ev.class.side() {
                            Side::Buy => Direction::Up,
                            Side::Sell => Direction::Down,
                        };
                        out.push(CenterEvent::Terminated { seg_start: cs, direction });
                    }
                }
                if hard_type3 && ev.class.side() == Side::Buy {
                    self.frozen[ladder] = Some(cs);
                }
                // H1：中枢死亡 ⇒ 该中枢的未决离开段窗口解除（confirmed
                // type3 的 dead/frozen 语义接管，窗口对象已不存在）。
                if self.pending_departure[ladder].is_some_and(|(p, _)| p == cs) {
                    self.pending_departure[ladder] = None;
                }
            } else if !dead.contains(&cs) {
                // Python 元组比较 (cs, zd, zg)；zd/zg 为 Option<f64>，
                // Python None==None 与 float== 语义由 Option<f64> 等值精确对应。
                let prev = self.last[ladder];
                let changed = match prev {
                    None => true,
                    Some(lc) => {
                        lc.seg_start != cs
                            || Some(lc.zd) != ev.zd
                            || Some(lc.zg) != ev.zg
                    }
                };
                if changed {
                    self.version += 1;
                    if let Some(out) = events_out.as_deref_mut() {
                        // cs 变化 = Formed（新中枢）；同 cs 边界更新 = Extended。
                        match prev {
                            Some(lc) if lc.seg_start == cs => {
                                out.push(CenterEvent::Extended { seg_start: cs });
                            }
                            _ => out.push(CenterEvent::Formed {
                                seg_start: cs,
                                zd: ev.zd.unwrap_or(f64::NAN),
                                zg: ev.zg.unwrap_or(f64::NAN),
                            }),
                        }
                    }
                }
                // Python last_center 存事件原始 zd/zg（可为 None——但 osc 开腿读
                // lc.zd/zg 做算术，None 在 Python 会 TypeError ⇒ 生产磁带上恒非
                // None。fail-fast 同构：None 时存 NaN，算术比较恒 False 显式化。
                self.last[ladder] = Some(LiveCenter {
                    seg_start: cs,
                    zd: ev.zd.unwrap_or(f64::NAN),
                    zg: ev.zg.unwrap_or(f64::NAN),
                });
                if let Some(f) = self.frozen[ladder] {
                    if f != cs {
                        self.frozen[ladder] = None;
                    }
                }
                // H1（49课行68）：candidate type3 = 次级别走势离开中枢——
                // 禁令窗口置位。confirmed type3 不走本分支（上方 kill 分支），
                // 故此处 kind==Type3 必为 candidate。每个新离开段（事件流
                // 按 (kind,side,seg_idx,confirmed) 去重，新 seg_idx 重发）
                // 重新置位窗口。
                if ev.class.kind() == BspKind::Type3 {
                    self.pending_departure[ladder] = Some((cs, ev.class.side()));
                    // H2：新窗口 excursion 归零位（运行 max 从无穷小起，
                    // 同 bar 的 negate 驱动即折入当 bar close）。
                    self.pending_excursion[ladder] = f64::NEG_INFINITY;
                    self.cf_windows += 1;
                }
                // H1：新中枢形成（cs 变化）⇒ 旧中枢的未决离开段窗口解除
                // （frozen 解除同构——窗口挂在锚中枢上，锚已被覆盖）。
                else if self.pending_departure[ladder].is_some_and(|(p, _)| p != cs) {
                    self.pending_departure[ladder] = None;
                }
            }
        }
    }

    /// H1 禁令窗口的价格否定（每 bar 驱动，市场性质——与持仓/配置无关）。
    ///
    /// 49课行52 的前提是"中枢震荡依旧"：向上离开段（candidate Buy3）在
    /// 价格回到 ZG 之下时被否定（回试跌回中枢 = 仍是中枢震荡，三买不成立
    /// ——38课答疑"能回到中枢就不是第三类买点"）；向下离开段（candidate
    /// Sell3）对称地在价格回到 ZD 之上时被否定。边界取 last 中枢的当前
    /// 边界（中枢延伸时随之更新）。NaN 边界比较恒 false ⇒ 不否定（保守
    /// 方向，与 osc 开腿 NaN 语义同构——生产磁带 zd/zg 恒非 None）。
    pub fn negate_pending_departure(&mut self, ladder: usize, c: f64) {
        let Some((cs, side)) = self.pending_departure[ladder] else { return };
        let Some(lc) = self.last[ladder] else { return };
        debug_assert_eq!(
            lc.seg_start, cs,
            "pending_departure 与 last 中枢不一致——ingest 的清除路径有缺口"
        );
        // H2：向上离开段力度观测（excursion = max(c) − 当时 ZG）。NaN ZG
        // 比较恒 false ⇒ 不更新（同窗口否定的 NaN 语义——生产磁带恒非 None）。
        if side == Side::Buy {
            let exc = c - lc.zg;
            if exc > self.pending_excursion[ladder] {
                self.pending_excursion[ladder] = exc;
            }
        }
        let negated = match side {
            Side::Buy => c < lc.zg,
            Side::Sell => c > lc.zd,
        };
        if negated {
            // H2：价格否定 = 回试跌回边界内 = "继续是中枢震荡"——本次向上
            // 离开段完成，力度推入 per-center 历史。Buy 侧否定（c < ZG）必经
            // 上方 excursion 更新 ⇒ 推入值恒有限（最差为本 bar 的 c − ZG < 0，
            // 设计预注册边界 (a) 的"单 bar 越界"退化形态，按原值记录）。
            if side == Side::Buy {
                self.push_up_strength(ladder, cs, self.pending_excursion[ladder]);
            }
            self.pending_departure[ladder] = None;
            self.cf_negations += 1;
        }
    }

    /// H2：向上离开段力度推入 per-center 环形历史（容量 2）。锚中枢变更
    /// （cs 不匹配）即重开历史——历史与中枢同生命周期，无跨中枢继承。
    fn push_up_strength(&mut self, ladder: usize, cs: i64, strength: f64) {
        self.sg_records += 1;
        match &mut self.up_strength[ladder] {
            Some((s, hist, len)) if *s == cs => {
                if *len < 2 {
                    hist[*len as usize] = strength;
                    *len += 1;
                    if *len == 2 {
                        self.sg_pairs += 1;
                    }
                } else {
                    hist[0] = hist[1];
                    hist[1] = strength;
                }
            }
            slot => *slot = Some((cs, [strength, 0.0], 1)),
        }
    }

    /// H2 力度收敛门判据（osc_strength_gate，开腿时刻只读——不等任何未来
    /// 事件，49课行52"用中枢震荡力度判断的方法，完全可以避开"的当下形式）：
    /// - 该锚中枢向上力度历史 <2 条 ⇒ Newborn（新生保守默认：行38"逐步
    ///   收敛"语义下无"震荡依旧"证据）；
    /// - 最近一次 > 前一次 ⇒ Expanding（行38 扩张 ⇒ 三类点预警）；
    /// - 最近 ≤ 前次 ⇒ Converged（放行）。
    pub fn up_strength_verdict(&self, ladder: usize, cs: i64) -> UpStrengthVerdict {
        match self.up_strength[ladder] {
            Some((s, hist, len)) if s == cs && len >= 2 => {
                if hist[1] > hist[0] {
                    UpStrengthVerdict::Expanding
                } else {
                    UpStrengthVerdict::Converged
                }
            }
            _ => UpStrengthVerdict::Newborn,
        }
    }

    /// H1 禁令窗口查询：该层存在未被否定的 candidate 离开段（49课行52
    /// "中枢完成后的向上移动时的差价是不能做的"——窗口内 osc 不开腿）。
    pub fn has_pending_departure(&self, ladder: usize) -> bool {
        self.pending_departure[ladder].is_some()
    }

    /// P6 相位机 candidate 先行读法（049:68 离开即启动移动语义；research
    /// §5.1"candidate 先行、confirmed 校正"——价格否定即校正）：未决离开段
    /// 方向。Buy = candidate MOVE↑ 窗口，Sell = candidate MOVE↓ 窗口。
    pub fn pending_departure_side(&self, ladder: usize) -> Option<Side> {
        self.pending_departure[ladder].map(|(_, s)| s)
    }

    /// 该层最后已知中枢（Python `last_center.get(k)`——注意：**不查 dead**，
    /// 存活判定由调用方组合 `is_dead`，与 Python 调用面逐字一致）。
    pub fn last(&self, ladder: usize) -> Option<LiveCenter> {
        self.last[ladder]
    }

    /// 该层当前存活中枢（last 存在 ∧ 不在 dead）。
    pub fn alive(&self, ladder: usize) -> Option<LiveCenter> {
        let lc = self.last[ladder]?;
        if self.is_dead(ladder, lc.seg_start) {
            None
        } else {
            Some(lc)
        }
    }

    pub fn is_dead(&self, ladder: usize, seg_start: i64) -> bool {
        self.dead[ladder].as_ref().is_some_and(|d| d.contains(&seg_start))
    }

    /// 该中枢是否被三卖（confirmed Sell3，向下离开）终结。49课严格形式的
    /// 方向判据："一旦出现第三类卖点，就不能回补了"只覆盖向下终结——
    /// 三买（向上终结）按"中枢向上移动时就应该满仓"必须立即回补。
    pub fn is_dead_down(&self, ladder: usize, seg_start: i64) -> bool {
        self.dead_down[ladder].as_ref().is_some_and(|d| d.contains(&seg_start))
    }

    pub fn is_frozen(&self, ladder: usize) -> bool {
        self.frozen[ladder].is_some()
    }

    /// 中枢死亡证明消费入口（票 #637 修复轮，2026-07-29 编排者裁定 1A/2A/4A；#575 消费方
    /// 接线 1/3；中枢死亡证明权唯一归三类点——18 课定理三充要条件 / ADR-0001）。
    ///
    /// **判同 = 锚判同，禁跨量纲数值比较（裁定 2A）**：`anchor = cert.center.start_index as i64`
    /// 对本层 `known`（名义映射——`start_index` 与 `seg_start` 是两套引擎各自切分的段
    /// 序列，仓内无对齐依据，只取"同角色字段直等"，如实声明非数值同源保证）。**不比较**
    /// `zd`/`zg` 与 `LiveCenter` 的价格边——一个是 theta_v0 `quantize` 后的 `Tick`（刻度计数），
    /// 一个是 legacy Python-parity 引擎透传的原始价格浮点，量纲不同，比较恒不命中（影子
    /// HIGH-3）。证明的 [`CenterFrame`] 全四条边只与**另一张证明**比较（同量纲 `Tick`），
    /// 见下方判定次序 ①。
    ///
    /// **判定次序**：
    /// 1. `broken_by_certificate` 命中 `anchor`：存档框全四边等于本证明 ⟹ `Ok(AlreadyBroken)`
    ///    零动作；不等 ⟹ `Err(ConflictingCertificate)`——同锚不同框 = 上游引擎改口，与
    ///    `retrace_ledger` 的 `NotConstitutedReason::CenterRebased` 同族，fail-loud。
    /// 2. `known` 不含 `anchor` ⟹ `Err(NoMatchingCenter)`——**从未见过**该锚（票面「无此
    ///    中枢」的准确含义；不含"被新中枢取代"，见下）。
    /// 3. 否则登记：`dead` 插 `anchor`；`broken_by_certificate` 存档本证明的框；若
    ///    `pending_departure[ladder]` 的锚 == `anchor` 则清除该窗口（对齐 `ingest` 杀路径
    ///    `:188-190` 的收尾——窗口滞留会让 [`Self::negate_pending_departure`] 往 `up_strength`
    ///    推非法样本）。`is_dead(ladder, anchor)` 此前已为真（`ingest` 先杀）⟹
    ///    `Ok(AlreadyBroken)`（`version` 不再前进——死亡早已登记，本证明是同一教义事件的
    ///    第二次观测，幂等确认，非分歧）；否则 ⟹ `Ok(Broken)`，`version += 1`。
    ///
    /// **不问迟到（054:60 / ADR-0001 补充十一 / #583 在案裁定）**：锚被新中枢取代（`last`
    /// 已前移）不等于"无此中枢"——补充十一「Superseded 不是中枢的死，原框三类点判据死后
    /// 继续适用」；`known` 记录锚曾经存在，故被更替的锚仍能判同（步骤 2），迟到证明照登记
    /// `Broken`。`issued_as_of` 本身不参与判定——`CenterBook` 不追踪可比对的时钟坐标，
    /// "迟到"由"照登记"这一行为本身表达，不是另设时钟分支。
    ///
    /// **判同映射的结构性缺口（如实登记，非遗漏）**：证明方 [`CenterFrame`] 有四条边，与
    /// `LiveCenter` 判同时只用 `start_index` 一条（裁定 2A 收窄）；`zd`/`zg`/`end_index` 三边
    /// 不参与对 `LiveCenter` 的判同，只在"同证明 vs 同证明"（步骤 1）时全四边比较。
    ///
    /// **方向登记（票 #664，闭合上述盲区；#664 修复轮改口——先杀落位、后不覆写）**：
    /// `cert.side` 携带杀路径方向（三买/三卖），**只在本次是真正首次死亡登记（下方 `Broken`
    /// 分支，即 `!already_dead`）时**据此落位：
    /// - `Sell`（向下离开终结）⟹ `anchor` 插 `dead_down`——`is_dead_down` 的生产消费方
    ///   （`level_operating_unit.rs` / `unified_osc.rs` / `axiom_voice.rs`）自此对 cert 杀
    ///   读到正确方向，"三卖不能回补"（49 课）不再被读反成三买回补；
    /// - `Buy` 且入参 `hard_type3` 为真 ⟹ `frozen[ladder] = Some(anchor)`——判据字面等同
    ///   `ingest` 的 `hard_type3 && side==Buy`（`:183-184`），只是入参来源从"`ingest` 调用时的
    ///   运行时旗标"换成"本方法调用方显式传入同一旗标"（cert 路径无 `ingest` 那样的天然
    ///   config 上下文，改为参数化传入，语义零改写）；
    /// - `CenterEvent::Terminated { seg_start: anchor, direction }` 补发（`events_out` 非
    ///   `None` 时），`direction` 映射同 `ingest`（Buy→Up / Sell→Down）。
    ///
    /// `AlreadyBroken`（步骤 1 早退的同框重复消费，或步骤 3 落到 `already_dead` 为真——
    /// `ingest` 已先行以 BSP 事件锚杀该锚）**本方法（consume_death_certificate 自身）一律
    /// 不动 `dead_down`/`frozen`、不补发事件**：本方法只在真 `Broken` 分支写方向，
    /// `AlreadyBroken` 分支零动作——这半条纪律无条件成立，若仍照 `cert.side` 写一遍会不推
    /// `version` 使版本门控方感知不到方向读数的静默变化，本票已堵死。
    ///
    /// **但"先杀通道方向从不缺失、故不覆写"这条纪律对 `dead_down` 成立、对 `frozen` 不成立
    /// （影子评审 MEDIUM-1，如实登记，非本票引入的问题）**：`ingest` 的 confirmed Type3 kill
    /// 分支里，`dead_down` 写在 `!dead.contains(&cs)` 守卫内（`:165-172`），对已死锚不会重复
    /// 触碰；但 `frozen` 置位（`hard_type3 && side==Buy`，`:183-184`）在该守卫**外**——这是
    /// Python parity 逐字对应的既有形状（`#664` 之前即如此，非本票引入，parity 禁区不改
    /// 代码），`ingest` 每次处理 confirmed Type3 事件都无条件执行，不问该锚是否已被 cert
    /// 先杀。故时序「cert 先以 `Sell` 杀锚（`frozen` 保持 `None`）→ `ingest` 后到同锚
    /// confirmed hard `Buy3`」下，`ingest` 的 `dead.contains` 已真、kill 块整体跳过，但
    /// `frozen` 仍会被**无条件翻成** `Some`（不推 `version`、不发第二次 `Terminated`，因为
    /// kill 块被跳过）——`frozen` 上"先杀为准"不成立，实际以 `ingest` 的 parity 行为为准。
    /// 跨通道方向冲突（两通道对同一锚给出不同 `side`）= **先杀者为准，`frozen` 是其中单侧
    /// 例外**，本票不设 fail-loud 检查——检测/收口归后续票（编排方立票中）。
    ///
    /// **`events_out` 传 `None` 的警示（影子评审 LOW-1）**：本方法对同一锚的 `Broken` 分支
    /// 只走一次（`AlreadyBroken` 不重发），若调用方在真 `Broken` 那一次调用传了 `None`，
    /// 该次 `CenterEvent::Terminated` **永久丢失**——不同于 `ingest` 侧（漏一次还能靠后续
    /// `Formed`/`Extended` 事件补），cert 杀路径对同一锚只登记一次死亡，之后再传
    /// `Some(out)` 也补不回来。当前无生产调用点（4 条测试传 `None` 皆为负控），故无实害；
    /// #575 驱动票接线时须传 `Some`。
    pub fn consume_death_certificate(
        &mut self,
        ladder: usize,
        cert: &CenterDeathCertificate,
        hard_type3: bool,
        events_out: Option<&mut Vec<CenterEvent>>,
    ) -> Result<DeathCertificateOutcome, DeathCertificateError> {
        let anchor = cert.center.start_index as i64;
        if let Some(frame) =
            self.broken_by_certificate[ladder].as_ref().and_then(|m| m.get(&anchor))
        {
            return if *frame == cert.center {
                Ok(DeathCertificateOutcome::AlreadyBroken)
            } else {
                Err(DeathCertificateError::ConflictingCertificate { ladder, anchor })
            };
        }
        if !self.known[ladder].as_ref().is_some_and(|k| k.contains(&anchor)) {
            return Err(DeathCertificateError::NoMatchingCenter { ladder, anchor });
        }
        let already_dead = self.is_dead(ladder, anchor);
        self.dead[ladder].get_or_insert_with(HashSet::new).insert(anchor);
        self.broken_by_certificate[ladder]
            .get_or_insert_with(HashMap::new)
            .insert(anchor, cert.center);
        if self.pending_departure[ladder].is_some_and(|(p, _)| p == anchor) {
            self.pending_departure[ladder] = None;
        }
        if already_dead {
            // 先杀者（ingest）已落位方向；本证明是同一教义事件的第二次观测，不覆写。
            Ok(DeathCertificateOutcome::AlreadyBroken)
        } else {
            if cert.side == RetraceSide::Sell {
                self.dead_down[ladder].get_or_insert_with(HashSet::new).insert(anchor);
            }
            if hard_type3 && cert.side == RetraceSide::Buy {
                self.frozen[ladder] = Some(anchor);
            }
            self.version += 1;
            if let Some(out) = events_out {
                let direction = match cert.side {
                    RetraceSide::Buy => Direction::Up,
                    RetraceSide::Sell => Direction::Down,
                };
                out.push(CenterEvent::Terminated { seg_start: anchor, direction });
            }
            Ok(DeathCertificateOutcome::Broken)
        }
    }
}

/// [`CenterBook::consume_death_certificate`] 成功消费的判别（票 #637）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeathCertificateOutcome {
    /// 首次消费：锚判同命中已知中枢，登记 Broken（`version` 前进）。
    Broken,
    /// 幂等确认：该锚死亡登记已存在（本证明重复消费，或同一三类点事件已被 `ingest`
    /// 通道先观测——两条通道观测同一教义事件，见模块头），零动作，`version` 不前进。
    AlreadyBroken,
}

/// [`CenterBook::consume_death_certificate`] 判同失败（票 #637：fail-loud，不静默）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeathCertificateError {
    /// 该 ladder **从未见过**该锚（`known` 不含 `anchor`）——票面「无此中枢」的准确含义。
    /// 不包含"被新中枢取代"——那种情形按裁定 1A 应判同成功（补充十一/#583，见方法 doc）。
    NoMatchingCenter { ladder: usize, anchor: i64 },
    /// 同一锚收到两张边框不同的证明——上游引擎改口（与 `retrace_ledger` 的
    /// `NotConstitutedReason::CenterRebased` 同族），fail-loud，不静默吞掉分歧。
    ConflictingCertificate { ladder: usize, anchor: i64 },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(class: BspClass, confirmed: bool, cs: i64, zd: f64, zg: f64) -> BspEvent {
        BspEvent {
            class,
            seg_idx: 0,
            confirmed,
            cs: Some(cs),
            zd: Some(zd),
            zg: Some(zg),
            price: 0.0,
        }
    }

    #[test]
    fn formed_extended_terminated_lifecycle() {
        let mut book = CenterBook::new();
        let mut out = Vec::new();
        // 新中枢 → Formed + version+1
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.0)], true, Some(&mut out));
        assert_eq!(book.version, 1);
        assert!(matches!(out[0], CenterEvent::Formed { seg_start: 10, .. }));
        assert!(book.alive(2).is_some());
        // 同 cs 边界更新 → Extended
        out.clear();
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.5)], true, Some(&mut out));
        assert!(matches!(out[0], CenterEvent::Extended { seg_start: 10 }));
        // 同 cs 同边界 → 无事件无版本号
        out.clear();
        let v = book.version;
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.5)], true, Some(&mut out));
        assert!(out.is_empty());
        assert_eq!(book.version, v);
        // confirmed Buy3 → Terminated{Up} + 冻结
        out.clear();
        book.ingest(2, &[ev(BspClass::Buy3, true, 10, 1.0, 2.5)], true, Some(&mut out));
        assert!(matches!(
            out[0],
            CenterEvent::Terminated { seg_start: 10, direction: Direction::Up }
        ));
        assert!(book.is_frozen(2));
        assert!(book.alive(2).is_none()); // last 仍在但已死
        // 新中枢出现 → 解冻 + Formed
        out.clear();
        book.ingest(2, &[ev(BspClass::Sell1, true, 20, 3.0, 4.0)], true, Some(&mut out));
        assert!(!book.is_frozen(2));
        assert!(matches!(out[0], CenterEvent::Formed { seg_start: 20, .. }));
    }

    #[test]
    fn dead_center_events_ignored() {
        let mut book = CenterBook::new();
        book.ingest(2, &[ev(BspClass::Buy3, true, 10, 1.0, 2.0)], true, None);
        let v = book.version;
        // 死中枢的后续事件不更新 last（Python elif cs not in dead）
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.0)], true, None);
        assert_eq!(book.version, v);
        assert!(book.last(2).is_none());
    }

    #[test]
    fn candidate_type3_does_not_kill() {
        let mut book = CenterBook::new();
        // candidate type3 走 elif 分支（确认才杀）——Python `confirmed and kind==type3`
        book.ingest(2, &[ev(BspClass::Buy3, false, 10, 1.0, 2.0)], true, None);
        assert!(book.alive(2).is_some());
        assert!(!book.is_frozen(2));
    }

    #[test]
    fn h1_candidate_departure_window_lifecycle() {
        let mut book = CenterBook::new();
        // 中枢形成 → 无窗口
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.0)], true, None);
        assert!(!book.has_pending_departure(2));
        // candidate Buy3（向上离开）→ 窗口置位（49课行68）
        book.ingest(2, &[ev(BspClass::Buy3, false, 10, 1.0, 2.0)], true, None);
        assert!(book.has_pending_departure(2));
        assert_eq!(book.cf_windows, 1);
        // 价格仍在 ZG 之上 → 窗口保持
        book.negate_pending_departure(2, 2.5);
        assert!(book.has_pending_departure(2));
        // 回试跌回 ZG 下 → 否定解冻（"能回到中枢就不是第三类买点"）
        book.negate_pending_departure(2, 1.9);
        assert!(!book.has_pending_departure(2));
        assert_eq!(book.cf_negations, 1);
        // 新离开段（新 seg_idx 的 candidate 事件）→ 重新置位
        book.ingest(2, &[ev(BspClass::Buy3, false, 10, 1.0, 2.0)], true, None);
        assert!(book.has_pending_departure(2));
        // confirmed Buy3（中枢死亡）→ 窗口解除，dead/frozen 语义接管
        book.ingest(2, &[ev(BspClass::Buy3, true, 10, 1.0, 2.0)], true, None);
        assert!(!book.has_pending_departure(2));
        assert!(book.is_frozen(2));
    }

    #[test]
    fn h1_sell_side_departure_negated_above_zd() {
        let mut book = CenterBook::new();
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.0)], true, None);
        // candidate Sell3（向下离开）→ 窗口置位
        book.ingest(2, &[ev(BspClass::Sell3, false, 10, 1.0, 2.0)], true, None);
        assert!(book.has_pending_departure(2));
        // 价格仍在 ZD 之下 → 保持
        book.negate_pending_departure(2, 0.8);
        assert!(book.has_pending_departure(2));
        // 回到 ZD 之上 → 否定解冻
        book.negate_pending_departure(2, 1.2);
        assert!(!book.has_pending_departure(2));
    }

    #[test]
    fn h2_up_strength_lifecycle() {
        let mut book = CenterBook::new();
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.0)], true, None);
        // 零历史 → Newborn（新生保守默认）
        assert_eq!(book.up_strength_verdict(2, 10), UpStrengthVerdict::Newborn);
        // 第一次向上离开：窗口内 excursion 峰值 0.5（c=2.5），价格否定推入
        book.ingest(2, &[ev(BspClass::Buy3, false, 10, 1.0, 2.0)], true, None);
        book.negate_pending_departure(2, 2.5);
        book.negate_pending_departure(2, 1.9); // 否定 → 推入 0.5
        assert_eq!(book.up_strength_verdict(2, 10), UpStrengthVerdict::Newborn); // 仅1条
        // 第二次离开力度 0.3 < 0.5 → 收敛放行
        book.ingest(2, &[ev(BspClass::Buy3, false, 10, 1.0, 2.0)], true, None);
        book.negate_pending_departure(2, 2.3);
        book.negate_pending_departure(2, 1.8);
        assert_eq!(book.up_strength_verdict(2, 10), UpStrengthVerdict::Converged);
        // 第三次离开力度 0.9 > 0.3（环形最近两次比较）→ 扩张拒开
        book.ingest(2, &[ev(BspClass::Buy3, false, 10, 1.0, 2.0)], true, None);
        book.negate_pending_departure(2, 2.9);
        book.negate_pending_departure(2, 1.5);
        assert_eq!(book.up_strength_verdict(2, 10), UpStrengthVerdict::Expanding);
    }

    #[test]
    fn h2_excursion_uses_current_zg() {
        let mut book = CenterBook::new();
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.0)], true, None);
        book.ingest(2, &[ev(BspClass::Buy3, false, 10, 1.0, 2.0)], true, None);
        book.negate_pending_departure(2, 3.0); // exc = 1.0（ZG=2.0）
        // 中枢延伸 ZG → 2.5：后续 excursion 相对当下 ZG（当下性声明）
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.5)], true, None);
        book.negate_pending_departure(2, 3.2); // exc = 0.7 < 1.0 不更新
        book.negate_pending_departure(2, 2.4); // c < 2.5 否定 → 推入 1.0
        book.ingest(2, &[ev(BspClass::Buy3, false, 10, 1.0, 2.5)], true, None);
        book.negate_pending_departure(2, 3.4); // exc = 0.9 < 1.0
        book.negate_pending_departure(2, 2.0); // 推入 0.9 → 收敛
        assert_eq!(book.up_strength_verdict(2, 10), UpStrengthVerdict::Converged);
    }

    #[test]
    fn h2_death_dismissed_window_not_recorded() {
        let mut book = CenterBook::new();
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.0)], true, None);
        book.ingest(2, &[ev(BspClass::Buy3, false, 10, 1.0, 2.0)], true, None);
        book.negate_pending_departure(2, 2.5);
        // confirmed Buy3 杀中枢 → 窗口解除但不推入（离开段终结中枢，
        // 不是"继续是中枢震荡"的样本）
        book.ingest(2, &[ev(BspClass::Buy3, true, 10, 1.0, 2.0)], true, None);
        assert_eq!(book.up_strength_verdict(2, 10), UpStrengthVerdict::Newborn);
    }

    #[test]
    fn h2_new_center_resets_history() {
        let mut book = CenterBook::new();
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.0)], true, None);
        for c_peak in [2.9, 2.3] {
            book.ingest(2, &[ev(BspClass::Buy3, false, 10, 1.0, 2.0)], true, None);
            book.negate_pending_departure(2, c_peak);
            book.negate_pending_departure(2, 1.5);
        }
        assert_eq!(book.up_strength_verdict(2, 10), UpStrengthVerdict::Converged);
        // 新中枢（cs=20）→ 历史与中枢同生命周期，新锚查询回 Newborn
        book.ingest(2, &[ev(BspClass::Sell1, true, 20, 3.0, 4.0)], true, None);
        assert_eq!(book.up_strength_verdict(2, 20), UpStrengthVerdict::Newborn);
    }

    #[test]
    fn h2_sell_side_window_not_recorded() {
        let mut book = CenterBook::new();
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.0)], true, None);
        // 向下离开窗口（candidate Sell3）价格否定——不进向上力度历史
        // （设计 §2 方向声明：只比较向上离开段序列）
        book.ingest(2, &[ev(BspClass::Sell3, false, 10, 1.0, 2.0)], true, None);
        book.negate_pending_departure(2, 0.5);
        book.negate_pending_departure(2, 1.2);
        book.ingest(2, &[ev(BspClass::Sell3, false, 10, 1.0, 2.0)], true, None);
        book.negate_pending_departure(2, 0.7);
        book.negate_pending_departure(2, 1.3);
        assert_eq!(book.up_strength_verdict(2, 10), UpStrengthVerdict::Newborn);
    }

    #[test]
    fn h1_new_center_clears_window() {
        let mut book = CenterBook::new();
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.0)], true, None);
        book.ingest(2, &[ev(BspClass::Buy3, false, 10, 1.0, 2.0)], true, None);
        assert!(book.has_pending_departure(2));
        // 新中枢形成（cs 变化）→ 旧锚窗口解除
        book.ingest(2, &[ev(BspClass::Sell1, true, 20, 3.0, 4.0)], true, None);
        assert!(!book.has_pending_departure(2));
    }

    // ═══════════════════════════════════════════════════════════════════
    // 票 #637：中枢死亡证明消费（修复轮，2026-07-29 编排者裁定 1A/2A/3A/4A）
    // ═══════════════════════════════════════════════════════════════════

    use crate::theta_v0::classifier::level_view::CoordinateWindow;
    use crate::theta_v0::classifier::retrace_ledger::{
        CenterFrame, RetraceInput, RetraceLedger, RetraceOutcome, RetracePoint as LedgerPoint,
        RetraceProvenance, StrictCompletedPair,
    };
    use crate::theta_v0::types::Direction as LedgerDirection;

    /// 手搓证明——只用于负控构造（判同失败 / 幂等 / 改口边界）。真实产出路径见
    /// [`real_death_certificate`]（票面验收第 1 条「端到端」的正确用法，影子 MEDIUM-2）。
    fn cert(
        start_index: usize,
        zd: i64,
        zg: i64,
        issued_as_of: usize,
        side: RetraceSide,
    ) -> CenterDeathCertificate {
        CenterDeathCertificate {
            center: CenterFrame { zd, zg, start_index, end_index: start_index + 5 },
            side,
            issued_as_of,
        }
    }

    /// 真实产出一张死亡证明：经 `RetraceLedger::observe` 判胜（Success）落锤，
    /// 从 `RetraceLedger::death_certificate` 取出——不手搓 `CenterFrame` 字面量。
    /// `leave_direction` 决定证明的 `side`（`Up`→`Buy`/`Down`→`Sell`，见
    /// `RetraceSide::from_departure`），供票 #664 方向端到端测试驱动两侧场景。
    fn real_death_certificate(
        start_index: usize,
        zd: i64,
        zg: i64,
        end_index: usize,
        confirmed_as_of: usize,
        leave_direction: LedgerDirection,
    ) -> CenterDeathCertificate {
        let center = CenterFrame { zd, zg, start_index, end_index };
        let mut ledger = RetraceLedger::new(RetraceProvenance {
            level: 2,
            window: CoordinateWindow { start: 0, end: 9_999 },
            data_basis: "center_book-test".to_owned(),
        });
        let retest_direction = match leave_direction {
            LedgerDirection::Up => LedgerDirection::Down,
            LedgerDirection::Down => LedgerDirection::Up,
        };
        let (leave_price, retest_price) = match leave_direction {
            LedgerDirection::Up => (zg + 50, zg + 10),
            LedgerDirection::Down => (zd - 50, zd - 10),
        };
        let input = RetraceInput {
            center,
            pair: StrictCompletedPair { leave_move_index: 3, retest_move_index: 4 },
            leave_direction,
            retest_direction,
            leave_end: LedgerPoint { index: end_index + 10, price: leave_price },
            retest_end: Some(LedgerPoint { index: end_index + 20, price: retest_price }),
            outcome: Some(RetraceOutcome::Success),
            as_of: confirmed_as_of,
        };
        ledger.observe(&input).unwrap();
        ledger.death_certificate(center.anchor()).unwrap()
    }

    #[test]
    fn death_certificate_registers_broken_end_to_end() {
        let mut book = CenterBook::new();
        // 产出：中枢形成于本账（seg_start=10）
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.0)], true, None);
        assert!(book.alive(2).is_some());
        let v_before = book.version;
        // 真实产证明：经 RetraceLedger 判胜落锤（非手搓字面量，锚判同 start_index=10）
        let c = real_death_certificate(10, 1, 2, 15, 99, LedgerDirection::Up);
        let outcome = book.consume_death_certificate(2, &c, true, None);
        assert_eq!(outcome, Ok(DeathCertificateOutcome::Broken));
        // 中枢态 Broken：is_dead 为真，alive 归空，version 前进
        assert!(book.is_dead(2, 10));
        assert!(book.alive(2).is_none());
        assert!(book.version > v_before);
    }

    // ═══════════════════════════════════════════════════════════════════
    // 票 #664：cert 杀路径方向语义（dead_down / frozen / CenterEvent::Terminated）
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn sell_certificate_end_to_end_registers_dead_down_with_correct_direction_reads() {
        let mut book = CenterBook::new();
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.0)], true, None);
        // 真实产证明：leave_direction=Down ⟹ side=Sell（三卖，向下离开终结）
        let c = real_death_certificate(10, 1, 2, 15, 99, LedgerDirection::Down);
        assert_eq!(c.side, RetraceSide::Sell);
        let mut out = Vec::new();
        let outcome = book.consume_death_certificate(2, &c, true, Some(&mut out));
        assert_eq!(outcome, Ok(DeathCertificateOutcome::Broken));
        // 生产消费方读数方向正确：三卖终结 ⟹ is_dead_down 为真，"不能回补"
        assert!(book.is_dead(2, 10));
        assert!(book.is_dead_down(2, 10));
        // Sell 侧不置 frozen（frozen 只对 Buy 侧、hard_type3 生效，同 ingest 判据）
        assert!(!book.is_frozen(2));
        assert_eq!(out.len(), 1);
        assert!(matches!(
            out[0],
            CenterEvent::Terminated { seg_start: 10, direction: Direction::Down }
        ));
    }

    #[test]
    fn buy_certificate_end_to_end_registers_up_direction_and_frozen_semantics() {
        let mut book = CenterBook::new();
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.0)], true, None);
        // 真实产证明：leave_direction=Up ⟹ side=Buy（三买，向上离开终结）
        let c = real_death_certificate(10, 1, 2, 15, 99, LedgerDirection::Up);
        assert_eq!(c.side, RetraceSide::Buy);
        let mut out = Vec::new();
        let outcome = book.consume_death_certificate(2, &c, true, Some(&mut out));
        assert_eq!(outcome, Ok(DeathCertificateOutcome::Broken));
        // Buy 侧不进 dead_down（三买按"立即回补"，非"不能回补"）
        assert!(book.is_dead(2, 10));
        assert!(!book.is_dead_down(2, 10));
        // frozen 语义按设计落定：hard_type3=true + Buy ⟹ 置 frozen（判据字面同 ingest）
        assert!(book.is_frozen(2));
        assert_eq!(out.len(), 1);
        assert!(matches!(
            out[0],
            CenterEvent::Terminated { seg_start: 10, direction: Direction::Up }
        ));
    }

    #[test]
    fn buy_certificate_without_hard_type3_does_not_freeze() {
        let mut book = CenterBook::new();
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.0)], true, None);
        let c = real_death_certificate(10, 1, 2, 15, 99, LedgerDirection::Up);
        // hard_type3=false（入参，同 ingest 语义）⟹ 即便 Buy 侧也不置 frozen
        let outcome = book.consume_death_certificate(2, &c, false, None);
        assert_eq!(outcome, Ok(DeathCertificateOutcome::Broken));
        assert!(!book.is_frozen(2));
    }

    #[test]
    fn already_broken_outcome_does_not_reemit_terminated_event() {
        let mut book = CenterBook::new();
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.0)], true, None);
        let c = cert(10, 1, 2, 5, RetraceSide::Sell);
        let mut out = Vec::new();
        assert_eq!(
            book.consume_death_certificate(2, &c, true, Some(&mut out)),
            Ok(DeathCertificateOutcome::Broken)
        );
        assert_eq!(out.len(), 1);
        // 同一证明重复消费 → AlreadyBroken，零动作——不重发 CenterEvent（避免同一教义
        // 事件被两条通道各报一次）
        out.clear();
        assert_eq!(
            book.consume_death_certificate(2, &c, true, Some(&mut out)),
            Ok(DeathCertificateOutcome::AlreadyBroken)
        );
        assert!(out.is_empty());
    }

    #[test]
    fn ingest_kill_then_same_side_certificate_does_not_overwrite_direction() {
        let mut book = CenterBook::new();
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.0)], true, None);
        // confirmed Sell3（ingest 自身的三类点杀链，向下离开）先终结该中枢
        book.ingest(2, &[ev(BspClass::Sell3, true, 10, 1.0, 2.0)], true, None);
        assert!(book.is_dead(2, 10));
        assert!(book.is_dead_down(2, 10)); // ingest（先杀通道）已正确落位
        let v_after_ingest_kill = book.version;
        // 证明后到，同锚同方向——AlreadyBroken，不覆写：方向读数保持 ingest 落位的原值
        let c = cert(10, 1, 2, 0, RetraceSide::Sell);
        let mut out = Vec::new();
        assert_eq!(
            book.consume_death_certificate(2, &c, true, Some(&mut out)),
            Ok(DeathCertificateOutcome::AlreadyBroken)
        );
        assert_eq!(book.version, v_after_ingest_kill);
        assert!(book.is_dead_down(2, 10)); // 未被 cert 触碰，仍是 ingest 落位的值
        assert!(out.is_empty()); // AlreadyBroken 不重发事件
    }

    #[test]
    fn ingest_kill_then_conflicting_side_certificate_does_not_overwrite_frozen() {
        let mut book = CenterBook::new();
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.0)], true, None);
        // confirmed Sell3（ingest 先杀，向下离开）——frozen 从不因 Sell 侧置位
        book.ingest(2, &[ev(BspClass::Sell3, true, 10, 1.0, 2.0)], true, None);
        assert!(book.is_dead(2, 10));
        assert!(book.is_dead_down(2, 10));
        assert!(!book.is_frozen(2));
        let v_after_ingest_kill = book.version;
        // cert 后到，方向与先杀者冲突（Buy）——AlreadyBroken：本票不设 fail-loud，
        // 但也不覆写；先杀者（ingest Sell）落位的方向读数原样保持，frozen 不被
        // cert 的 Buy 侧静默翻成 Some，也不补发第二次 Terminated
        let c = cert(10, 1, 2, 0, RetraceSide::Buy);
        let mut out = Vec::new();
        assert_eq!(
            book.consume_death_certificate(2, &c, true, Some(&mut out)),
            Ok(DeathCertificateOutcome::AlreadyBroken)
        );
        assert_eq!(book.version, v_after_ingest_kill);
        assert!(book.is_dead_down(2, 10)); // 未被覆写/清除
        assert!(!book.is_frozen(2)); // 未被静默翻转
        assert!(out.is_empty()); // 无第二次 Terminated
    }

    /// 影子评审 MEDIUM-2：镜像上一条负控——`ingest` 先以 hard Buy3 杀（`frozen` 落位，
    /// `dead_down` 保持 false），cert 携冲突方向 Sell 后到 ⟹ `AlreadyBroken`，`dead_down`
    /// 必须保持 false（不被 cert 的 Sell 静默插入）。这是「最危险的静默翻转」的真负控——
    /// 若把 `:474-476` 的 `dead_down` 写移出 `!already_dead` 守卫（回退到首轮实现），本测试
    /// 必须失败；此前 `ingest_kill_then_same_side_certificate_does_not_overwrite_direction`
    /// （同方向 Sell/Sell）无法区分"不写"与"写成同值"，不构成该分支的负控。
    #[test]
    fn ingest_kill_then_conflicting_side_certificate_does_not_overwrite_dead_down() {
        let mut book = CenterBook::new();
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.0)], true, None);
        // confirmed Buy3（ingest 先杀，向上离开，hard_type3=true）——dead_down 从不因 Buy 侧置位
        book.ingest(2, &[ev(BspClass::Buy3, true, 10, 1.0, 2.0)], true, None);
        assert!(book.is_dead(2, 10));
        assert!(!book.is_dead_down(2, 10));
        assert!(book.is_frozen(2));
        let v_after_ingest_kill = book.version;
        // cert 后到，方向与先杀者冲突（Sell）——AlreadyBroken：不覆写；先杀者（ingest Buy）
        // 落位的方向读数原样保持，dead_down 不被 cert 的 Sell 侧静默插入，也不补发第二次
        // Terminated
        let c = cert(10, 1, 2, 0, RetraceSide::Sell);
        let mut out = Vec::new();
        assert_eq!(
            book.consume_death_certificate(2, &c, true, Some(&mut out)),
            Ok(DeathCertificateOutcome::AlreadyBroken)
        );
        assert_eq!(book.version, v_after_ingest_kill);
        assert!(!book.is_dead_down(2, 10)); // 未被静默插入
        assert!(book.is_frozen(2)); // 未被覆写/清除
        assert!(out.is_empty()); // 无第二次 Terminated
    }

    #[test]
    fn no_matching_center_never_seen_fails_loud() {
        let mut book = CenterBook::new();
        // 该层从未见过任何中枢——anchor 不在 known 集
        let c = cert(10, 1, 2, 0, RetraceSide::Sell);
        assert_eq!(
            book.consume_death_certificate(2, &c, true, None),
            Err(DeathCertificateError::NoMatchingCenter { ladder: 2, anchor: 10 })
        );
    }

    #[test]
    fn superseded_anchor_certificate_still_registers_broken() {
        let mut book = CenterBook::new();
        // 旧中枢形成（seg_start=10），随后被新中枢取代（cs 变化，未经三类点杀）
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.0)], true, None);
        book.ingest(2, &[ev(BspClass::Sell1, true, 20, 3.0, 4.0)], true, None);
        assert!(book.alive(2).map(|lc| lc.seg_start) == Some(20));
        // 裁定 1A：被更替 ≠ 无此中枢——迟到证明照登记 Broken（补充十一/#583）
        let c = cert(10, 1, 2, 1, RetraceSide::Sell);
        assert_eq!(
            book.consume_death_certificate(2, &c, true, None),
            Ok(DeathCertificateOutcome::Broken)
        );
        assert!(book.is_dead(2, 10));
        // 新中枢不受影响
        assert!(book.alive(2).map(|lc| lc.seg_start) == Some(20));
    }

    #[test]
    fn ingest_kill_then_certificate_is_idempotent_and_archives_frame() {
        let mut book = CenterBook::new();
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.0)], true, None);
        // confirmed Buy3（ingest 自身的三类点杀链）先终结该中枢
        book.ingest(2, &[ev(BspClass::Buy3, true, 10, 1.0, 2.0)], true, None);
        assert!(book.is_dead(2, 10));
        let v_after_ingest_kill = book.version;
        // 证明后到——同一教义事件的第二次观测（定理三充要），幂等确认，version 不再前进
        let c = cert(10, 1, 2, 0, RetraceSide::Buy);
        assert_eq!(
            book.consume_death_certificate(2, &c, true, None),
            Ok(DeathCertificateOutcome::AlreadyBroken)
        );
        assert_eq!(book.version, v_after_ingest_kill);
        // 框已存档（供后续重复证明判幂等 / 判改口）
        assert_eq!(
            book.broken_by_certificate[2].as_ref().and_then(|m| m.get(&10)),
            Some(&c.center)
        );
    }

    #[test]
    fn repeated_consumption_is_idempotent_zero_action() {
        let mut book = CenterBook::new();
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.0)], true, None);
        let c = cert(10, 1, 2, 5, RetraceSide::Sell);
        assert_eq!(
            book.consume_death_certificate(2, &c, true, None),
            Ok(DeathCertificateOutcome::Broken)
        );
        let v_after_first = book.version;
        // 同一证明重复消费 → 零动作（幂等），版本号不再前进
        assert_eq!(
            book.consume_death_certificate(2, &c, true, None),
            Ok(DeathCertificateOutcome::AlreadyBroken)
        );
        assert_eq!(book.version, v_after_first);
        assert!(book.is_dead(2, 10));
    }

    #[test]
    fn conflicting_certificate_same_anchor_different_frame_fails_loud() {
        let mut book = CenterBook::new();
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.0)], true, None);
        let first = cert(10, 1, 2, 5, RetraceSide::Sell);
        assert_eq!(
            book.consume_death_certificate(2, &first, true, None),
            Ok(DeathCertificateOutcome::Broken)
        );
        // 同锚不同框——上游改口，fail-loud（影子 MEDIUM-1：幂等键从锚收紧为锚+框）
        let conflicting = cert(10, 999, 999, 0, RetraceSide::Sell);
        assert_eq!(
            book.consume_death_certificate(2, &conflicting, true, None),
            Err(DeathCertificateError::ConflictingCertificate { ladder: 2, anchor: 10 })
        );
    }

    #[test]
    fn cert_kill_clears_pending_departure_window_and_negate_is_noop() {
        let mut book = CenterBook::new();
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.0)], true, None);
        // candidate Buy3（向上离开）→ 禁令窗口置位
        book.ingest(2, &[ev(BspClass::Buy3, false, 10, 1.0, 2.0)], true, None);
        assert!(book.has_pending_departure(2));
        let sg_records_before = book.sg_records;
        let cf_negations_before = book.cf_negations;
        // cert 杀（同一向上离开段的三买终结）→ 窗口应解除（影子 HIGH-5：对齐 ingest 杀路径的收尾）
        let c = cert(10, 1, 2, 0, RetraceSide::Buy);
        assert_eq!(
            book.consume_death_certificate(2, &c, true, None),
            Ok(DeathCertificateOutcome::Broken)
        );
        assert!(!book.has_pending_departure(2));
        // 后续 negate_pending_departure 不应推 up_strength（窗口已不存在，函数早退）
        book.negate_pending_departure(2, 1.9);
        assert_eq!(book.sg_records, sg_records_before);
        assert_eq!(book.cf_negations, cf_negations_before);
        assert_eq!(book.up_strength_verdict(2, 10), UpStrengthVerdict::Newborn);
    }

    #[test]
    fn late_issued_as_of_does_not_block_broken_when_anchor_still_present() {
        let mut book = CenterBook::new();
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.0)], true, None);
        // 迟到证明（issued_as_of 很小）+ 锚仍是当前 last——不问迟到，正常登记 Broken
        let c = cert(10, 1, 2, 1, RetraceSide::Sell);
        assert_eq!(
            book.consume_death_certificate(2, &c, true, None),
            Ok(DeathCertificateOutcome::Broken)
        );
        assert!(book.is_dead(2, 10));
    }
}
