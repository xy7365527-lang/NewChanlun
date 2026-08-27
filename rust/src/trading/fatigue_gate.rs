//! GUARD-ROLE: organic-fugue-v2-trading-layer——名分：现役（详见 `trading/mod.rs` 头部 GUARD-ROLE 块，#763 C7-E4 核定；零删除/零移入 legacy/）。
//!
//! FatigueGate — 41课守门员，v2 点态衰竭语义（C7）。
//!
//! v1 积累证据集（"曾出现过衰竭证据"）在高密度层恒真（M-d：segment 层证据集
//! 99.95%+ 时间非空，门拒=0，反事实空集）。v2 整体替换为点态语义：
//! "当下处于未被否定的衰竭段内"——41课"该大级别走势没有任何衰竭"的严格读法。
//!
//! 转换规则（C7 步骤6 逐字）：
//!   Fresh → Fatigued：u 层卖侧力度背驰 div（Trend ∨ Consolidation，#985
//!     ForceL force_c < force_a）∨ type1 卖（candidate 含——E4 左侧优先）∨
//!     confirmed type3 卖（BSP 证据定义 v1 零改动；生产磁带走价格振幅 fallback，
//!     T2 是唯一维度 ⟹ 卖侧 div 力度口径与 v1「卖侧 div 即证据」逐位等价）。
//!   Fatigued → Fresh（任一即清，三路同分辨率，M-d 修复）：
//!     (1) c > high_water ∧ 不背驰 —— 强力不背驰创新高 = 衰竭被市场否定
//!         （43:194 答疑逐字；#1232 裁定 a；力度项 = #985 ForceL L(C) >= L(B)，
//!         与 xzd_force_exception 同构同源，不另造判据）
//!     (2) CenterEvent::Formed   —— u 层新中枢 = 新一段上涨结构（C6 供给）
//!     (3) MoveSettled{Up}       —— v1 唯一路径，保留为兜底
//!
//! 事件序纪律（v1 继承）：清空先于添加——同 bar "清空事件 + 新卖侧证据"时，
//! 新证据属于清空之后的市场状态，保留。
//!
//! ## 能力边界（090号声明=能力）
//! 路径(1) 依赖磁带 run_high 行（D3）与 div 事件力度字段（force_a/force_c）。
//! 当前磁带（organic_signals.py）无 run_high 行 ⇒ `rev_gate=true` 的配置在
//! runner 入口被 capability guard 拒绝（fail-fast）；无卖侧 div 的 bar 按
//! 无力度源不判（不清，照 #985）。本模块不提供"用 close 代理 run_high"或
//! "用价格代理力度"的降级路径（代理 = 双真相源）。

use super::types::*;

/// 单层衰竭状态（41课点态语义的直译）。
#[derive(Debug, Clone, Copy)]
pub enum FatigueState {
    /// 无衰竭：u 层当前向上走势力度未见顶。
    Fresh,
    /// 衰竭段内：u 层出现卖侧证据，且尚未被市场否定。
    /// high_water = 进入衰竭时刻 u 层 run 高点（清空路径(1)「创新高 ∧ 不背驰」
    /// 的价格比较基准；力度项另由 div 事件力度字段供给，不存进状态）。
    /// 设计稿含 since_bar 字段——三路清空均不消费它，无消费者的字段是声明
    /// 膨胀（090号），删除；需要衰竭时长报告时随消费者一起恢复。
    Fatigued { high_water: f64 },
}

#[derive(Debug)]
pub struct FatigueGate {
    state: [FatigueState; MAX_LADDER],
    /// "曾涌现结构"的可观测形式 = 该层曾产出任何 BSP/背驰事件（v1 K7 逐字）。
    structure_seen: u16,
}

impl Default for FatigueGate {
    fn default() -> Self {
        FatigueGate {
            state: [FatigueState::Fresh; MAX_LADDER],
            structure_seen: 0,
        }
    }
}

impl FatigueGate {
    pub fn new() -> Self {
        Self::default()
    }

    /// 卖侧衰竭证据谓词（v1 证据定义零改动；BspClass 穷举义务）。
    fn is_evidence(e: &BspEvent) -> bool {
        match e.class {
            // type1 卖：candidate 即可（E4/P3 左侧信号 > 右侧确认）
            BspClass::Sell1 => true,
            // confirmed type3 卖：中枢向下离开 = 上级别自身转入向下段
            BspClass::Sell3 => e.confirmed,
            // Sell2 不是衰竭证据（v1 定义零改动）；买侧三类与衰竭无关
            BspClass::Sell2 | BspClass::Buy1 | BspClass::Buy2 | BspClass::Buy3 => false,
        }
    }

    /// 卖侧力度背驰谓词（#985 ForceL：L(C) < L(B) 口径）。
    ///
    /// div 事件透传 A/C 段力度（force_a/force_c）；force_c < force_a ⟹ 背驰
    /// （弱冲高）。生产磁带走价格振幅 fallback（无 MACD——设计 §5.1 成本声明），
    /// T2 是唯一维度 ⟹ 每个卖侧 div 都满足 force_c < force_a ⟹ 与 v1「卖侧
    /// div 即证据」逐位等价（零行为变化，力度口径只是把隐藏等价显式化）。
    fn is_force_diverged(d: &DivEvent) -> bool {
        d.side() == crate::buysellpoint::Side::Sell && d.force_c < d.force_a
    }

    /// 力度项（#985 ForceL：L(C) >= L(B) 口径，与 `xzd_force_exception` 同构同源）。
    ///
    /// 返回 `Some(true)` ⟹ 不背驰（强力冲高）；`Some(false)` ⟹ 背驰（弱冲高）；
    /// `None` ⟹ 无卖侧 div（无力度源），不判（照 #985 无源不判，不降级）。
    /// 任一卖侧 div 力度背驰即弱冲高——弱冲高不得清 Fatigued。
    fn force_not_diverged(devs: &[DivEvent]) -> Option<bool> {
        let mut has_sell = false;
        for d in devs {
            if d.side() != crate::buysellpoint::Side::Sell {
                continue;
            }
            has_sell = true;
            if d.force_c < d.force_a {
                return Some(false);
            }
        }
        if has_sell {
            Some(true)
        } else {
            None
        }
    }

    /// 消费 u 层本 bar 全部输入，更新点态状态。每 bar 每承载层调用（rev_gate 时）。
    #[allow(clippy::too_many_arguments)]
    pub fn observe(
        &mut self,
        u: usize,
        evs: &[BspEvent],
        devs: &[DivEvent],
        center_evs: &[CenterEvent],
        up_settled: bool,
        c: f64,
        run_high: f64,
    ) {
        if !evs.is_empty() || !devs.is_empty() {
            self.structure_seen |= 1 << u;
        }
        // ── 清空（三路同分辨率，先于添加）──
        if let FatigueState::Fatigued { high_water, .. } = self.state[u] {
            // 路径(1)：创新高 ∧ 不背驰（43:194「强力不背驰创新高」，#1232 裁定 a）。
            // 力度项复用 #985 ForceL（L(C) >= L(B)，与 xzd_force_exception 同构
            // 同源，不另造判据）；无力度源 ⟹ 无源不判（不清）。
            let new_high = c > high_water && matches!(Self::force_not_diverged(devs), Some(true));
            let center_formed = center_evs
                .iter()
                .any(|ce| matches!(ce, CenterEvent::Formed { .. }));
            if new_high || center_formed || up_settled {
                self.state[u] = FatigueState::Fresh;
            }
        }
        // ── 证据进入 ──
        let evidence =
            evs.iter().any(Self::is_evidence) || devs.iter().any(Self::is_force_diverged);
        if evidence {
            if let FatigueState::Fresh = self.state[u] {
                self.state[u] = FatigueState::Fatigued {
                    high_water: run_high,
                };
            }
        }
    }

    /// u(k)：k 之上最近曾涌现结构的承载层（≤ entry_ladder）。无 → None（v1 K7 逐字）。
    pub fn u_of(&self, k: usize, entry_ladder: usize) -> Option<usize> {
        (k + 1..=entry_ladder.min(MAX_LADDER - 1)).find(|&u| (self.structure_seen >> u) & 1 == 1)
    }

    /// 门开 ⇔ u(k) 存在 ∧ state[u(k)] 是 Fatigued。u 不存在 ⇒ 恒关（41课保守侧）。
    pub fn gate_open(&self, k: usize, entry_ladder: usize) -> bool {
        match self.u_of(k, entry_ladder) {
            None => false,
            Some(u) => matches!(self.state[u], FatigueState::Fatigued { .. }),
        }
    }

    /// 门开率可观测义务（C7 步骤5-iv）：本 bar Fatigued 的层位掩码。
    pub fn fatigued_mask(&self) -> u16 {
        let mut m = 0u16;
        for (u, st) in self.state.iter().enumerate() {
            if matches!(st, FatigueState::Fatigued { .. }) {
                m |= 1 << u;
            }
        }
        m
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::divergence::DivKind;
    use crate::stroke::Direction;

    fn sell1_cand() -> BspEvent {
        BspEvent {
            class: BspClass::Sell1,
            seg_idx: 0,
            confirmed: false,
            cs: None,
            zd: None,
            zg: None,
            price: 10.0,
        }
    }

    fn div_sell() -> DivEvent {
        DivEvent {
            kind: DivKind::Consolidation,
            direction: Direction::Up,
            seg_idx: 0,
            force_a: 2.0,
            force_c: 1.0,
            price: 10.0,
        }
    }

    fn div_sell_strong() -> DivEvent {
        DivEvent {
            kind: DivKind::Consolidation,
            direction: Direction::Up,
            seg_idx: 0,
            force_a: 1.0,
            force_c: 2.0,
            price: 10.0,
        }
    }

    #[test]
    fn path1_weak_rush_high_does_not_clear() {
        let mut g = FatigueGate::new();
        // u=3 出现 candidate type1 卖 → Fatigued（high_water=10.0）
        g.observe(3, &[sell1_cand()], &[], &[], false, 9.8, 10.0);
        assert!(g.gate_open(2, 3));
        // 创新高但力度背驰（force_c < force_a）= 弱冲高 → 不清（#1232 裁定 a）
        g.observe(3, &[], &[div_sell()], &[], false, 10.5, 10.5);
        assert!(g.gate_open(2, 3));
    }

    #[test]
    fn path1_strong_rush_high_clears() {
        let mut g = FatigueGate::new();
        g.observe(3, &[sell1_cand()], &[], &[], false, 9.8, 10.0);
        assert!(g.gate_open(2, 3));
        // 强力不背驰创新高（force_c >= force_a）→ Fresh（路径1；43:194）
        g.observe(3, &[], &[div_sell_strong()], &[], false, 10.5, 10.5);
        assert!(!g.gate_open(2, 3));
    }

    #[test]
    fn path1_pure_price_new_high_without_force_source_does_not_clear() {
        let mut g = FatigueGate::new();
        g.observe(3, &[sell1_cand()], &[], &[], false, 9.8, 10.0);
        assert!(g.gate_open(2, 3));
        // 纯价格创新高，无卖侧 div（无力度源）→ 无源不判，不清（照 #985）
        g.observe(3, &[], &[], &[], false, 10.5, 10.5);
        assert!(g.gate_open(2, 3));
    }

    #[test]
    fn clear_by_center_formed_and_up_settle() {
        let mut g = FatigueGate::new();
        g.observe(3, &[], &[div_sell()], &[], false, 9.0, 10.0);
        assert!(g.gate_open(2, 3));
        // 路径2：新中枢
        let formed = CenterEvent::Formed {
            seg_start: 7,
            zd: 1.0,
            zg: 2.0,
        };
        g.observe(3, &[], &[], &[formed], false, 9.0, 10.0);
        assert!(!g.gate_open(2, 3));
        // 重新进入，再用路径3清空
        g.observe(3, &[], &[div_sell()], &[], false, 9.0, 10.0);
        assert!(g.gate_open(2, 3));
        g.observe(3, &[], &[], &[], true, 9.0, 10.0);
        assert!(!g.gate_open(2, 3));
    }

    #[test]
    fn clear_precedes_add_same_bar() {
        let mut g = FatigueGate::new();
        g.observe(3, &[], &[div_sell()], &[], false, 9.0, 10.0);
        // 同 bar：up_settle 清空 + 新证据 → 新证据保留（事件序纪律）
        g.observe(3, &[sell1_cand()], &[], &[], true, 9.0, 10.0);
        assert!(g.gate_open(2, 3));
    }

    #[test]
    fn u_of_requires_structure_seen() {
        let g = FatigueGate::new();
        assert_eq!(g.u_of(2, 5), None);
        assert!(!g.gate_open(2, 5)); // u 不存在 ⇒ 恒关
    }

    #[test]
    fn fatigued_state_keeps_entry_high_water() {
        let mut g = FatigueGate::new();
        g.observe(3, &[sell1_cand()], &[], &[], false, 9.0, 10.0);
        // 已 Fatigued 时新证据不重置 high_water（进入时刻快照）
        g.observe(3, &[sell1_cand()], &[], &[], false, 9.5, 12.0);
        // c=11 > 进入时 high_water=10 且强力不背驰 → 清空（若被 12.0 覆盖则不会清）
        g.observe(3, &[], &[div_sell_strong()], &[], false, 11.0, 12.0);
        assert!(!g.gate_open(2, 3));
    }
}
