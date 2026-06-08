"""PH persistence 分层赋格 FSM —— persistence 是级别本身，不是过滤器。

存在论位置
----------
`a_online_persistence.OnlineMergeTree` 一次跑完，**所有尺度的 settle 事件同时产出**。
本模块把 persistence 当作**级别度量**（不是噪声过滤阈值）：一个 settle 事件的
persistence 大小直接决定它属于哪个缠论级别（L0/L1/L2）。这是 §7.5 因果 merge tree
"按 prominence 组织的区间套"的操盘读出——区间套的层数 = 级别的层数。

双树（buy-side / sell-side）
----------------------------
- **sublevel 树**（喂 `close`）：分量是价格谷底；settle = 下跌波 death 因果确定
  （价格升过右侧屏障）= **底部确认** → BUY-side 信号。
- **superlevel 树**（喂 `-close`）：-价的谷 = 正价的峰；settle = 上涨波顶部确认
  （价格跌破右侧屏障）= **顶部确认** → SELL-side 信号。
  persistence 在 -价空间 = peak − barrier_trough，仍为正、价格单位，与 sublevel 同尺度。

因果 settle 的精髓（零前视根据）：settle 事件的 death_idx（鞍点）在过去，但 settle 在
**当前 bar** 价格突破屏障时才确认。信号在当前 bar 收盘可交易——未来数据零参与。

级别阈值 τ（因果自适应）
------------------------
τ 必须**因果**：用全样本分位数 = 前视作弊（formalization-validity-domain 规则）。
本模块用**滚动 ATR 倍数**——persistence/ATR 是无量纲的级别度量，随价格水平自适应：
  τ₀ = k₀·ATR  (L0 下界，过滤 sub-τ₀ 噪声)
  τ₁ = k₁·ATR  (L0/L1 分界)
  τ₂ = k₂·ATR  (L1/L2 分界)
band(persistence) = 2 if ≥τ₂ ; 1 if ≥τ₁ ; 0 if ≥τ₀ ; None(噪声) otherwise。

赋格三声部（自相似/分形）
-------------------------
三个独立 CostReductionFSM 实例，每个 Lk 声部以**主带 k + 子带 k−1** 运行同一支
267号降成本旋律：
  - L2 声部：主带2(进/出主仓) + 子带1(短差降成本)
  - L1 声部：主带1(进/出) + 子带0(短差)
  - L0 声部：主带0(进/出) + 无子带(最细，仅开平)
band-1 的 SELL 信号**同时**是 L2 的短差减仓与 L1 的主清仓——这就是赋格声部重叠
（同一中级别下跌，对 L2 是短差、对 L1 是整段）。三声部并行于同一价格流，按 persistence
带分工。total equity = 三声部之和。

认识论等级（formalization-validity-domain）
-------------------------------------------
- band 分层 / 双树 / 因果 settle：**L0**（merge tree 确定性属性，零信息增量）。
- ATR-τ 自适应 + streaming 零前视：**L0**（纯算法/管线）。
- "三层赋格 > 单层 baseline" 等收益断言：**L2**（QQQ 5min 单标的单时段，可否证）。
- 缠论买卖点分类（一/二/三买）：基于 persistence(=幅度∈ker(D))，是**幅度代理**，
  **非 MACD 力度背驰**——只提供必要条件过滤（见 a_online_persistence 模块顶部边界）。

概念溯源标签
-----------
- persistence = 级别 [新缠论:候选——§7.5 区间套层数=级别层数]
- 双树 buy/sell-side settle [新缠论:候选——sublevel↔底 / superlevel↔顶]
- 赋格三声部 [缠论:267号操作方法论 v1——满仓满融降成本 + 小转大赋格]
"""

from __future__ import annotations

from dataclasses import dataclass, field, replace
from enum import Enum, auto

from newchan.a_online_persistence import MergeBar, OnlineMergeTree
from newchan.trading.cost_reduction_fsm import (
    CostReductionFSM,
    CostState,
    FsmEvent,
    FsmEventType,
    transition,
)

__all__ = [
    "Side",
    "PersistenceBands",
    "Signal",
    "rolling_atr",
    "DualTreeStream",
    "VoiceLedger",
    "TradeRecord",
    "FugueVoice",
    "ChanBspType",
    "ChanlunBspClassifier",
]


# ====================================================================
# 信号 / 级别带
# ====================================================================


class Side(Enum):
    """信号方向：BUY = 底部确认（sublevel settle），SELL = 顶部确认（superlevel settle）。"""

    BUY = auto()
    SELL = auto()


@dataclass(frozen=True, slots=True)
class PersistenceBands:
    """ATR 倍数级别阈值（因果自适应）。

    k0 < k1 < k2。band ∈ {0,1,2} 或 None（sub-τ₀ 噪声）。
    """

    k0: float = 1.0
    k1: float = 3.0
    k2: float = 8.0

    def classify(self, persistence: float, atr: float) -> int | None:
        """persistence + 当前 ATR → 级别带（2/1/0/None）。"""
        if atr <= 0.0:
            return None
        if persistence >= self.k2 * atr:
            return 2
        if persistence >= self.k1 * atr:
            return 1
        if persistence >= self.k0 * atr:
            return 0
        return None


@dataclass(frozen=True, slots=True)
class Signal:
    """某根 bar 上因果确认的一个级别信号（settle 事件读出）。"""

    bar_idx: int
    side: Side
    band: int            # 0 / 1 / 2
    persistence: float
    price: float         # 当前 bar close（信号可交易价，零前视）
    birth_idx: int       # 触发特征的 valley/peak 诞生索引（缠论分类用）
    birth_price: float   # 该特征的诞生极值价（buy=谷底价, sell=峰顶价）


# ====================================================================
# 因果滚动 ATR
# ====================================================================


def rolling_atr(
    highs: list[float], lows: list[float], closes: list[float], *, window: int = 270
) -> list[float]:
    """因果滚动 ATR（True Range 的滚动均值，单位=价格）。

    atr[i] 只用 [0..i] 的数据 → 纯因果。window 内不足时用已有 TR 均值。
    atr[0] = high0 - low0（无前收）。返回与输入等长。
    """
    n = len(closes)
    tr = [0.0] * n
    tr[0] = highs[0] - lows[0]
    for i in range(1, n):
        prev_c = closes[i - 1]
        tr[i] = max(highs[i] - lows[i], abs(highs[i] - prev_c), abs(lows[i] - prev_c))
    atr = [0.0] * n
    run = 0.0
    for i in range(n):
        run += tr[i]
        if i >= window:
            run -= tr[i - window]
            atr[i] = run / window
        else:
            atr[i] = run / (i + 1)
    return atr


# ====================================================================
# 双树 streaming 驱动（buy-side + sell-side 同时产出）
# ====================================================================


class DualTreeStream:
    """sublevel(buy) + superlevel(sell) 双在线 merge tree 的流式包装。

    每根 bar `step(close)` 返回本根新 settle 的级别信号列表（buy/sell 混合）。
    完全因果——只喂当前 close，settle 事件按因果判据实时确认。
    """

    __slots__ = ("_sub", "_sup", "_bands", "_i")

    def __init__(self, bands: PersistenceBands) -> None:
        # track_dominant=False：跳过 O(n²) 的 _record_dom（只用 max_alive_persistence）。
        self._sub = OnlineMergeTree(track_dominant=False)   # buy-side（价格谷 = 底）
        self._sup = OnlineMergeTree(track_dominant=False)   # sell-side（-价格谷 = 顶）
        self._bands = bands
        self._i = -1

    def step(self, close: float, atr: float) -> list[Signal]:
        """喂一根 close + 当前 ATR → 本根因果确认的级别信号。"""
        self._i += 1
        i = self._i
        out: list[Signal] = []
        for mb in self._sub.update(close):
            band = self._bands.classify(mb.persistence, atr)
            if band is not None:
                out.append(Signal(i, Side.BUY, band, mb.persistence, close,
                                   mb.birth_idx, mb.birth_price))
        for mb in self._sup.update(-close):
            band = self._bands.classify(mb.persistence, atr)
            if band is not None:
                # superlevel 在 -价空间：birth_price(-) → 正价峰 = -birth_price
                out.append(Signal(i, Side.SELL, band, mb.persistence, close,
                                   mb.birth_idx, -mb.birth_price))
        return out

    def dominant_alive_buy_persistence(self) -> float:
        """当前 buy-side(sublevel) 最大 alive persistence —— LEVEL_UPGRADE 检测用。

        用 O(栈深) 的 max_alive_persistence，避免每 bar 构造/排序整个 barcode 快照。
        """
        return self._sub.max_alive_persistence


# ====================================================================
# 权益账本（MTM）—— 与 FSM 状态分离的真实资金流
# ====================================================================


@dataclass(slots=True)
class VoiceLedger:
    """单声部的真实资金账本（mark-to-market 权益）。

    held_shares 在短差中波动；FSM 的 total_shares 是满仓目标。
    equity = cash + held_shares × price。realized 累计已实现盈亏。
    采用移动平均成本法核算已实现盈亏。
    """

    cash: float
    held_shares: float = 0.0
    avg_cost: float = 0.0
    realized: float = 0.0

    def buy(self, price: float, shares: float) -> None:
        """买入 shares，更新移动平均成本。"""
        if shares <= 0.0:
            return
        cost = price * shares
        new_total = self.held_shares + shares
        self.avg_cost = (self.avg_cost * self.held_shares + cost) / new_total
        self.held_shares = new_total
        self.cash -= cost

    def sell(self, price: float, shares: float) -> float:
        """卖出 shares（≤ held），返回本次已实现盈亏。"""
        shares = min(shares, self.held_shares)
        if shares <= 0.0:
            return 0.0
        pnl = (price - self.avg_cost) * shares
        self.realized += pnl
        self.cash += price * shares
        self.held_shares -= shares
        if self.held_shares <= 1e-9:
            self.held_shares = 0.0
            self.avg_cost = 0.0
        return pnl

    def invest_all(self, price: float, *, leverage: float = 1.0) -> float:
        """用全部 cash（×leverage）满仓买入，返回买入 shares。"""
        budget = self.cash * leverage
        if budget <= 0.0 or price <= 0.0:
            return 0.0
        shares = budget / price
        self.buy(price, shares)
        return shares

    def equity(self, price: float) -> float:
        return self.cash + self.held_shares * price


# ====================================================================
# 交易记录
# ====================================================================


@dataclass(frozen=True, slots=True)
class TradeRecord:
    """一笔成交记录（开/平/短差），用于审计与 equity 归因。"""

    bar_idx: int
    layer: str            # 'L0' | 'L1' | 'L2'
    action: str           # 'OPEN' | 'CLOSE' | 'SHORT_SELL' | 'SHORT_BUY' | 'UPGRADE'
    band: int
    side: str             # 'BUY' | 'SELL'
    price: float
    shares: float
    realized_pnl: float
    persistence: float
    note: str = ""


# ====================================================================
# 赋格声部（一个级别 = 一个 CostReductionFSM + 一个 Ledger）
# ====================================================================


@dataclass(slots=True)
class FugueVoice:
    """单个级别声部：主带进出 + 子带短差降成本。

    自相似——L2(主2/子1)、L1(主1/子0)、L0(主0/无子)共用同一支旋律。
    FSM 管状态与 267号成本核算；Ledger 管真实资金 MTM；本类做信号→事件→资金翻译。
    """

    name: str                 # 'L0' | 'L1' | 'L2'
    main_band: int
    sub_band: int | None
    fsm: CostReductionFSM
    ledger: VoiceLedger
    sub_ratio: float = 0.3
    leverage: float = 1.0
    stop_atr_mult: float = 0.0      # >0 时启用硬止损（entry − k·ATR），0=禁用
    entry_price: float = 0.0
    entry_bar: int = -1
    upgraded: bool = False          # 本持仓周期是否已触发 LEVEL_UPGRADE（去重）
    trades: list[TradeRecord] = field(default_factory=list)

    @staticmethod
    def create(
        name: str, main_band: int, sub_band: int | None, capital: float, *,
        sub_ratio: float = 0.3, leverage: float = 1.0, stop_atr_mult: float = 0.0,
    ) -> FugueVoice:
        return FugueVoice(
            name=name, main_band=main_band, sub_band=sub_band,
            fsm=CostReductionFSM.create(own_capital=capital, sub_ratio=sub_ratio),
            ledger=VoiceLedger(cash=capital),
            sub_ratio=sub_ratio, leverage=leverage, stop_atr_mult=stop_atr_mult,
        )

    @property
    def holding(self) -> bool:
        return self.fsm.state in (
            CostState.POSITION_OPEN, CostState.COST_REDUCING,
            CostState.PRINCIPAL_WITHDRAWN,
        )

    @property
    def has_open_short(self) -> bool:
        sd = self.fsm.active_short_diff
        return sd is not None and sd.is_open

    # ---------------------------------------------------------------

    def on_signal(self, sig: Signal) -> None:
        """消费一个级别信号，按声部角色驱动 FSM + Ledger。"""
        # ── 主带信号 ──
        if sig.band == self.main_band:
            if sig.side is Side.BUY and self.fsm.state is CostState.SCANNING:
                self._open(sig)
            elif sig.side is Side.SELL and self.holding:
                self._close(sig)
            return
        # ── 子带信号（短差）──
        if self.sub_band is not None and sig.band == self.sub_band and self.holding:
            if sig.side is Side.SELL and not self.has_open_short:
                self._short_sell(sig)
            elif sig.side is Side.BUY and self.has_open_short:
                self._short_buy(sig)

    def _open(self, sig: Signal) -> None:
        shares = self.ledger.invest_all(sig.price, leverage=self.leverage)
        self.fsm = transition(
            self.fsm, FsmEvent(FsmEventType.BUY_POINT_CONFIRMED, sig.price, self.name)
        )
        self.entry_price = sig.price
        self.entry_bar = sig.bar_idx
        self.upgraded = False
        self.trades.append(TradeRecord(
            sig.bar_idx, self.name, "OPEN", sig.band, "BUY", sig.price, shares,
            0.0, sig.persistence,
        ))

    def _close(self, sig: Signal, *, note: str = "") -> None:
        # 267号状态表：POSITION_OPEN（未进降成本）不接受 MAIN_LEVEL_SELL_POINT——
        # 此时顶部确认 = 多头买点被否定，走 BUY_POINT_NEGATED（同一 _stop_out 出口）。
        # COST_REDUCING / PRINCIPAL_WITHDRAWN 才接受主级别卖点。资金动作两者一致（清仓）。
        ev = (
            FsmEventType.MAIN_LEVEL_SELL_POINT
            if self.fsm.state in (CostState.COST_REDUCING, CostState.PRINCIPAL_WITHDRAWN)
            else FsmEventType.BUY_POINT_NEGATED
        )
        shares = self.ledger.held_shares
        pnl = self.ledger.sell(sig.price, shares)
        self.fsm = transition(self.fsm, FsmEvent(ev, sig.price, self.name))
        self.trades.append(TradeRecord(
            sig.bar_idx, self.name, "CLOSE", sig.band, "SELL", sig.price, shares,
            pnl, sig.persistence, note,
        ))
        # 平仓后重置 → 复利（new_capital = 当前现金）
        self.fsm = transition(
            self.fsm,
            FsmEvent(FsmEventType.RESET, sig.price, self.name,
                     new_own_capital=self.ledger.cash),
        )
        self.entry_price = 0.0
        self.entry_bar = -1
        self.upgraded = False

    def _short_sell(self, sig: Signal) -> None:
        shares = self.ledger.held_shares * self.sub_ratio
        pnl = self.ledger.sell(sig.price, shares)  # 减仓即时落现金（成本法 pnl 为该批毛利）
        self.fsm = transition(
            self.fsm, FsmEvent(FsmEventType.SUB_LEVEL_SELL_POINT, sig.price,
                               self.sub_label())
        )
        self.trades.append(TradeRecord(
            sig.bar_idx, self.name, "SHORT_SELL", sig.band, "SELL", sig.price, shares,
            pnl, sig.persistence,
        ))

    def _short_buy(self, sig: Signal) -> None:
        sd = self.fsm.active_short_diff
        assert sd is not None
        shares = sd.shares
        self.ledger.buy(sig.price, shares)         # 低位买回
        before = self.ledger.realized
        self.fsm = transition(
            self.fsm, FsmEvent(FsmEventType.SUB_LEVEL_BUY_POINT, sig.price,
                               self.sub_label())
        )
        # 短差闭合的"利润"在成本法下已体现在 _short_sell 的 realized 中；
        # 这里记录买回动作，realized 增量为 0（买入不实现盈亏）。
        self.trades.append(TradeRecord(
            sig.bar_idx, self.name, "SHORT_BUY", sig.band, "BUY", sig.price, shares,
            self.ledger.realized - before, sig.persistence,
        ))

    def maybe_upgrade(self, sig_bar: int, dom_alive_pers: float, tau2: float,
                      price: float) -> None:
        """小转大：持仓中 buy-side 主导 alive persistence 越过 τ₂ → 赋格分裂（去重）。

        只对非 L2 声部有意义（L2 已是最高带）。记录利润底仓 + 新循环 voice。
        267号§三：小转大发生在**降成本过程中**——故仅 COST_REDUCING 状态接受
        LEVEL_UPGRADE（FSM 状态表约束，非补丁）。刚建仓未短差(POSITION_OPEN)或本金
        已退(PRINCIPAL_WITHDRAWN)不触发。
        """
        if (self.name == "L2" or self.upgraded
                or self.fsm.state is not CostState.COST_REDUCING):
            return
        if dom_alive_pers >= tau2:
            self.fsm = transition(
                self.fsm, FsmEvent(FsmEventType.LEVEL_UPGRADE, price, "L2")
            )
            self.upgraded = True
            self.trades.append(TradeRecord(
                sig_bar, self.name, "UPGRADE", 2, "BUY", price,
                self.ledger.held_shares, 0.0, dom_alive_pers, note="小转大→赋格分裂",
            ))

    def check_stop(self, bar_idx: int, price: float, atr: float) -> None:
        """硬止损（可选）：持仓且 price < entry − stop_atr_mult·ATR → 清仓。"""
        if self.stop_atr_mult <= 0.0 or not self.holding or self.entry_price <= 0.0:
            return
        if price < self.entry_price - self.stop_atr_mult * atr:
            shares = self.ledger.held_shares
            pnl = self.ledger.sell(price, shares)
            self.fsm = transition(
                self.fsm,
                FsmEvent(FsmEventType.BUY_POINT_NEGATED, price, self.name),
            )
            self.trades.append(TradeRecord(
                bar_idx, self.name, "CLOSE", self.main_band, "SELL", price, shares,
                pnl, 0.0, note="硬止损",
            ))
            self.fsm = transition(
                self.fsm,
                FsmEvent(FsmEventType.RESET, price, self.name,
                         new_own_capital=self.ledger.cash),
            )
            self.entry_price = 0.0
            self.entry_bar = -1
            self.upgraded = False

    def sub_label(self) -> str:
        return f"{self.name}.sub"

    def equity(self, price: float) -> float:
        return self.ledger.equity(price)


# ====================================================================
# 缠论买卖点因果分类器（幅度代理，非力度背驰——见模块顶部边界）
# ====================================================================


class ChanBspType(Enum):
    """缠论买点类型（因果分类，基于 persistence=幅度）。"""

    TYPE1 = auto()   # 一买：创新低 + 幅度衰减（背驰代理）
    TYPE2 = auto()   # 二买：不创新低（higher-low）
    TYPE3 = auto()   # 三买：回踩不破前中枢/前低
    NONE = auto()    # 不构成已知买点类型


@dataclass(slots=True)
class ChanlunBspClassifier:
    """对 BUY-side settle 信号做因果缠论买点分类。

    认识论：persistence = 幅度 ∈ ker(D)（§5），**只能算"创新低/幅度衰减"**，
    不能算 MACD 力度背驰。故"一买"是**幅度背驰代理**（创新低 + persistence 较前次低
    更小），是必要条件过滤器，不是充分确认（a_online_persistence 模块顶部边界）。

    状态：记录历次 BUY 信号的 (birth_price 谷底, persistence)。
    """

    _last_low: float | None = None        # 上一确认谷底价（最近一次确认底）
    _last_pers: float = 0.0               # 上一确认谷底的 persistence
    _last_type: ChanBspType = ChanBspType.NONE   # 上一非 NONE 分类（推进一→二→三买序列）

    def classify(self, sig: Signal) -> ChanBspType:
        if sig.side is not Side.BUY:
            return ChanBspType.NONE
        low = sig.birth_price
        pers = sig.persistence
        if self._last_low is None:
            result = ChanBspType.TYPE1                 # 首个确认底，按一买处理
        elif low < self._last_low:
            # 创新低：幅度衰减(pers↓) → 一买（背驰代理）；幅度未减 → 中继(非买点)
            result = ChanBspType.TYPE1 if pers < self._last_pers else ChanBspType.NONE
        else:
            # higher-low（不创新低）：一买后的第一个回升 → 二买；其后仍高于底 → 三买
            result = (
                ChanBspType.TYPE2
                if self._last_type is ChanBspType.TYPE1
                else ChanBspType.TYPE3
            )
        self._last_low = low
        self._last_pers = pers
        if result is not ChanBspType.NONE:
            self._last_type = result
        return result
