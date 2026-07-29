//! 买卖点身份账本 S1 测试群（票 #621）。
//!
//! 接缝（spec 测试决策：一本账一条缝，不测内部实现细节）：
//!
//! - [`state_machine`]：账本「观察入 → 事件/修订出」边界（表驱动全盖）；
//! - [`clocks`]：三钟纪律（首写不改 / 倒退拒收 + 警报 / 知情时 ≠ 位置）；
//! - [`log_replay`]：日志重放一致性（`fold(log)=state`、prefix 一致、缓存溯源校验、JSONL 边界）；
//! - [`portal`]：成立档只读面（证据包齐、死亡证明载荷齐）；
//! - [`standby`]（票 #623）：备战档只读面（未决可枚举 + 类型面隔离）；
//! - [`short_retrace`]（票 #623）：短差档只读面（亚型签固定 + 三锁 + 失败处置通知）；
//! - [`rebase`]（票 #622）：引擎改口处死（窗口变 / 中枢消失 / 同窗口重确认零动作，`observe`
//!   碰撞路径 + `reconcile_window` 显式核对路径）；
//! - [`dead_center`]（票 #622）：死人挂号拒收 vs 同身份迟到静默吸收（两路不混）；
//! - [`two_pass_evidence`]（票 #622）：注册拍 / 落锤拍证据一致性守卫（影子评审 #621 MEDIUM-2）；
//! - [`rebase_audit`]（票 #622）：四类警报 audit 流（落流 + append-only + 不进折叠 + 篡改隔离）；
//! - [`recovery_guard`]（票 #632）：恢复后未决身份落锤显式护栏（影子评审 #621 MEDIUM-1；
//!   陈旧知情时拒收 + 边界 + 终态身份不受影响 + 活账零约束）；
//! - [`replay_parity`]（票 #624）：旧模块 `first_retrace_replay` fixtures 的行为等价面对拍
//!   （严格 pair 映射 / 消费一次性 / Supersede→Restart 纪律 / 两错 fail-loud 的对应面）；
//! - [`golden_log`]（票 #624）：事件日志 golden 锚（JSONL 字节 + 指纹 + 重放一致，六场景覆盖）。

use super::super::super::types::{Direction, Tick};
use super::super::level_view::CoordinateWindow;
use super::*;

mod clocks;
mod dead_center;
mod golden_log;
mod log_replay;
mod portal;
mod rebase;
mod rebase_audit;
mod recovery_guard;
mod replay_parity;
mod short_retrace;
mod standby;
mod state_machine;
mod two_pass_evidence;

/// 合成中枢核心区间下沿。
const ZD: Tick = 100;
/// 合成中枢核心区间上沿。
const ZG: Tick = 200;
/// 合成中枢起时间边（= 路由锚）。
const ANCHOR_START: usize = 1_000;

/// 四条边快照（起时间边固定 ⟹ 同一中枢锚）。
fn frame(end_index: usize) -> CenterFrame {
    CenterFrame {
        zd: ZD,
        zg: ZG,
        start_index: ANCHOR_START,
        end_index,
    }
}

/// 另一个中枢（不同起时间边 ⟹ 不同锚）。
fn other_frame(end_index: usize) -> CenterFrame {
    CenterFrame {
        zd: ZD,
        zg: ZG,
        start_index: ANCHOR_START + 5_000,
        end_index: ANCHOR_START + 5_000 + end_index,
    }
}

fn provenance() -> RetraceProvenance {
    RetraceProvenance {
        level: 3,
        window: CoordinateWindow {
            start: 0,
            end: 9_999,
        },
        data_basis: "synthetic-s1".to_owned(),
    }
}

fn ledger() -> RetraceLedger {
    RetraceLedger::new(provenance())
}

/// 一条合法的「向上离开」原始观察（三类买点候选）。
///
/// `outcome = None` ⟹ 未决（回抽位置诚实缺席）；`Some(_)` ⟹ 判案材料齐。
fn up_input(
    center: CenterFrame,
    departure: usize,
    outcome: Option<RetraceOutcome>,
    as_of: usize,
) -> RetraceInput {
    RetraceInput {
        center,
        pair: StrictCompletedPair {
            leave_move_index: departure,
            retest_move_index: departure + 1,
        },
        leave_direction: Direction::Up,
        retest_direction: Direction::Down,
        leave_end: RetracePoint {
            index: center.end_index + 10,
            price: ZG + 50,
        },
        retest_end: outcome.map(|_| RetracePoint {
            index: center.end_index + 20,
            price: ZG + 10,
        }),
        outcome,
        as_of,
    }
}

/// 「向下离开」原始观察（三类卖点候选）。
fn down_input(
    center: CenterFrame,
    departure: usize,
    outcome: Option<RetraceOutcome>,
    as_of: usize,
) -> RetraceInput {
    RetraceInput {
        center,
        pair: StrictCompletedPair {
            leave_move_index: departure,
            retest_move_index: departure + 1,
        },
        leave_direction: Direction::Down,
        retest_direction: Direction::Up,
        leave_end: RetracePoint {
            index: center.end_index + 10,
            price: ZD - 50,
        },
        retest_end: outcome.map(|_| RetracePoint {
            index: center.end_index + 20,
            price: ZD - 10,
        }),
        outcome,
        as_of,
    }
}

fn key_of(center: CenterFrame, departure: usize) -> RetraceKey {
    RetraceKey {
        frame: center,
        departure_move_index: departure,
    }
}

/// 收口断言：任何测试末尾都跑一遍全量不变量（含 `fold(journal) == book`）。
fn settled(ledger: &RetraceLedger) -> &RetraceLedger {
    ledger.assert_invariants();
    ledger
}

fn kinds(entry: &RetraceEntry) -> Vec<RetraceRevisionKind> {
    entry.revisions.iter().map(|revision| revision.kind).collect()
}
