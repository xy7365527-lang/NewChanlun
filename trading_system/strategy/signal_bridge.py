"""ChanlunBridge —— PyO3 桥接：Nautilus Bar → Rust 引擎 → 缠论信号。

设计文档判决三（analysis/nautilus_integration_design.md §0）：
引擎 process_bar(o,h,l,c) 无时间戳，bar index = 隐式时间轴。
时间戳↔bar_index 的双向映射、单调性守卫、gap 检测**全部**集中在本类，
引擎和 Strategy 都不碰时间换算。违反单调性（ts_event 倒退）= fail-fast 抛异常。
"""

from __future__ import annotations

import enum
from dataclasses import dataclass

import newchan_rust


class FeedResult(enum.Enum):
    """feed() 的三种结果：接受 / 重叠丢弃（可计数）/ 预热中。"""

    ACCEPTED = "accepted"
    DUPLICATE = "duplicate"


@dataclass(frozen=True)
class ChanlunSignal:
    """引擎产出的操作信号（不可变）。

    action        : "BUY" | "SELL" | "HOLD"
    level         : 买卖点所属级别（当前骨架恒为 1 = 走势级；递归层接入后扩展）
    notional_frac : 名义敞口配额比例 [0, 1]（fusion_tr 接入前为占位常数）
    price         : 结构锚定价（限价单的挂单基准，BSP 事件的 price 字段）
    kind          : BSP 种类（"type1"/"type2"/"type3"）
    reason        : 人读诊断（如 "bsp:type1:buy:confirmed seg=42"）
    bar_index     : 引擎隐式时间轴上的位置（回溯锚定用）
    ts_event_ns   : 对应 Nautilus 纳秒时间戳
    """

    action: str
    level: int
    notional_frac: float
    price: float
    kind: str
    reason: str
    bar_index: int
    ts_event_ns: int


class ChanlunBridge:
    """引擎驱动 + 时间轴守卫。一个实例绑定一个标的一个床位（bar 周期）。

    职责（全部集中此处）：
    - Bar(定点 Price) → f64 OHLC，喂 RecursiveOrchestrator
    - watermark 单调性守卫：重复=丢弃计数；倒退=抛异常（上游数据流坏了）
    - gap 只记录不填充（与现有"记录消除不填充"约定一致）
    - bar_index ↔ ts_event 映射（意图/诊断回溯锚定）
    - delta 信号消费：bsp_epoch 门控 + take_trend_bsp_events（O(Δ)）
    """

    def __init__(self, engine_config: dict | None = None) -> None:
        cfg = dict(engine_config or {})
        self._orch = newchan_rust.RecursiveOrchestrator(**cfg)
        self._watermark_ns: int | None = None  # 最后接受的 ts_event
        self._bar_interval_ns: int | None = None  # 由前两根 bar 推断
        self._bar_index: int = -1  # 引擎隐式时间轴
        self._ts_to_index: list[int] = []  # bar_index → ts_event_ns
        self._gap_count: int = 0
        self._dup_count: int = 0
        self._last_bsp_epoch: int = 0

    # ── 喂数（下行）─────────────────────────────────────────────

    def feed(self, bar) -> FeedResult:
        """喂入一根 Nautilus Bar。返回 ACCEPTED / DUPLICATE。

        定点→浮点只在此边界发生一次（Price.as_double()）。
        """
        ts = bar.ts_event
        if self._watermark_ns is not None:
            if ts < self._watermark_ns:
                raise RuntimeError(
                    f"ts_event 倒退：{ts} < watermark {self._watermark_ns}"
                    "——上游数据流坏了，fail-fast 不静默跳过"
                )
            if ts == self._watermark_ns:
                self._dup_count += 1
                return FeedResult.DUPLICATE
            if self._bar_interval_ns is None:
                self._bar_interval_ns = ts - self._watermark_ns
            elif ts - self._watermark_ns > self._bar_interval_ns:
                self._gap_count += 1  # 只记录不填充
        self._orch.process_bar(
            bar.open.as_double(),
            bar.high.as_double(),
            bar.low.as_double(),
            bar.close.as_double(),
        )
        self._bar_index += 1
        self._ts_to_index.append(ts)
        self._watermark_ns = ts
        return FeedResult.ACCEPTED

    # ── 信号消费（上行）─────────────────────────────────────────

    def drain_signals(self) -> list[ChanlunSignal]:
        """读出自上次调用以来的新信号（O(Δ)，epoch 门控）。

        当前骨架的信号映射：走势级 confirmed BSP 事件 → BUY/SELL。
        TODO(阶段2): 接入 PositionalStream（设计 §4.2）后，本方法改为
            row = 组装磁带行(bsp/div/move_settle 事件)
            intents = stream.on_bar_row(row)
        由 fusion_tr/hold26 voice 状态机产出意图，BSP 直读映射退役。
        """
        epoch = self._orch.bsp_epoch()
        if epoch == self._last_bsp_epoch:
            return []  # epoch 未变 ⟹ 无新事件，O(1) 跳过
        self._last_bsp_epoch = epoch

        _b1, _s1, _sa, _ba, events = self._orch.take_trend_bsp_events()
        signals: list[ChanlunSignal] = []
        for kind, side, seg_idx, confirmed, _css, _zd, _zg, price in events:
            if not confirmed:
                continue  # candidate 不出操作信号（PH settle 边界：candidate 仅形态学）
            action = "BUY" if side == "buy" else "SELL"
            signals.append(
                ChanlunSignal(
                    action=action,
                    level=1,
                    # TODO(阶段2): notional_frac 由 voice 配额产出；骨架占位 1.0
                    notional_frac=1.0,
                    price=price,
                    kind=kind,
                    reason=f"bsp:{kind}:{side}:confirmed seg={seg_idx}",
                    bar_index=self._bar_index,
                    ts_event_ns=self._watermark_ns or 0,
                )
            )
        return signals

    def drain_div_events(self) -> list[tuple]:
        """走势级背驰 delta 事件（诊断/日志用）。"""
        return self._orch.take_trend_div_events()

    # ── 状态查询 ────────────────────────────────────────────────

    @property
    def bar_count(self) -> int:
        return self._bar_index + 1

    @property
    def gap_count(self) -> int:
        return self._gap_count

    @property
    def dup_count(self) -> int:
        return self._dup_count

    @property
    def watermark_ns(self) -> int | None:
        return self._watermark_ns

    def structure_snapshot(self) -> dict:
        """当前结构计数（诊断用，全量 marshal——只在收盘/停机时调用）。"""
        return {
            "strokes": len(self._orch.current_strokes()),
            "segments": len(self._orch.current_segments()),
            "zhongshus": len(self._orch.current_zhongshus()),
            "moves": len(self._orch.current_moves()),
        }

    def current_zhongshu_band(self) -> tuple[float, float] | None:
        """最近中枢的 (ZD, ZG)——LeverageCalculator 的 P_neg 输入。

        TODO(阶段3): P_neg 严格定义由相位决定（MOVE↑→ZG_new；OSC→ZD，
        见 notional_exposure_leverage_research）。骨架先暴露最近中枢带。
        """
        zss = self._orch.current_zhongshus()
        if not zss:
            return None
        last = zss[-1]
        # ZhongshuTuple: (zd, zg, seg_start, seg_end, ...) —— rust/src/lib.rs:204
        return (last[0], last[1])
