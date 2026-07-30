//! 预注册样本切分窗口值 v0（冻结）——`docs/backtest-prereg-windows-v0.md` 的机器可读副本。
//!
//! ## 认识论：纯技术性产出（L1 基础设施，零信息增量）
//!
//! 本文件把 backtest-protocol-v0.md §2 的切分规则具体化为 8 品种精确日期窗口常量。
//! 切分在**看任何 L2 回测结果之前**冻结（防数据挖掘，§0/§2.3）——冻结时刻 2026-06-26，
//! 数据起止已知（cc-backtest #81 核实）、零 L2 结果产出。
//!
//! ## 与文档的一致性约束
//!
//! 本文件与 `docs/backtest-prereg-windows-v0.md` **必须逐字一致**。任一漂移 = 切分被改 =
//! 走 backtest-protocol §9 change-request。`prereg_consistency_*` 测试锁定结构不变量。
//!
//! ## 三个预注册裁定（文档 §3.0 / §4，看结果前冻结，供质询）
//!
//! - **W1**：walk-forward test 窗起点 ≤ OOS 末（2025-06-30），test 末越界则截到 OOS 末
//!   （`clipped=true`）——不规划触碰 Holdout 的窗。clipped 窗样本不足，报告但不计入时间
//!   稳定性主判据。
//! - **W2**：anchored train 扩张窗后期含 OOS 前半历史——anchored 固有性质（§2.2），Θ v0
//!   无参数故不产生拟合自由度，非泄漏。
//! - **OK1**：OKLO §2.4 特例 OOS（2025-01-01→数据末）跨越全局 Holdout 边界——忠实 §2.4
//!   字面（OKLO 不进主判据/探索性），张力显式标注，改判走 §9。

/// 品种池角色（backtest-protocol §1.3）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pool {
    /// 核心池（L2+L3 主力）：BTC/ES/CL/GC。
    Core,
    /// 扩展池（L3 鲁棒性）：BRN/DX/QQQ。
    Extended,
    /// 观察池（不进主判据）：OKLO。
    Observation,
}

/// 单个 walk-forward 窗（train+test，§2.2）。日期为 ISO `"YYYY-MM-DD"`（闭区间，含端点）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WfWindow {
    /// 窗序号（0-based，时间升序）。
    pub i: u32,
    pub train_start: &'static str,
    pub train_end: &'static str,
    pub test_start: &'static str,
    pub test_end: &'static str,
    /// test 末是否被截到 OOS 末（裁定 W1）——clipped 窗样本不足，不计入主判据。
    pub clipped: bool,
}

/// 单品种的完整预注册窗口集（§2.1 全局切分 + §2.2 walk-forward 双模式）。
#[derive(Debug, Clone, Copy)]
pub struct SymbolWindows {
    pub symbol: &'static str,
    pub pool: Pool,
    pub data_start: &'static str,
    pub data_end: &'static str,
    /// IS 窗 `(start, end)`；OKLO 无 IS（数据起晚于 IS 末）⟹ `None`。
    pub is_window: Option<(&'static str, &'static str)>,
    /// OOS 窗 `(start, end)`。核心/扩展池统一 `(2023-01-01, 2025-06-30)`；OKLO §2.4 特例。
    pub oos: (&'static str, &'static str),
    /// Holdout 窗 `(start, end)`；OKLO 特例 OOS 已含数据末（OK1 张力）⟹ `None`。
    pub holdout: Option<(&'static str, &'static str)>,
    /// walk-forward anchored 模式（train 起点固定 = 数据起，扩张窗）。OKLO 空。
    pub wf_anchored: &'static [WfWindow],
    /// walk-forward rolling 模式（train 固定 24 月滚动）。OKLO 空。
    pub wf_rolling: &'static [WfWindow],
}

/// 8 品种预注册窗口（冻结，顺序 = 核心 BTC/ES/CL/GC → 扩展 BRN/DX/QQQ → 观察 OKLO）。
pub const PREREG_WINDOWS: &[SymbolWindows] = &[
    SymbolWindows {
        symbol: "BTC",
        pool: Pool::Core,
        data_start: "2017-08-17",
        data_end: "2026-05-31",
        is_window: Some(("2017-08-17", "2022-12-31")),
        oos: ("2023-01-01", "2025-06-30"),
        holdout: Some(("2025-07-01", "2026-05-31")),
        wf_anchored: &[
            WfWindow {
                i: 0,
                train_start: "2017-08-17",
                train_end: "2019-08-16",
                test_start: "2019-08-17",
                test_end: "2020-02-16",
                clipped: false,
            },
            WfWindow {
                i: 1,
                train_start: "2017-08-17",
                train_end: "2020-02-16",
                test_start: "2020-02-17",
                test_end: "2020-08-16",
                clipped: false,
            },
            WfWindow {
                i: 2,
                train_start: "2017-08-17",
                train_end: "2020-08-16",
                test_start: "2020-08-17",
                test_end: "2021-02-16",
                clipped: false,
            },
            WfWindow {
                i: 3,
                train_start: "2017-08-17",
                train_end: "2021-02-16",
                test_start: "2021-02-17",
                test_end: "2021-08-16",
                clipped: false,
            },
            WfWindow {
                i: 4,
                train_start: "2017-08-17",
                train_end: "2021-08-16",
                test_start: "2021-08-17",
                test_end: "2022-02-16",
                clipped: false,
            },
            WfWindow {
                i: 5,
                train_start: "2017-08-17",
                train_end: "2022-02-16",
                test_start: "2022-02-17",
                test_end: "2022-08-16",
                clipped: false,
            },
            WfWindow {
                i: 6,
                train_start: "2017-08-17",
                train_end: "2022-08-16",
                test_start: "2022-08-17",
                test_end: "2023-02-16",
                clipped: false,
            },
            WfWindow {
                i: 7,
                train_start: "2017-08-17",
                train_end: "2023-02-16",
                test_start: "2023-02-17",
                test_end: "2023-08-16",
                clipped: false,
            },
            WfWindow {
                i: 8,
                train_start: "2017-08-17",
                train_end: "2023-08-16",
                test_start: "2023-08-17",
                test_end: "2024-02-16",
                clipped: false,
            },
            WfWindow {
                i: 9,
                train_start: "2017-08-17",
                train_end: "2024-02-16",
                test_start: "2024-02-17",
                test_end: "2024-08-16",
                clipped: false,
            },
            WfWindow {
                i: 10,
                train_start: "2017-08-17",
                train_end: "2024-08-16",
                test_start: "2024-08-17",
                test_end: "2025-02-16",
                clipped: false,
            },
            WfWindow {
                i: 11,
                train_start: "2017-08-17",
                train_end: "2025-02-16",
                test_start: "2025-02-17",
                test_end: "2025-06-30",
                clipped: true,
            },
        ],
        wf_rolling: &[
            WfWindow {
                i: 0,
                train_start: "2017-08-17",
                train_end: "2019-08-16",
                test_start: "2019-08-17",
                test_end: "2020-02-16",
                clipped: false,
            },
            WfWindow {
                i: 1,
                train_start: "2018-02-17",
                train_end: "2020-02-16",
                test_start: "2020-02-17",
                test_end: "2020-08-16",
                clipped: false,
            },
            WfWindow {
                i: 2,
                train_start: "2018-08-17",
                train_end: "2020-08-16",
                test_start: "2020-08-17",
                test_end: "2021-02-16",
                clipped: false,
            },
            WfWindow {
                i: 3,
                train_start: "2019-02-17",
                train_end: "2021-02-16",
                test_start: "2021-02-17",
                test_end: "2021-08-16",
                clipped: false,
            },
            WfWindow {
                i: 4,
                train_start: "2019-08-17",
                train_end: "2021-08-16",
                test_start: "2021-08-17",
                test_end: "2022-02-16",
                clipped: false,
            },
            WfWindow {
                i: 5,
                train_start: "2020-02-17",
                train_end: "2022-02-16",
                test_start: "2022-02-17",
                test_end: "2022-08-16",
                clipped: false,
            },
            WfWindow {
                i: 6,
                train_start: "2020-08-17",
                train_end: "2022-08-16",
                test_start: "2022-08-17",
                test_end: "2023-02-16",
                clipped: false,
            },
            WfWindow {
                i: 7,
                train_start: "2021-02-17",
                train_end: "2023-02-16",
                test_start: "2023-02-17",
                test_end: "2023-08-16",
                clipped: false,
            },
            WfWindow {
                i: 8,
                train_start: "2021-08-17",
                train_end: "2023-08-16",
                test_start: "2023-08-17",
                test_end: "2024-02-16",
                clipped: false,
            },
            WfWindow {
                i: 9,
                train_start: "2022-02-17",
                train_end: "2024-02-16",
                test_start: "2024-02-17",
                test_end: "2024-08-16",
                clipped: false,
            },
            WfWindow {
                i: 10,
                train_start: "2022-08-17",
                train_end: "2024-08-16",
                test_start: "2024-08-17",
                test_end: "2025-02-16",
                clipped: false,
            },
            WfWindow {
                i: 11,
                train_start: "2023-02-17",
                train_end: "2025-02-16",
                test_start: "2025-02-17",
                test_end: "2025-06-30",
                clipped: true,
            },
        ],
    },
    SymbolWindows {
        symbol: "ES",
        pool: Pool::Core,
        data_start: "2016-01-03",
        data_end: "2026-06-24",
        is_window: Some(("2016-01-03", "2022-12-31")),
        oos: ("2023-01-01", "2025-06-30"),
        holdout: Some(("2025-07-01", "2026-06-24")),
        wf_anchored: &[
            WfWindow {
                i: 0,
                train_start: "2016-01-03",
                train_end: "2018-01-02",
                test_start: "2018-01-03",
                test_end: "2018-07-02",
                clipped: false,
            },
            WfWindow {
                i: 1,
                train_start: "2016-01-03",
                train_end: "2018-07-02",
                test_start: "2018-07-03",
                test_end: "2019-01-02",
                clipped: false,
            },
            WfWindow {
                i: 2,
                train_start: "2016-01-03",
                train_end: "2019-01-02",
                test_start: "2019-01-03",
                test_end: "2019-07-02",
                clipped: false,
            },
            WfWindow {
                i: 3,
                train_start: "2016-01-03",
                train_end: "2019-07-02",
                test_start: "2019-07-03",
                test_end: "2020-01-02",
                clipped: false,
            },
            WfWindow {
                i: 4,
                train_start: "2016-01-03",
                train_end: "2020-01-02",
                test_start: "2020-01-03",
                test_end: "2020-07-02",
                clipped: false,
            },
            WfWindow {
                i: 5,
                train_start: "2016-01-03",
                train_end: "2020-07-02",
                test_start: "2020-07-03",
                test_end: "2021-01-02",
                clipped: false,
            },
            WfWindow {
                i: 6,
                train_start: "2016-01-03",
                train_end: "2021-01-02",
                test_start: "2021-01-03",
                test_end: "2021-07-02",
                clipped: false,
            },
            WfWindow {
                i: 7,
                train_start: "2016-01-03",
                train_end: "2021-07-02",
                test_start: "2021-07-03",
                test_end: "2022-01-02",
                clipped: false,
            },
            WfWindow {
                i: 8,
                train_start: "2016-01-03",
                train_end: "2022-01-02",
                test_start: "2022-01-03",
                test_end: "2022-07-02",
                clipped: false,
            },
            WfWindow {
                i: 9,
                train_start: "2016-01-03",
                train_end: "2022-07-02",
                test_start: "2022-07-03",
                test_end: "2023-01-02",
                clipped: false,
            },
            WfWindow {
                i: 10,
                train_start: "2016-01-03",
                train_end: "2023-01-02",
                test_start: "2023-01-03",
                test_end: "2023-07-02",
                clipped: false,
            },
            WfWindow {
                i: 11,
                train_start: "2016-01-03",
                train_end: "2023-07-02",
                test_start: "2023-07-03",
                test_end: "2024-01-02",
                clipped: false,
            },
            WfWindow {
                i: 12,
                train_start: "2016-01-03",
                train_end: "2024-01-02",
                test_start: "2024-01-03",
                test_end: "2024-07-02",
                clipped: false,
            },
            WfWindow {
                i: 13,
                train_start: "2016-01-03",
                train_end: "2024-07-02",
                test_start: "2024-07-03",
                test_end: "2025-01-02",
                clipped: false,
            },
            WfWindow {
                i: 14,
                train_start: "2016-01-03",
                train_end: "2025-01-02",
                test_start: "2025-01-03",
                test_end: "2025-06-30",
                clipped: true,
            },
        ],
        wf_rolling: &[
            WfWindow {
                i: 0,
                train_start: "2016-01-03",
                train_end: "2018-01-02",
                test_start: "2018-01-03",
                test_end: "2018-07-02",
                clipped: false,
            },
            WfWindow {
                i: 1,
                train_start: "2016-07-03",
                train_end: "2018-07-02",
                test_start: "2018-07-03",
                test_end: "2019-01-02",
                clipped: false,
            },
            WfWindow {
                i: 2,
                train_start: "2017-01-03",
                train_end: "2019-01-02",
                test_start: "2019-01-03",
                test_end: "2019-07-02",
                clipped: false,
            },
            WfWindow {
                i: 3,
                train_start: "2017-07-03",
                train_end: "2019-07-02",
                test_start: "2019-07-03",
                test_end: "2020-01-02",
                clipped: false,
            },
            WfWindow {
                i: 4,
                train_start: "2018-01-03",
                train_end: "2020-01-02",
                test_start: "2020-01-03",
                test_end: "2020-07-02",
                clipped: false,
            },
            WfWindow {
                i: 5,
                train_start: "2018-07-03",
                train_end: "2020-07-02",
                test_start: "2020-07-03",
                test_end: "2021-01-02",
                clipped: false,
            },
            WfWindow {
                i: 6,
                train_start: "2019-01-03",
                train_end: "2021-01-02",
                test_start: "2021-01-03",
                test_end: "2021-07-02",
                clipped: false,
            },
            WfWindow {
                i: 7,
                train_start: "2019-07-03",
                train_end: "2021-07-02",
                test_start: "2021-07-03",
                test_end: "2022-01-02",
                clipped: false,
            },
            WfWindow {
                i: 8,
                train_start: "2020-01-03",
                train_end: "2022-01-02",
                test_start: "2022-01-03",
                test_end: "2022-07-02",
                clipped: false,
            },
            WfWindow {
                i: 9,
                train_start: "2020-07-03",
                train_end: "2022-07-02",
                test_start: "2022-07-03",
                test_end: "2023-01-02",
                clipped: false,
            },
            WfWindow {
                i: 10,
                train_start: "2021-01-03",
                train_end: "2023-01-02",
                test_start: "2023-01-03",
                test_end: "2023-07-02",
                clipped: false,
            },
            WfWindow {
                i: 11,
                train_start: "2021-07-03",
                train_end: "2023-07-02",
                test_start: "2023-07-03",
                test_end: "2024-01-02",
                clipped: false,
            },
            WfWindow {
                i: 12,
                train_start: "2022-01-03",
                train_end: "2024-01-02",
                test_start: "2024-01-03",
                test_end: "2024-07-02",
                clipped: false,
            },
            WfWindow {
                i: 13,
                train_start: "2022-07-03",
                train_end: "2024-07-02",
                test_start: "2024-07-03",
                test_end: "2025-01-02",
                clipped: false,
            },
            WfWindow {
                i: 14,
                train_start: "2023-01-03",
                train_end: "2025-01-02",
                test_start: "2025-01-03",
                test_end: "2025-06-30",
                clipped: true,
            },
        ],
    },
    SymbolWindows {
        symbol: "CL",
        pool: Pool::Core,
        data_start: "2016-01-03",
        data_end: "2026-06-24",
        is_window: Some(("2016-01-03", "2022-12-31")),
        oos: ("2023-01-01", "2025-06-30"),
        holdout: Some(("2025-07-01", "2026-06-24")),
        wf_anchored: &[
            WfWindow {
                i: 0,
                train_start: "2016-01-03",
                train_end: "2018-01-02",
                test_start: "2018-01-03",
                test_end: "2018-07-02",
                clipped: false,
            },
            WfWindow {
                i: 1,
                train_start: "2016-01-03",
                train_end: "2018-07-02",
                test_start: "2018-07-03",
                test_end: "2019-01-02",
                clipped: false,
            },
            WfWindow {
                i: 2,
                train_start: "2016-01-03",
                train_end: "2019-01-02",
                test_start: "2019-01-03",
                test_end: "2019-07-02",
                clipped: false,
            },
            WfWindow {
                i: 3,
                train_start: "2016-01-03",
                train_end: "2019-07-02",
                test_start: "2019-07-03",
                test_end: "2020-01-02",
                clipped: false,
            },
            WfWindow {
                i: 4,
                train_start: "2016-01-03",
                train_end: "2020-01-02",
                test_start: "2020-01-03",
                test_end: "2020-07-02",
                clipped: false,
            },
            WfWindow {
                i: 5,
                train_start: "2016-01-03",
                train_end: "2020-07-02",
                test_start: "2020-07-03",
                test_end: "2021-01-02",
                clipped: false,
            },
            WfWindow {
                i: 6,
                train_start: "2016-01-03",
                train_end: "2021-01-02",
                test_start: "2021-01-03",
                test_end: "2021-07-02",
                clipped: false,
            },
            WfWindow {
                i: 7,
                train_start: "2016-01-03",
                train_end: "2021-07-02",
                test_start: "2021-07-03",
                test_end: "2022-01-02",
                clipped: false,
            },
            WfWindow {
                i: 8,
                train_start: "2016-01-03",
                train_end: "2022-01-02",
                test_start: "2022-01-03",
                test_end: "2022-07-02",
                clipped: false,
            },
            WfWindow {
                i: 9,
                train_start: "2016-01-03",
                train_end: "2022-07-02",
                test_start: "2022-07-03",
                test_end: "2023-01-02",
                clipped: false,
            },
            WfWindow {
                i: 10,
                train_start: "2016-01-03",
                train_end: "2023-01-02",
                test_start: "2023-01-03",
                test_end: "2023-07-02",
                clipped: false,
            },
            WfWindow {
                i: 11,
                train_start: "2016-01-03",
                train_end: "2023-07-02",
                test_start: "2023-07-03",
                test_end: "2024-01-02",
                clipped: false,
            },
            WfWindow {
                i: 12,
                train_start: "2016-01-03",
                train_end: "2024-01-02",
                test_start: "2024-01-03",
                test_end: "2024-07-02",
                clipped: false,
            },
            WfWindow {
                i: 13,
                train_start: "2016-01-03",
                train_end: "2024-07-02",
                test_start: "2024-07-03",
                test_end: "2025-01-02",
                clipped: false,
            },
            WfWindow {
                i: 14,
                train_start: "2016-01-03",
                train_end: "2025-01-02",
                test_start: "2025-01-03",
                test_end: "2025-06-30",
                clipped: true,
            },
        ],
        wf_rolling: &[
            WfWindow {
                i: 0,
                train_start: "2016-01-03",
                train_end: "2018-01-02",
                test_start: "2018-01-03",
                test_end: "2018-07-02",
                clipped: false,
            },
            WfWindow {
                i: 1,
                train_start: "2016-07-03",
                train_end: "2018-07-02",
                test_start: "2018-07-03",
                test_end: "2019-01-02",
                clipped: false,
            },
            WfWindow {
                i: 2,
                train_start: "2017-01-03",
                train_end: "2019-01-02",
                test_start: "2019-01-03",
                test_end: "2019-07-02",
                clipped: false,
            },
            WfWindow {
                i: 3,
                train_start: "2017-07-03",
                train_end: "2019-07-02",
                test_start: "2019-07-03",
                test_end: "2020-01-02",
                clipped: false,
            },
            WfWindow {
                i: 4,
                train_start: "2018-01-03",
                train_end: "2020-01-02",
                test_start: "2020-01-03",
                test_end: "2020-07-02",
                clipped: false,
            },
            WfWindow {
                i: 5,
                train_start: "2018-07-03",
                train_end: "2020-07-02",
                test_start: "2020-07-03",
                test_end: "2021-01-02",
                clipped: false,
            },
            WfWindow {
                i: 6,
                train_start: "2019-01-03",
                train_end: "2021-01-02",
                test_start: "2021-01-03",
                test_end: "2021-07-02",
                clipped: false,
            },
            WfWindow {
                i: 7,
                train_start: "2019-07-03",
                train_end: "2021-07-02",
                test_start: "2021-07-03",
                test_end: "2022-01-02",
                clipped: false,
            },
            WfWindow {
                i: 8,
                train_start: "2020-01-03",
                train_end: "2022-01-02",
                test_start: "2022-01-03",
                test_end: "2022-07-02",
                clipped: false,
            },
            WfWindow {
                i: 9,
                train_start: "2020-07-03",
                train_end: "2022-07-02",
                test_start: "2022-07-03",
                test_end: "2023-01-02",
                clipped: false,
            },
            WfWindow {
                i: 10,
                train_start: "2021-01-03",
                train_end: "2023-01-02",
                test_start: "2023-01-03",
                test_end: "2023-07-02",
                clipped: false,
            },
            WfWindow {
                i: 11,
                train_start: "2021-07-03",
                train_end: "2023-07-02",
                test_start: "2023-07-03",
                test_end: "2024-01-02",
                clipped: false,
            },
            WfWindow {
                i: 12,
                train_start: "2022-01-03",
                train_end: "2024-01-02",
                test_start: "2024-01-03",
                test_end: "2024-07-02",
                clipped: false,
            },
            WfWindow {
                i: 13,
                train_start: "2022-07-03",
                train_end: "2024-07-02",
                test_start: "2024-07-03",
                test_end: "2025-01-02",
                clipped: false,
            },
            WfWindow {
                i: 14,
                train_start: "2023-01-03",
                train_end: "2025-01-02",
                test_start: "2025-01-03",
                test_end: "2025-06-30",
                clipped: true,
            },
        ],
    },
    SymbolWindows {
        symbol: "GC",
        pool: Pool::Core,
        data_start: "2016-01-03",
        data_end: "2026-06-24",
        is_window: Some(("2016-01-03", "2022-12-31")),
        oos: ("2023-01-01", "2025-06-30"),
        holdout: Some(("2025-07-01", "2026-06-24")),
        wf_anchored: &[
            WfWindow {
                i: 0,
                train_start: "2016-01-03",
                train_end: "2018-01-02",
                test_start: "2018-01-03",
                test_end: "2018-07-02",
                clipped: false,
            },
            WfWindow {
                i: 1,
                train_start: "2016-01-03",
                train_end: "2018-07-02",
                test_start: "2018-07-03",
                test_end: "2019-01-02",
                clipped: false,
            },
            WfWindow {
                i: 2,
                train_start: "2016-01-03",
                train_end: "2019-01-02",
                test_start: "2019-01-03",
                test_end: "2019-07-02",
                clipped: false,
            },
            WfWindow {
                i: 3,
                train_start: "2016-01-03",
                train_end: "2019-07-02",
                test_start: "2019-07-03",
                test_end: "2020-01-02",
                clipped: false,
            },
            WfWindow {
                i: 4,
                train_start: "2016-01-03",
                train_end: "2020-01-02",
                test_start: "2020-01-03",
                test_end: "2020-07-02",
                clipped: false,
            },
            WfWindow {
                i: 5,
                train_start: "2016-01-03",
                train_end: "2020-07-02",
                test_start: "2020-07-03",
                test_end: "2021-01-02",
                clipped: false,
            },
            WfWindow {
                i: 6,
                train_start: "2016-01-03",
                train_end: "2021-01-02",
                test_start: "2021-01-03",
                test_end: "2021-07-02",
                clipped: false,
            },
            WfWindow {
                i: 7,
                train_start: "2016-01-03",
                train_end: "2021-07-02",
                test_start: "2021-07-03",
                test_end: "2022-01-02",
                clipped: false,
            },
            WfWindow {
                i: 8,
                train_start: "2016-01-03",
                train_end: "2022-01-02",
                test_start: "2022-01-03",
                test_end: "2022-07-02",
                clipped: false,
            },
            WfWindow {
                i: 9,
                train_start: "2016-01-03",
                train_end: "2022-07-02",
                test_start: "2022-07-03",
                test_end: "2023-01-02",
                clipped: false,
            },
            WfWindow {
                i: 10,
                train_start: "2016-01-03",
                train_end: "2023-01-02",
                test_start: "2023-01-03",
                test_end: "2023-07-02",
                clipped: false,
            },
            WfWindow {
                i: 11,
                train_start: "2016-01-03",
                train_end: "2023-07-02",
                test_start: "2023-07-03",
                test_end: "2024-01-02",
                clipped: false,
            },
            WfWindow {
                i: 12,
                train_start: "2016-01-03",
                train_end: "2024-01-02",
                test_start: "2024-01-03",
                test_end: "2024-07-02",
                clipped: false,
            },
            WfWindow {
                i: 13,
                train_start: "2016-01-03",
                train_end: "2024-07-02",
                test_start: "2024-07-03",
                test_end: "2025-01-02",
                clipped: false,
            },
            WfWindow {
                i: 14,
                train_start: "2016-01-03",
                train_end: "2025-01-02",
                test_start: "2025-01-03",
                test_end: "2025-06-30",
                clipped: true,
            },
        ],
        wf_rolling: &[
            WfWindow {
                i: 0,
                train_start: "2016-01-03",
                train_end: "2018-01-02",
                test_start: "2018-01-03",
                test_end: "2018-07-02",
                clipped: false,
            },
            WfWindow {
                i: 1,
                train_start: "2016-07-03",
                train_end: "2018-07-02",
                test_start: "2018-07-03",
                test_end: "2019-01-02",
                clipped: false,
            },
            WfWindow {
                i: 2,
                train_start: "2017-01-03",
                train_end: "2019-01-02",
                test_start: "2019-01-03",
                test_end: "2019-07-02",
                clipped: false,
            },
            WfWindow {
                i: 3,
                train_start: "2017-07-03",
                train_end: "2019-07-02",
                test_start: "2019-07-03",
                test_end: "2020-01-02",
                clipped: false,
            },
            WfWindow {
                i: 4,
                train_start: "2018-01-03",
                train_end: "2020-01-02",
                test_start: "2020-01-03",
                test_end: "2020-07-02",
                clipped: false,
            },
            WfWindow {
                i: 5,
                train_start: "2018-07-03",
                train_end: "2020-07-02",
                test_start: "2020-07-03",
                test_end: "2021-01-02",
                clipped: false,
            },
            WfWindow {
                i: 6,
                train_start: "2019-01-03",
                train_end: "2021-01-02",
                test_start: "2021-01-03",
                test_end: "2021-07-02",
                clipped: false,
            },
            WfWindow {
                i: 7,
                train_start: "2019-07-03",
                train_end: "2021-07-02",
                test_start: "2021-07-03",
                test_end: "2022-01-02",
                clipped: false,
            },
            WfWindow {
                i: 8,
                train_start: "2020-01-03",
                train_end: "2022-01-02",
                test_start: "2022-01-03",
                test_end: "2022-07-02",
                clipped: false,
            },
            WfWindow {
                i: 9,
                train_start: "2020-07-03",
                train_end: "2022-07-02",
                test_start: "2022-07-03",
                test_end: "2023-01-02",
                clipped: false,
            },
            WfWindow {
                i: 10,
                train_start: "2021-01-03",
                train_end: "2023-01-02",
                test_start: "2023-01-03",
                test_end: "2023-07-02",
                clipped: false,
            },
            WfWindow {
                i: 11,
                train_start: "2021-07-03",
                train_end: "2023-07-02",
                test_start: "2023-07-03",
                test_end: "2024-01-02",
                clipped: false,
            },
            WfWindow {
                i: 12,
                train_start: "2022-01-03",
                train_end: "2024-01-02",
                test_start: "2024-01-03",
                test_end: "2024-07-02",
                clipped: false,
            },
            WfWindow {
                i: 13,
                train_start: "2022-07-03",
                train_end: "2024-07-02",
                test_start: "2024-07-03",
                test_end: "2025-01-02",
                clipped: false,
            },
            WfWindow {
                i: 14,
                train_start: "2023-01-03",
                train_end: "2025-01-02",
                test_start: "2025-01-03",
                test_end: "2025-06-30",
                clipped: true,
            },
        ],
    },
    SymbolWindows {
        symbol: "BRN",
        pool: Pool::Extended,
        data_start: "2018-12-26",
        data_end: "2026-06-24",
        is_window: Some(("2018-12-26", "2022-12-31")),
        oos: ("2023-01-01", "2025-06-30"),
        holdout: Some(("2025-07-01", "2026-06-24")),
        wf_anchored: &[
            WfWindow {
                i: 0,
                train_start: "2018-12-26",
                train_end: "2020-12-25",
                test_start: "2020-12-26",
                test_end: "2021-06-25",
                clipped: false,
            },
            WfWindow {
                i: 1,
                train_start: "2018-12-26",
                train_end: "2021-06-25",
                test_start: "2021-06-26",
                test_end: "2021-12-25",
                clipped: false,
            },
            WfWindow {
                i: 2,
                train_start: "2018-12-26",
                train_end: "2021-12-25",
                test_start: "2021-12-26",
                test_end: "2022-06-25",
                clipped: false,
            },
            WfWindow {
                i: 3,
                train_start: "2018-12-26",
                train_end: "2022-06-25",
                test_start: "2022-06-26",
                test_end: "2022-12-25",
                clipped: false,
            },
            WfWindow {
                i: 4,
                train_start: "2018-12-26",
                train_end: "2022-12-25",
                test_start: "2022-12-26",
                test_end: "2023-06-25",
                clipped: false,
            },
            WfWindow {
                i: 5,
                train_start: "2018-12-26",
                train_end: "2023-06-25",
                test_start: "2023-06-26",
                test_end: "2023-12-25",
                clipped: false,
            },
            WfWindow {
                i: 6,
                train_start: "2018-12-26",
                train_end: "2023-12-25",
                test_start: "2023-12-26",
                test_end: "2024-06-25",
                clipped: false,
            },
            WfWindow {
                i: 7,
                train_start: "2018-12-26",
                train_end: "2024-06-25",
                test_start: "2024-06-26",
                test_end: "2024-12-25",
                clipped: false,
            },
            WfWindow {
                i: 8,
                train_start: "2018-12-26",
                train_end: "2024-12-25",
                test_start: "2024-12-26",
                test_end: "2025-06-25",
                clipped: false,
            },
            WfWindow {
                i: 9,
                train_start: "2018-12-26",
                train_end: "2025-06-25",
                test_start: "2025-06-26",
                test_end: "2025-06-30",
                clipped: true,
            },
        ],
        wf_rolling: &[
            WfWindow {
                i: 0,
                train_start: "2018-12-26",
                train_end: "2020-12-25",
                test_start: "2020-12-26",
                test_end: "2021-06-25",
                clipped: false,
            },
            WfWindow {
                i: 1,
                train_start: "2019-06-26",
                train_end: "2021-06-25",
                test_start: "2021-06-26",
                test_end: "2021-12-25",
                clipped: false,
            },
            WfWindow {
                i: 2,
                train_start: "2019-12-26",
                train_end: "2021-12-25",
                test_start: "2021-12-26",
                test_end: "2022-06-25",
                clipped: false,
            },
            WfWindow {
                i: 3,
                train_start: "2020-06-26",
                train_end: "2022-06-25",
                test_start: "2022-06-26",
                test_end: "2022-12-25",
                clipped: false,
            },
            WfWindow {
                i: 4,
                train_start: "2020-12-26",
                train_end: "2022-12-25",
                test_start: "2022-12-26",
                test_end: "2023-06-25",
                clipped: false,
            },
            WfWindow {
                i: 5,
                train_start: "2021-06-26",
                train_end: "2023-06-25",
                test_start: "2023-06-26",
                test_end: "2023-12-25",
                clipped: false,
            },
            WfWindow {
                i: 6,
                train_start: "2021-12-26",
                train_end: "2023-12-25",
                test_start: "2023-12-26",
                test_end: "2024-06-25",
                clipped: false,
            },
            WfWindow {
                i: 7,
                train_start: "2022-06-26",
                train_end: "2024-06-25",
                test_start: "2024-06-26",
                test_end: "2024-12-25",
                clipped: false,
            },
            WfWindow {
                i: 8,
                train_start: "2022-12-26",
                train_end: "2024-12-25",
                test_start: "2024-12-26",
                test_end: "2025-06-25",
                clipped: false,
            },
            WfWindow {
                i: 9,
                train_start: "2023-06-26",
                train_end: "2025-06-25",
                test_start: "2025-06-26",
                test_end: "2025-06-30",
                clipped: true,
            },
        ],
    },
    SymbolWindows {
        symbol: "DX",
        pool: Pool::Extended,
        data_start: "2018-12-26",
        data_end: "2026-06-23",
        is_window: Some(("2018-12-26", "2022-12-31")),
        oos: ("2023-01-01", "2025-06-30"),
        holdout: Some(("2025-07-01", "2026-06-23")),
        wf_anchored: &[
            WfWindow {
                i: 0,
                train_start: "2018-12-26",
                train_end: "2020-12-25",
                test_start: "2020-12-26",
                test_end: "2021-06-25",
                clipped: false,
            },
            WfWindow {
                i: 1,
                train_start: "2018-12-26",
                train_end: "2021-06-25",
                test_start: "2021-06-26",
                test_end: "2021-12-25",
                clipped: false,
            },
            WfWindow {
                i: 2,
                train_start: "2018-12-26",
                train_end: "2021-12-25",
                test_start: "2021-12-26",
                test_end: "2022-06-25",
                clipped: false,
            },
            WfWindow {
                i: 3,
                train_start: "2018-12-26",
                train_end: "2022-06-25",
                test_start: "2022-06-26",
                test_end: "2022-12-25",
                clipped: false,
            },
            WfWindow {
                i: 4,
                train_start: "2018-12-26",
                train_end: "2022-12-25",
                test_start: "2022-12-26",
                test_end: "2023-06-25",
                clipped: false,
            },
            WfWindow {
                i: 5,
                train_start: "2018-12-26",
                train_end: "2023-06-25",
                test_start: "2023-06-26",
                test_end: "2023-12-25",
                clipped: false,
            },
            WfWindow {
                i: 6,
                train_start: "2018-12-26",
                train_end: "2023-12-25",
                test_start: "2023-12-26",
                test_end: "2024-06-25",
                clipped: false,
            },
            WfWindow {
                i: 7,
                train_start: "2018-12-26",
                train_end: "2024-06-25",
                test_start: "2024-06-26",
                test_end: "2024-12-25",
                clipped: false,
            },
            WfWindow {
                i: 8,
                train_start: "2018-12-26",
                train_end: "2024-12-25",
                test_start: "2024-12-26",
                test_end: "2025-06-25",
                clipped: false,
            },
            WfWindow {
                i: 9,
                train_start: "2018-12-26",
                train_end: "2025-06-25",
                test_start: "2025-06-26",
                test_end: "2025-06-30",
                clipped: true,
            },
        ],
        wf_rolling: &[
            WfWindow {
                i: 0,
                train_start: "2018-12-26",
                train_end: "2020-12-25",
                test_start: "2020-12-26",
                test_end: "2021-06-25",
                clipped: false,
            },
            WfWindow {
                i: 1,
                train_start: "2019-06-26",
                train_end: "2021-06-25",
                test_start: "2021-06-26",
                test_end: "2021-12-25",
                clipped: false,
            },
            WfWindow {
                i: 2,
                train_start: "2019-12-26",
                train_end: "2021-12-25",
                test_start: "2021-12-26",
                test_end: "2022-06-25",
                clipped: false,
            },
            WfWindow {
                i: 3,
                train_start: "2020-06-26",
                train_end: "2022-06-25",
                test_start: "2022-06-26",
                test_end: "2022-12-25",
                clipped: false,
            },
            WfWindow {
                i: 4,
                train_start: "2020-12-26",
                train_end: "2022-12-25",
                test_start: "2022-12-26",
                test_end: "2023-06-25",
                clipped: false,
            },
            WfWindow {
                i: 5,
                train_start: "2021-06-26",
                train_end: "2023-06-25",
                test_start: "2023-06-26",
                test_end: "2023-12-25",
                clipped: false,
            },
            WfWindow {
                i: 6,
                train_start: "2021-12-26",
                train_end: "2023-12-25",
                test_start: "2023-12-26",
                test_end: "2024-06-25",
                clipped: false,
            },
            WfWindow {
                i: 7,
                train_start: "2022-06-26",
                train_end: "2024-06-25",
                test_start: "2024-06-26",
                test_end: "2024-12-25",
                clipped: false,
            },
            WfWindow {
                i: 8,
                train_start: "2022-12-26",
                train_end: "2024-12-25",
                test_start: "2024-12-26",
                test_end: "2025-06-25",
                clipped: false,
            },
            WfWindow {
                i: 9,
                train_start: "2023-06-26",
                train_end: "2025-06-25",
                test_start: "2025-06-26",
                test_end: "2025-06-30",
                clipped: true,
            },
        ],
    },
    SymbolWindows {
        symbol: "QQQ",
        pool: Pool::Extended,
        data_start: "2018-05-01",
        data_end: "2026-06-24",
        is_window: Some(("2018-05-01", "2022-12-31")),
        oos: ("2023-01-01", "2025-06-30"),
        holdout: Some(("2025-07-01", "2026-06-24")),
        wf_anchored: &[
            WfWindow {
                i: 0,
                train_start: "2018-05-01",
                train_end: "2020-04-30",
                test_start: "2020-05-01",
                test_end: "2020-10-31",
                clipped: false,
            },
            WfWindow {
                i: 1,
                train_start: "2018-05-01",
                train_end: "2020-10-31",
                test_start: "2020-11-01",
                test_end: "2021-04-30",
                clipped: false,
            },
            WfWindow {
                i: 2,
                train_start: "2018-05-01",
                train_end: "2021-04-30",
                test_start: "2021-05-01",
                test_end: "2021-10-31",
                clipped: false,
            },
            WfWindow {
                i: 3,
                train_start: "2018-05-01",
                train_end: "2021-10-31",
                test_start: "2021-11-01",
                test_end: "2022-04-30",
                clipped: false,
            },
            WfWindow {
                i: 4,
                train_start: "2018-05-01",
                train_end: "2022-04-30",
                test_start: "2022-05-01",
                test_end: "2022-10-31",
                clipped: false,
            },
            WfWindow {
                i: 5,
                train_start: "2018-05-01",
                train_end: "2022-10-31",
                test_start: "2022-11-01",
                test_end: "2023-04-30",
                clipped: false,
            },
            WfWindow {
                i: 6,
                train_start: "2018-05-01",
                train_end: "2023-04-30",
                test_start: "2023-05-01",
                test_end: "2023-10-31",
                clipped: false,
            },
            WfWindow {
                i: 7,
                train_start: "2018-05-01",
                train_end: "2023-10-31",
                test_start: "2023-11-01",
                test_end: "2024-04-30",
                clipped: false,
            },
            WfWindow {
                i: 8,
                train_start: "2018-05-01",
                train_end: "2024-04-30",
                test_start: "2024-05-01",
                test_end: "2024-10-31",
                clipped: false,
            },
            WfWindow {
                i: 9,
                train_start: "2018-05-01",
                train_end: "2024-10-31",
                test_start: "2024-11-01",
                test_end: "2025-04-30",
                clipped: false,
            },
            WfWindow {
                i: 10,
                train_start: "2018-05-01",
                train_end: "2025-04-30",
                test_start: "2025-05-01",
                test_end: "2025-06-30",
                clipped: true,
            },
        ],
        wf_rolling: &[
            WfWindow {
                i: 0,
                train_start: "2018-05-01",
                train_end: "2020-04-30",
                test_start: "2020-05-01",
                test_end: "2020-10-31",
                clipped: false,
            },
            WfWindow {
                i: 1,
                train_start: "2018-11-01",
                train_end: "2020-10-31",
                test_start: "2020-11-01",
                test_end: "2021-04-30",
                clipped: false,
            },
            WfWindow {
                i: 2,
                train_start: "2019-05-01",
                train_end: "2021-04-30",
                test_start: "2021-05-01",
                test_end: "2021-10-31",
                clipped: false,
            },
            WfWindow {
                i: 3,
                train_start: "2019-11-01",
                train_end: "2021-10-31",
                test_start: "2021-11-01",
                test_end: "2022-04-30",
                clipped: false,
            },
            WfWindow {
                i: 4,
                train_start: "2020-05-01",
                train_end: "2022-04-30",
                test_start: "2022-05-01",
                test_end: "2022-10-31",
                clipped: false,
            },
            WfWindow {
                i: 5,
                train_start: "2020-11-01",
                train_end: "2022-10-31",
                test_start: "2022-11-01",
                test_end: "2023-04-30",
                clipped: false,
            },
            WfWindow {
                i: 6,
                train_start: "2021-05-01",
                train_end: "2023-04-30",
                test_start: "2023-05-01",
                test_end: "2023-10-31",
                clipped: false,
            },
            WfWindow {
                i: 7,
                train_start: "2021-11-01",
                train_end: "2023-10-31",
                test_start: "2023-11-01",
                test_end: "2024-04-30",
                clipped: false,
            },
            WfWindow {
                i: 8,
                train_start: "2022-05-01",
                train_end: "2024-04-30",
                test_start: "2024-05-01",
                test_end: "2024-10-31",
                clipped: false,
            },
            WfWindow {
                i: 9,
                train_start: "2022-11-01",
                train_end: "2024-10-31",
                test_start: "2024-11-01",
                test_end: "2025-04-30",
                clipped: false,
            },
            WfWindow {
                i: 10,
                train_start: "2023-05-01",
                train_end: "2025-04-30",
                test_start: "2025-05-01",
                test_end: "2025-06-30",
                clipped: true,
            },
        ],
    },
    SymbolWindows {
        symbol: "OKLO",
        pool: Pool::Observation,
        data_start: "2024-05-10",
        data_end: "2026-06-24",
        is_window: None,
        oos: ("2025-01-01", "2026-06-24"),
        holdout: None,
        wf_anchored: &[],
        wf_rolling: &[],
    },
];

/// 全局 OOS 边界（§2.1，核心/扩展池统一）。
pub const OOS_START: &str = "2023-01-01";
pub const OOS_END: &str = "2025-06-30";
/// 全局 Holdout 起（§2.1）。Holdout 末 = 各品种数据末。
pub const HOLDOUT_START: &str = "2025-07-01";
/// IS 末（§2.1，各品种 IS 起 = 数据起）。
pub const IS_END: &str = "2022-12-31";

#[cfg(test)]
mod tests {
    use super::*;

    /// 结构不变量：8 品种，池角色分布正确（核心 4 / 扩展 3 / 观察 1）。
    #[test]
    fn prereg_consistency_pool_counts() {
        assert_eq!(PREREG_WINDOWS.len(), 8, "8 品种（§1.1）");
        let core = PREREG_WINDOWS
            .iter()
            .filter(|w| w.pool == Pool::Core)
            .count();
        let ext = PREREG_WINDOWS
            .iter()
            .filter(|w| w.pool == Pool::Extended)
            .count();
        let obs = PREREG_WINDOWS
            .iter()
            .filter(|w| w.pool == Pool::Observation)
            .count();
        assert_eq!((core, ext, obs), (4, 3, 1), "核心4/扩展3/观察1（§1.3）");
    }

    /// 全局 OOS 统一性：核心+扩展池 7 品种 OOS 边界一致（§2.1）。
    #[test]
    fn prereg_consistency_unified_oos() {
        for w in PREREG_WINDOWS {
            if w.pool == Pool::Observation {
                continue; // OKLO §2.4 特例
            }
            assert_eq!(w.oos, (OOS_START, OOS_END), "{} OOS 须统一", w.symbol);
        }
    }

    /// 窗数锚（防漂移回归）：与文档 §3 表逐品种窗数一致。
    #[test]
    fn prereg_consistency_window_counts() {
        let expect = [
            ("BTC", 12),
            ("ES", 15),
            ("CL", 15),
            ("GC", 15),
            ("BRN", 10),
            ("DX", 10),
            ("QQQ", 11),
            ("OKLO", 0),
        ];
        for (sym, n) in expect {
            let w = PREREG_WINDOWS
                .iter()
                .find(|w| w.symbol == sym)
                .expect("品种存在");
            assert_eq!(w.wf_anchored.len(), n, "{sym} anchored 窗数");
            assert_eq!(
                w.wf_rolling.len(),
                n,
                "{sym} rolling 窗数（与 anchored 同数）"
            );
        }
    }

    /// 裁定 W1：每非空 walk-forward 序列的末窗 clipped（test 截到 OOS 末），其余不 clipped。
    #[test]
    fn prereg_w1_only_last_window_clipped() {
        for w in PREREG_WINDOWS {
            if w.wf_anchored.is_empty() {
                continue;
            }
            let n = w.wf_anchored.len();
            for (idx, win) in w.wf_anchored.iter().enumerate() {
                let is_last = idx == n - 1;
                assert_eq!(
                    win.clipped, is_last,
                    "{} anchored #{} clipped 应={}",
                    w.symbol, idx, is_last
                );
                if is_last {
                    assert_eq!(win.test_end, OOS_END, "{} 末窗 test 截到 OOS 末", w.symbol);
                }
            }
        }
    }

    /// OKLO §2.4 特例：无 IS、无 walk-forward、OOS=2025-01-01→数据末（OK1）。
    #[test]
    fn prereg_okla_special_case() {
        let oklo = PREREG_WINDOWS
            .iter()
            .find(|w| w.symbol == "OKLO")
            .expect("OKLO");
        assert_eq!(oklo.pool, Pool::Observation);
        assert!(oklo.is_window.is_none(), "OKLO 无 IS（§2.4）");
        assert!(
            oklo.wf_anchored.is_empty() && oklo.wf_rolling.is_empty(),
            "OKLO 无 walk-forward"
        );
        assert_eq!(oklo.oos.0, "2025-01-01", "OKLO 单段 OOS 起（§2.4）");
        assert_eq!(
            oklo.oos.1, oklo.data_end,
            "OKLO OOS 末=数据末（OK1 张力，含 Holdout 区）"
        );
        assert!(
            oklo.holdout.is_none(),
            "OKLO OOS 已含数据末 ⟹ 无独立 Holdout"
        );
    }
}
