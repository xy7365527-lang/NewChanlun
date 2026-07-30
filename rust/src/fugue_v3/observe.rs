//! **groupoid 观察轴桥接**（ObserveBridge impl ObserveAxis）：包装向心 confirm + 区间套定位。
//!
//! GUARD-ROLE: t-engine-accounting-basis——名分：现役（详见 `fugue_v3/mod.rs`
//! 头部 GUARD-ROLE 块，#762 C7-E3 核定）。
//!
//! 用户裁决（2026-06-17）：把现有向心回溯（`helix_centripetal_confirm`，T49）包装成 ObserveAxis
//! trait。把信号层 confirm/located 产出暴露为接口——operate 不直接 reach into
//! SignalState.located_sell/located_buy。
//!
//! ## confirm 读法矛盾（诚实标注，no-patch）
//! 向心"读过去 vs 当下合取"读法矛盾**仍 open**（escalation 2026-06-15-confirm-arming-differance）。
//! 本桥接**原样继承**信号层"向心读过去"实现，不引入也不解决该矛盾（声明=能力）。
//!
//! ## 认识论等级
//! 全文件 **L1**（桥接 view，直接转发信号层 confirm/located 产出）。

use crate::spiral::signal::{GroupEventFrame, PendingLocate, SignalState};
use crate::trading::types::MAX_LADDER;

use super::axis::ObserveAxis;

/// groupoid 观察轴桥接（轻量 view，持引用零拷贝）。
pub struct ObserveBridge<'a> {
    signal: &'a SignalState,
    frame: &'a GroupEventFrame,
}

impl<'a> ObserveBridge<'a> {
    pub fn new(signal: &'a SignalState, frame: &'a GroupEventFrame) -> Self {
        ObserveBridge { signal, frame }
    }
}

impl ObserveAxis for ObserveBridge<'_> {
    fn nf_sell(&self, ladder: usize) -> Option<f64> {
        self.frame.nf_sell[ladder]
    }

    fn nf_buy(&self, ladder: usize) -> Option<f64> {
        self.frame.nf_buy[ladder]
    }

    fn sell_source(&self) -> Option<usize> {
        self.frame.sell_source
    }

    fn buy_source(&self) -> Option<usize> {
        self.frame.buy_source
    }

    fn located_sell_chain(&self) -> &[Option<PendingLocate>; MAX_LADDER] {
        &self.signal.located_sell
    }

    fn located_buy_chain(&self) -> &[Option<PendingLocate>; MAX_LADDER] {
        &self.signal.located_buy
    }
}
