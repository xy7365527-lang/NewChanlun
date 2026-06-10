//! CenterBook — 中枢生命周期账本（市场性质，跨 trade 持续）。
//!
//! `ingest` 逐字移植 Python `run_organic` 主循环的中枢账本段（last_center /
//! dead_centers / frozen / center_version，含 not-in 守卫的版本号语义）。
//!
//! v2 增量（C6/D2）：ingest 同时在**唯一 diff 点**派生 `CenterEvent` 三态流——
//! 多消费者各自 diff 是分歧温床（C6 候选 D 的否定论证）。数据源边界声明见
//! `types::CenterEvent` docstring（BSP 事件锚派生，非信号层 zhongshus diff）。

use std::collections::HashSet;

use super::types::*;
use crate::buysellpoint::{BspKind, Side};
use crate::stroke::Direction;

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
    /// ladder → 冻结中枢 seg_start（Python `frozen`；hard_type3 buy 置位）。
    frozen: [Option<i64>; MAX_LADDER],
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
            let dead = self.dead[ladder].get_or_insert_with(HashSet::new);
            if ev.confirmed && ev.class.kind() == BspKind::Type3 {
                if !dead.contains(&cs) {
                    dead.insert(cs);
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
            }
        }
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

    pub fn is_frozen(&self, ladder: usize) -> bool {
        self.frozen[ladder].is_some()
    }
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
}
