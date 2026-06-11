//! DepthRef — 锚中枢相对振幅的因果滚动参照（θ 自适应深度门，2026-06-11 任务）。
//!
//! 动机：固定 θ=1% 在 OKLO/BRN（中枢振幅大）工作，QQQ 的 1 分钟中枢振幅
//! 几乎全 <1% ⇒ 1065 对震荡型配对被门后只剩 3 对——量级失配，非策略失败。
//!
//! 严格性裁定（方案 A/B/C 对比见 analysis/theta_adaptive_results.md §2）：
//!   θ_t(k) = 该层**形成时刻早于当前 bar** 的最近 window 个中枢相对振幅
//!            (ZG−ZD)/close_at_observation 的 q 分位（nearest-rank），
//!            **排除当前锚自身**（门槛决策不得是被检对象自身的函数）。
//!   样本 < min_obs ⇒ 回退 cfg.theta_depth 固定值（warm-up 期保持基线门，
//!   可观测：counters.n_rev_theta_fallbacks）。
//!
//! 因果性：观测点 = CenterBook.last 边界变化 bar（中枢形成/延伸的事件时刻），
//! 参照集只含已观测中枢——零前瞻。q/window/min_obs 预注册为变体常数，
//! 不做标的级拟合（方案 C 的全样本分位数 = 回测期内前瞻，被否定）。

use std::collections::{HashMap, VecDeque};

use super::center_book::CenterBook;
use super::types::{FIRST_BSP_LADDER, MAX_LADDER};

/// 振幅观测窗容量（θ 自适应变体的预注册常数；Fixed 模式下仅供调研日志）。
pub const DEPTH_REF_WINDOW: usize = 50;

#[derive(Debug)]
pub struct DepthRef {
    /// ladder → 最近 window 个 (中枢 seg_start, 相对振幅) 环形缓冲。
    bufs: [VecDeque<(i64, f64)>; MAX_LADDER],
    /// ladder → 已观测的 (seg_start, zd, zg)——边界变化检测（延伸原位更新）。
    last_seen: [Option<(i64, f64, f64)>; MAX_LADDER],
    window: usize,
    /// 调研日志：每中枢一行 (ladder, seg_start, 最终观测振幅)，延伸原位更新。
    log: Vec<(u8, i64, f64)>,
    log_idx: HashMap<(u8, i64), usize>,
}

impl DepthRef {
    pub fn new(window: usize) -> Self {
        DepthRef {
            bufs: std::array::from_fn(|_| VecDeque::with_capacity(window + 1)),
            last_seen: [None; MAX_LADDER],
            window,
            log: Vec::new(),
            log_idx: HashMap::new(),
        }
    }

    /// 每 bar（事件 bar）观测：CenterBook.last 边界变化 ⇒ 记录/更新该中枢的
    /// 相对振幅 (ZG−ZD)/c。NaN 边界（CenterBook 的 fail-fast 哨兵）不入参照集。
    pub fn observe(&mut self, book: &CenterBook, c: f64) {
        for lad in FIRST_BSP_LADDER..MAX_LADDER {
            let Some(lc) = book.last(lad) else { continue };
            let key = (lc.seg_start, lc.zd, lc.zg);
            if self.last_seen[lad] == Some(key) {
                continue;
            }
            self.last_seen[lad] = Some(key);
            if !(lc.zd.is_finite() && lc.zg.is_finite()) || c <= 0.0 {
                continue;
            }
            let rel = (lc.zg - lc.zd) / c;
            if !rel.is_finite() {
                continue;
            }
            let buf = &mut self.bufs[lad];
            match buf.back_mut() {
                // 同中枢延伸：原位更新（一个中枢一个参照条目）
                Some(back) if back.0 == lc.seg_start => back.1 = rel,
                _ => {
                    buf.push_back((lc.seg_start, rel));
                    if buf.len() > self.window {
                        buf.pop_front();
                    }
                }
            }
            match self.log_idx.entry((lad as u8, lc.seg_start)) {
                std::collections::hash_map::Entry::Occupied(e) => {
                    self.log[*e.get()].2 = rel;
                }
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert(self.log.len());
                    self.log.push((lad as u8, lc.seg_start, rel));
                }
            }
        }
    }

    /// 该层当前 θ：参照集（排除 exclude_cs）的 q 分位（nearest-rank）。
    /// 样本 < min_obs ⇒ None（调用方回退固定 θ）。
    pub fn theta(
        &self,
        ladder: usize,
        exclude_cs: Option<i64>,
        q: f64,
        min_obs: usize,
    ) -> Option<f64> {
        let mut amps: Vec<f64> = self.bufs[ladder]
            .iter()
            .filter(|(cs, _)| Some(*cs) != exclude_cs)
            .map(|(_, a)| *a)
            .collect();
        if amps.len() < min_obs.max(1) {
            return None;
        }
        amps.sort_by(|a, b| a.partial_cmp(b).expect("参照集振幅恒有限"));
        // nearest-rank：ceil(q·n) 个最小值（q∈(0,1]；q=0 取最小值）
        let n = amps.len();
        let rank = ((q * n as f64).ceil() as usize).clamp(1, n);
        Some(amps[rank - 1])
    }

    /// 调研日志（每中枢一行最终振幅）——lib.rs diag marshal 消费。
    pub fn take_log(&mut self) -> Vec<(u8, i64, f64)> {
        self.log_idx.clear();
        std::mem::take(&mut self.log)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trading::types::{BspClass, BspEvent};

    fn ev(cs: i64, zd: f64, zg: f64) -> BspEvent {
        BspEvent {
            class: BspClass::Sell1,
            seg_idx: 0,
            confirmed: true,
            cs: Some(cs),
            zd: Some(zd),
            zg: Some(zg),
            price: 0.0,
        }
    }

    fn feed(dr: &mut DepthRef, book: &mut CenterBook, cs: i64, zd: f64, zg: f64, c: f64) {
        book.ingest(2, &[ev(cs, zd, zg)], true, None);
        dr.observe(book, c);
    }

    #[test]
    fn quantile_nearest_rank_and_exclusion() {
        let mut dr = DepthRef::new(50);
        let mut book = CenterBook::new();
        // 振幅 1%,2%,3%,4%（c=100）
        for (i, amp) in [1.0, 2.0, 3.0, 4.0].iter().enumerate() {
            feed(&mut dr, &mut book, 10 + i as i64, 50.0, 50.0 + amp, 100.0);
        }
        // P50 of {0.01,0.02,0.03,0.04} = 第2小 = 0.02
        assert_eq!(dr.theta(2, None, 0.5, 1), Some(0.02));
        // 排除 cs=13（amp=4%）→ {1,2,3}% 的 P50 = 0.02
        assert_eq!(dr.theta(2, Some(13), 0.5, 1), Some(0.02));
        // 排除后样本 3 < min_obs 4 → None（回退信号）
        assert_eq!(dr.theta(2, Some(13), 0.5, 4), None);
        // P25 = 第1小 = 0.01
        assert_eq!(dr.theta(2, None, 0.25, 1), Some(0.01));
    }

    #[test]
    fn extension_updates_in_place_window_evicts() {
        let mut dr = DepthRef::new(2);
        let mut book = CenterBook::new();
        feed(&mut dr, &mut book, 10, 50.0, 51.0, 100.0); // 1%
        // 同 cs 延伸：边界更新 → 原位改写，参照集仍 1 条
        feed(&mut dr, &mut book, 10, 50.0, 52.0, 100.0); // → 2%
        assert_eq!(dr.theta(2, None, 0.5, 1), Some(0.02));
        feed(&mut dr, &mut book, 20, 50.0, 53.0, 100.0); // 3%
        feed(&mut dr, &mut book, 30, 50.0, 54.0, 100.0); // 4% → 窗容量2，cs=10 逐出
        assert_eq!(dr.theta(2, None, 1.0, 1), Some(0.04));
        assert_eq!(dr.theta(2, None, 0.0, 1), Some(0.03)); // cs=10 已不在
        // 日志保留全部中枢（调研面不受窗限制），延伸原位更新
        let log = dr.take_log();
        assert_eq!(log.len(), 3);
        assert_eq!(log[0], (2, 10, 0.02));
    }

    #[test]
    fn nan_boundary_skipped() {
        let mut dr = DepthRef::new(50);
        let mut book = CenterBook::new();
        book.ingest(
            2,
            &[BspEvent {
                class: BspClass::Sell1,
                seg_idx: 0,
                confirmed: true,
                cs: Some(10),
                zd: None,
                zg: None,
                price: 0.0,
            }],
            true,
            None,
        );
        dr.observe(&book, 100.0);
        assert_eq!(dr.theta(2, None, 0.5, 1), None);
        assert!(dr.take_log().is_empty());
    }
}
