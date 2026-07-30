//! 信号层消费：BarSig → 群事件映射（架构 §8）。
//!
//! v2 **不重写信号层**，消费现有契约（`trading::tape::BarSig` + `BspEvent`）。信号层是
//! 操作层的**唯一定义域单元**。本模块把信号产出映射为螺旋群事件：
//! - `buy1/sell1` confirmed type1 @ ladder k → φ=0 奇点在 r=k（候选 F/C 势源）。
//! - `max_ladder` → r 上界（root_emergent_ladder 爬升上界）。
//! - 向心 confirm（`helix_centripetal_confirm`）→ nf fire @ 自层（供 D/E，N7）+
//!   confirm @ source（供级联 located → C/F，N5）。
//!
//! ## φ 隐式裁决（架构 §8.5）
//! BspEvent 无 φ 角向坐标字段。confirm fire 是 `φ→0` 的代理；操作层只需 φ=0 唯一奇点。
//! `SpiralState.phi` 是观测/标注量，不字面追踪 23 环（避免引入环边界划分参数，231号零参数）。
//!
//! ## 结构守卫（信号层 located 链，定义于本模块——近数据原则）
//! `prove_chain`（N5/N6 链顶一致+时序）/ `prove_n5_cascade`（连续前缀）/
//! `prove_t52_gauge_fix`（区间套规范固定）/ `prove_t53_connection_assoc`（连接结合律）。
//! 群关系 L0 + 会计 L2 守卫在 `prove.rs`（架构 §7.1/§7.2）。

use super::params::{DEPTH_REF_WINDOW, FIRST_BSP_LADDER, PENDING_LO};
use super::prove::prove_n3_type2;
use super::result::SpiralResult;
use crate::buysellpoint::{BspKind, Side};
use crate::stroke::Direction;
use crate::trading::center_book::CenterBook;
use crate::trading::depth_ref::DepthRef;
use crate::trading::tape::BarSig;
use crate::trading::types::{BspEvent, MAX_LADDER};

/// pending 窗口（高级别 candidate 持续记忆——540号压缩侧↑载体）。
/// `since_bar` = candidate 首现 bar（压缩完成 φ→0）；极值刷新保留首现值。
#[derive(Debug, Clone, Copy)]
struct Pending {
    extreme: f64,
    since_bar: i64,
}

/// pending confirm 兑现条目（N5/N6 载体；级联后一条链内全层 source_ladder 统一）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PendingLocate {
    pub extreme: f64,
    pub source_ladder: usize,
    pub direction: Side,
    pub compress_bar: i64,
    pub confirm_bar: i64,
}

/// 本 bar 的群事件帧（engine 消费）：nf fire（D/E）+ 链顶 source（C/F）+ r 上界。
#[derive(Debug, Clone)]
pub struct GroupEventFrame {
    /// 自层卖侧向心 confirm fire（供多头 voice E/D，N7）。
    pub nf_sell: [Option<f64>; MAX_LADDER],
    /// 自层买侧向心 confirm fire（供空头 voice E/D，N7）。
    pub nf_buy: [Option<f64>; MAX_LADDER],
    /// 卖侧级联链顶 source（C 翻空/清仓势源）。
    pub sell_source: Option<usize>,
    /// 买侧级联链顶 source（F 入场 / C 翻多势源）。
    pub buy_source: Option<usize>,
    /// 涌现爬升上界（max_ladder+1，封顶 MAX_LADDER）。
    pub max_l: usize,
}

/// 级联武装 located（N5 第14环）：confirm@source ⇒ 武装 `[FIRST_BSP..=source]` 全层。
/// 高 source 优先。`source ≥ PENDING_LO`（segment 非势源）。
fn cascade_arm(
    located: &mut [Option<PendingLocate>; MAX_LADDER],
    dir: Side,
    source: usize,
    extreme: f64,
    compress_bar: i64,
    confirm_bar: i64,
) {
    debug_assert!(
        source >= PENDING_LO,
        "pending 只在 move(L1) 及以上注册；source={source}"
    );
    debug_assert!(
        compress_bar < confirm_bar,
        "540号严格时序：压缩 < 展开（同 bar=伪确认）"
    );
    for slot in located.iter_mut().take(source + 1).skip(FIRST_BSP_LADDER) {
        let overwrite = slot.map_or(true, |e| source >= e.source_ladder);
        if overwrite {
            *slot = Some(PendingLocate {
                extreme,
                source_ladder: source,
                direction: dir,
                compress_bar,
                confirm_bar,
            });
        }
    }
}

/// 级联链顶 S（最高有 located 的层）。
fn chain_source(located: &[Option<PendingLocate>; MAX_LADDER]) -> Option<usize> {
    (FIRST_BSP_LADDER..MAX_LADDER)
        .rev()
        .find(|&k| located[k].is_some())
}

/// 升序 bar 序列中 ≤ `ub` 的最大值（向心回溯内圈末段 settle bar）。
fn latest_le(hist: &[i64], ub: i64) -> Option<i64> {
    let cnt = hist.partition_point(|&b| b <= ub);
    (cnt > 0).then(|| hist[cnt - 1])
}

/// **T49（confirm 向心回溯）**：candidate@k 沿 φ=0 母线**向心**逐圈 k−1…FIRST_BSP
/// 检验**已 settle 的内圈 type1**（嵌套：末段 sub-component 在外层之内，settle bar
/// ≤ 外层 settle bar）。内圈 type1 在 candidate 之过去（展开恒等式 Δt∝λʲ⁻¹(λ−1)>0），
/// **非前向等待**。返回母线是否逐圈贯通（贯通 ⇒ confirm@k）。
fn helix_centripetal_confirm(
    type1_hist: &[[Vec<i64>; 2]; MAX_LADDER],
    k: usize,
    side: Side,
    since: i64,
) -> bool {
    let si = match side {
        Side::Sell => 0,
        Side::Buy => 1,
    };
    let mut upper = since;
    for j in (FIRST_BSP_LADDER..k).rev() {
        match latest_le(&type1_hist[j][si], upper) {
            Some(b) => upper = b,
            None => return false,
        }
    }
    true
}

/// 信号层滚动状态（nest 窗口 + located 链 + type1 历史 + 方向锚 + 成本门）。
pub struct SignalState {
    nest_sell: [Option<Pending>; MAX_LADDER],
    nest_buy: [Option<Pending>; MAX_LADDER],
    /// 级联定位链（卖侧出场链 + 买侧入场链）；pub ⇒ engine 读（prove_chain）+ 清链（F/C 后）。
    pub located_sell: [Option<PendingLocate>; MAX_LADDER],
    pub located_buy: [Option<PendingLocate>; MAX_LADDER],
    /// 每级 type1 settle 历史（bar 升序 append；[Sell,Buy] 双侧）。
    type1_hist: [[Vec<i64>; 2]; MAX_LADDER],
    /// 方向/段锚滚动状态（T5 根级别涌现 E* 读数）。pub ⇒ engine 读（root_emergent）。
    pub dir_state: [Option<Direction>; MAX_LADDER],
    pub anchor_state: [i64; MAX_LADDER],
    /// 中枢账本（成本门振幅参照的事件源；复用 unn pub 类型，bit-exact 前提）。
    book: CenterBook,
    /// 成本门振幅参照（N4；复用 unn `DepthRef`，与 unn 同 θ 机件）。pub ⇒ engine E 读。
    pub depth: DepthRef,
}

impl Default for SignalState {
    fn default() -> Self {
        SignalState::new()
    }
}

impl SignalState {
    pub fn new() -> Self {
        SignalState {
            nest_sell: [None; MAX_LADDER],
            nest_buy: [None; MAX_LADDER],
            located_sell: [None; MAX_LADDER],
            located_buy: [None; MAX_LADDER],
            type1_hist: Default::default(),
            dir_state: [None; MAX_LADDER],
            anchor_state: [-1; MAX_LADDER],
            book: CenterBook::new(),
            depth: DepthRef::new(DEPTH_REF_WINDOW),
        }
    }

    /// 清卖侧 located 链（C 翻空 / 清仓后）。
    pub fn clear_located_sell(&mut self) {
        self.located_sell = [None; MAX_LADDER];
    }

    /// 清买侧 located 链（F 入场 / C 翻多后）。
    pub fn clear_located_buy(&mut self) {
        self.located_buy = [None; MAX_LADDER];
    }

    /// 单 bar 信号处理 → 群事件帧。内含 N3/N5/T52/T53 信号层结构守卫。
    pub fn process(
        &mut self,
        sig: &BarSig,
        flip_edge: &[Option<Direction>; MAX_LADDER],
        bar: i64,
        res: &mut SpiralResult,
    ) -> GroupEventFrame {
        let c = sig.close;

        // 方向/段锚滚动状态（T5 根级别涌现读数载体）。
        for lad in 0..MAX_LADDER {
            if let Some(d) = flip_edge[lad] {
                self.dir_state[lad] = Some(d);
                self.anchor_state[lad] = bar;
            }
        }
        let max_l = (sig.max_ladder as usize + 1).min(MAX_LADDER);

        // 市场性质：中枢账本 ingest + 振幅参照 observe（unn step §1134-1147 逐字——
        // bit-exact 前提：同一 CenterBook/DepthRef 机件，成本门 θ 与 unn 一致）。
        let empty: [Vec<BspEvent>; MAX_LADDER] = Default::default();
        let evrows: &[Vec<BspEvent>; MAX_LADDER] = sig.bsp_events.as_deref().unwrap_or(&empty);
        if sig.bsp_events.is_some() {
            for (lad, row) in evrows
                .iter()
                .enumerate()
                .take(MAX_LADDER)
                .skip(FIRST_BSP_LADDER)
            {
                self.book.ingest(lad, row, true, None);
            }
            self.depth.observe(&self.book, c);
        }

        // ── pending 窗口维护 + 向心 confirm（双侧，k ≥ PENDING_LO）──
        let mut confirm_sell: [Option<(f64, i64)>; MAX_LADDER] = [None; MAX_LADDER];
        let mut confirm_buy: [Option<(f64, i64)>; MAX_LADDER] = [None; MAX_LADDER];
        let mut nf_sell: [Option<f64>; MAX_LADDER] = [None; MAX_LADDER];
        let mut nf_buy: [Option<f64>; MAX_LADDER] = [None; MAX_LADDER];
        let mut type2_seen = 0u64;
        let mut type2_handled = 0u64;
        for k in PENDING_LO..MAX_LADDER {
            // ① 破极值否定（027:25）。
            if self.nest_sell[k].is_some_and(|w| c > w.extreme) {
                self.nest_sell[k] = None;
                res.n_breaks_by_ladder[k] += 1;
            }
            if self.nest_buy[k].is_some_and(|w| c < w.extreme) {
                self.nest_buy[k] = None;
                res.n_breaks_by_ladder[k] += 1;
            }
            // ② candidate 武装（N3：type2 按 side() 同等武装，无 continue）/ confirmed 清窗。
            if sig.bsp_events.is_some() {
                for e in &evrows[k] {
                    if e.class.kind() == BspKind::Type2 {
                        type2_seen += 1;
                    }
                    let sellside = matches!(e.class.side(), Side::Sell);
                    let win = if sellside {
                        &mut self.nest_sell[k]
                    } else {
                        &mut self.nest_buy[k]
                    };
                    if e.confirmed {
                        *win = None;
                    } else {
                        let (ext, since) = win.map_or((e.price, bar), |w| {
                            let ext = if sellside {
                                w.extreme.max(e.price)
                            } else {
                                w.extreme.min(e.price)
                            };
                            (ext, w.since_bar)
                        });
                        *win = Some(Pending {
                            extreme: ext,
                            since_bar: since,
                        });
                        res.n_arms_by_ladder[k] += 1;
                    }
                    if e.class.kind() == BspKind::Type2 {
                        type2_handled += 1;
                    }
                }
            }
            // ③ 向心 confirm（T49）：母线逐圈贯通 ∧ since<bar ⇒ confirm@k。
            if let Some(w) = self.nest_sell[k] {
                if w.since_bar < bar
                    && helix_centripetal_confirm(&self.type1_hist, k, Side::Sell, w.since_bar)
                {
                    confirm_sell[k] = Some((w.extreme, w.since_bar));
                    nf_sell[k] = Some(w.extreme);
                    res.n_fire_sell_by_ladder[k] += 1;
                    self.nest_sell[k] = None;
                }
            }
            if let Some(w) = self.nest_buy[k] {
                if w.since_bar < bar
                    && helix_centripetal_confirm(&self.type1_hist, k, Side::Buy, w.since_bar)
                {
                    confirm_buy[k] = Some((w.extreme, w.since_bar));
                    nf_buy[k] = Some(w.extreme);
                    res.n_fire_buy_by_ladder[k] += 1;
                    self.nest_buy[k] = None;
                }
            }
        }
        prove_n3_type2(type2_seen, type2_handled, bar);

        // ── 记录本 bar 各层 type1 settle（向心回溯历史；confirm 之后记录）──
        if sig.bsp_events.is_some() {
            for (j, hist_j) in self
                .type1_hist
                .iter_mut()
                .enumerate()
                .take(MAX_LADDER)
                .skip(FIRST_BSP_LADDER)
            {
                for e in &evrows[j] {
                    if e.class.kind() == BspKind::Type1 {
                        let si = match e.class.side() {
                            Side::Sell => 0,
                            Side::Buy => 1,
                        };
                        hist_j[si].push(bar);
                    }
                }
            }
        }

        // ── 级联武装 located（N5；高 source 优先，降序施加）+ T53 结合律 ──
        let pre_sell = self.located_sell;
        let pre_buy = self.located_buy;
        for k in (PENDING_LO..MAX_LADDER).rev() {
            if let Some((ext, since)) = confirm_sell[k] {
                cascade_arm(&mut self.located_sell, Side::Sell, k, ext, since, bar);
            }
            if let Some((ext, since)) = confirm_buy[k] {
                cascade_arm(&mut self.located_buy, Side::Buy, k, ext, since, bar);
            }
        }
        prove_t53_connection_assoc(
            &pre_sell,
            &confirm_sell,
            Side::Sell,
            bar,
            &self.located_sell,
        );
        prove_t53_connection_assoc(&pre_buy, &confirm_buy, Side::Buy, bar, &self.located_buy);
        // 破极值否定（级联统一极值 ⇒ 整链同破）。
        for k in FIRST_BSP_LADDER..MAX_LADDER {
            if self.located_sell[k].is_some_and(|e| c > e.extreme) {
                self.located_sell[k] = None;
            }
            if self.located_buy[k].is_some_and(|e| c < e.extreme) {
                self.located_buy[k] = None;
            }
        }
        prove_n5_cascade(&self.located_sell, bar, "sell");
        prove_n5_cascade(&self.located_buy, bar, "buy");

        GroupEventFrame {
            nf_sell,
            nf_buy,
            sell_source: chain_source(&self.located_sell),
            buy_source: chain_source(&self.located_buy),
            max_l,
        }
    }
}

// ════════════════════ 信号层结构守卫（located 链）════════════════════

/// **N5+N6（区间套自上而下定位 + 先势后定位时序，第14环/540号）**：操作 source 是
/// 一条操作前已完整级联形成的定位链顶。① source≥PENDING_LO；② located[s] 链顶一致；
/// ③ compress<confirm≤bar；④ [FIRST_BSP..=s] 全 located 且 source 统一。violation=panic。
pub fn prove_chain(
    located: &[Option<PendingLocate>; MAX_LADDER],
    dir: Side,
    s: usize,
    bar: i64,
    op: &str,
) {
    assert!(
        s >= PENDING_LO,
        "N5 违反@bar {bar} {op}：source={s} < move(L1)={PENDING_LO}（segment 非势源）"
    );
    let top = located[s].unwrap_or_else(|| panic!("N5 违反@bar {bar} {op}：source={s} 无 located"));
    assert_eq!(
        top.source_ladder, s,
        "N5 违反@bar {bar} {op}：located[{s}].source_ladder≠{s}"
    );
    assert_eq!(
        top.direction, dir,
        "N5 违反@bar {bar} {op}：located[{s}].direction 方向错配"
    );
    assert!(
        top.compress_bar < top.confirm_bar,
        "N6 违反@bar {bar} {op}：压缩 {} ≥ 展开 {}（伪确认）",
        top.compress_bar,
        top.confirm_bar
    );
    assert!(
        top.confirm_bar <= bar,
        "N6 违反@bar {bar} {op}：confirm_bar={} > bar（未来武装）",
        top.confirm_bar
    );
    for k in FIRST_BSP_LADDER..=s {
        let e = located[k].unwrap_or_else(|| panic!("N5 违反@bar {bar} {op}：定位链层 {k} 断裂"));
        assert_eq!(
            e.source_ladder, s,
            "N5 违反@bar {bar} {op}：located[{k}].source_ladder≠链顶 {s}"
        );
    }
}

/// **N5（级联结构，每 bar 后置）**：located 非空 ⇒ 连续前缀 `[FIRST_BSP..=S]` 且每层
/// `source_ladder ≥ k`（自上而下）。violation = panic。
pub fn prove_n5_cascade(located: &[Option<PendingLocate>; MAX_LADDER], bar: i64, side: &str) {
    if let Some(s) = chain_source(located) {
        for k in FIRST_BSP_LADDER..=s {
            let e = located[k]
                .unwrap_or_else(|| panic!("N5 违反@bar {bar} {side}：层 {k} 空（非连续前缀）"));
            assert!(
                e.source_ladder >= k,
                "N5 违反@bar {bar} {side}：located[{k}].source_ladder<{k}（自下而上）"
            );
            assert!(
                e.source_ladder >= PENDING_LO,
                "N5 违反@bar {bar} {side}：source<move(L1)"
            );
        }
        for k in (s + 1)..MAX_LADDER {
            assert!(
                located[k].is_none(),
                "N5 违反@bar {bar} {side}：源层 {s} 之上层 {k} 有 located（非前缀）"
            );
        }
    }
}

/// **T52（走势多义性，第33课 → 区间套规范固定）**：整个 fiber 由同一区间套 confirm
/// 事件确定（统一 `(compress,confirm)`）= 单一 gauge。与 prove_chain 不重叠（后者不验
/// (compress,confirm) 跨层统一）。violation = panic。
pub fn prove_t52_gauge_fix(
    located: &[Option<PendingLocate>; MAX_LADDER],
    s: usize,
    bar: i64,
    op: &str,
) {
    let top =
        located[s].unwrap_or_else(|| panic!("T52 违反@bar {bar} {op}：塔顶 source={s} 无 located"));
    let gauge = (top.compress_bar, top.confirm_bar);
    for k in FIRST_BSP_LADDER..=s {
        let e =
            located[k].unwrap_or_else(|| panic!("T52 违反@bar {bar} {op}：fiber 层 {k} 缺提升"));
        assert_eq!(
            (e.compress_bar, e.confirm_bar),
            gauge,
            "T52 违反@bar {bar} {op}：fiber 层 {k} gauge≠塔顶（多义性被逐层随意拼接）"
        );
    }
}

/// **T53（走势类型连接结合律，第36课）**：级联 fold 不依赖施加顺序——降序 fold（actual）
/// == 升序 re-fold。violation（顺序依赖=结合律破）= panic。
pub fn prove_t53_connection_assoc(
    pre: &[Option<PendingLocate>; MAX_LADDER],
    confirms: &[Option<(f64, i64)>; MAX_LADDER],
    dir: Side,
    bar: i64,
    post: &[Option<PendingLocate>; MAX_LADDER],
) {
    let mut alt = *pre;
    for k in PENDING_LO..MAX_LADDER {
        if let Some((ext, since)) = confirms[k] {
            cascade_arm(&mut alt, dir, k, ext, since, bar);
        }
    }
    for k in FIRST_BSP_LADDER..MAX_LADDER {
        assert!(
            alt[k] == post[k],
            "T53 违反@bar {bar} {dir:?}：级联连接非结合——层 {k} 升序 fold≠降序 fold"
        );
    }
}
