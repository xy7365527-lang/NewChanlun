"""Interactive Brokers (TWS / IB Gateway) 数据源"""

from __future__ import annotations

import asyncio
import collections
import threading
import time as _time
from datetime import datetime
from typing import Self

# Python 3.12+ 不再自动在主线程创建 event loop，
# 而 ib_insync (eventkit) 在导入时就需要它。
try:
    asyncio.get_running_loop()
except RuntimeError:
    _loop = asyncio.new_event_loop()
    asyncio.set_event_loop(_loop)

from ib_insync import IB, ContFuture, Contract, Future, Stock, util  # noqa: E402

from newchan.config import IB_CLIENT_ID, IB_HOST, IB_PORT
from newchan.types import Bar

# ------------------------------------------------------------------
# 常用期货品种 -> (exchange, multiplier) 映射
# ------------------------------------------------------------------

_FUTURES_MAP: dict[str, tuple[str, str]] = {
    "CL": ("NYMEX", "1000"),   # 原油
    "GC": ("COMEX", "100"),    # 黄金
    "SI": ("COMEX", "5000"),   # 白银
    "ES": ("CME", "50"),       # 标普 E-mini
    "NQ": ("CME", "20"),       # 纳指 E-mini
    "YM": ("CBOT", "5"),       # 道指 E-mini
    "RTY": ("CME", "50"),      # 罗素 2000 E-mini
    "ZB": ("CBOT", "1000"),    # 30 年国债
    "ZN": ("CBOT", "1000"),    # 10 年国债
    "NG": ("NYMEX", "10000"),  # 天然气
    "HG": ("COMEX", "25000"),  # 铜
}

# ib_insync barSizeSetting 值映射（用户友好名 -> IB API 字符串）
_BAR_SIZE_MAP: dict[str, str] = {
    "1s": "1 secs",
    "5s": "5 secs",
    "10s": "10 secs",
    "15s": "15 secs",
    "30s": "30 secs",
    "1min": "1 min",
    "2min": "2 mins",
    "3min": "3 mins",
    "5min": "5 mins",
    "10min": "10 mins",
    "15min": "15 mins",
    "20min": "20 mins",
    "30min": "30 mins",
    "1hour": "1 hour",
    "2hour": "2 hours",
    "3hour": "3 hours",
    "4hour": "4 hours",
    "8hour": "8 hours",
    "1day": "1 day",
    "1week": "1 week",
    "1month": "1 month",
}


def supported_symbols() -> list[str]:
    """返回内置支持的期货品种列表。"""
    return sorted(_FUTURES_MAP.keys())


class IBKRProvider:
    """Interactive Brokers 历史行情提供者。

    Parameters
    ----------
    host : str
        TWS / Gateway 地址，默认读取 config。
    port : int
        TWS / Gateway 端口，默认读取 config。
    client_id : int
        客户端 ID，默认读取 config。
    """

    def __init__(
        self,
        host: str | None = None,
        port: int | None = None,
        client_id: int | None = None,
    ) -> None:
        import random
        self.host = host or IB_HOST
        self.port = port or IB_PORT
        self.client_id = client_id if client_id is not None else random.randint(1, 99)
        self._ib = IB()

    # ------------------------------------------------------------------
    # 连接管理（支持 context manager）
    # ------------------------------------------------------------------

    def connect(self) -> Self:
        """连接到 TWS / IB Gateway。"""
        self._ib.connect(self.host, self.port, clientId=self.client_id)
        return self

    def disconnect(self) -> None:
        """断开连接。"""
        if self._ib.isConnected():
            self._ib.disconnect()

    def __enter__(self) -> Self:
        return self.connect()

    def __exit__(self, *exc) -> None:  # noqa: ANN002
        self.disconnect()

    # ------------------------------------------------------------------
    # Contract 构造
    # ------------------------------------------------------------------

    @staticmethod
    def make_contract(symbol: str) -> Contract:
        """根据品种代码构造合约。

        - 若在期货映射表中 → ContFuture（连续合约）
        - 否则 → Stock（默认 SMART/USD）
        """
        symbol = symbol.upper()
        if symbol in _FUTURES_MAP:
            exchange, _ = _FUTURES_MAP[symbol]
            return ContFuture(symbol=symbol, exchange=exchange, currency="USD")
        # 默认当作美股处理
        return Stock(symbol=symbol, exchange="SMART", currency="USD")

    # ------------------------------------------------------------------
    # 拉取历史数据
    # ------------------------------------------------------------------

    def fetch_historical(
        self,
        symbol: str,
        interval: str = "1min",
        duration: str = "2 D",
        what_to_show: str = "TRADES",
        use_rth: bool = False,
    ) -> list[Bar]:
        """拉取历史 K 线。

        Parameters
        ----------
        symbol : str
            品种代码（如 CL、GC）。
        interval : str
            K 线周期，支持: {intervals}。
        duration : str
            拉取时长，IB 格式，如 ``"2 D"``、``"1 W"``、``"1 M"``。
        what_to_show : str
            数据类型，默认 ``"TRADES"``。
        use_rth : bool
            是否仅包含常规交易时段，默认 ``False``（包含盘前盘后）。

        Returns
        -------
        list[Bar]
        """.format(intervals=", ".join(sorted(_BAR_SIZE_MAP.keys())))

        bar_size = _BAR_SIZE_MAP.get(interval)
        if bar_size is None:
            raise ValueError(
                f"不支持的 interval '{interval}'，可选: "
                f"{', '.join(sorted(_BAR_SIZE_MAP.keys()))}"
            )

        contract = self._qualify_contract(symbol)
        ib_bars = self._ib.reqHistoricalData(
            contract,
            endDateTime="",
            durationStr=duration,
            barSizeSetting=bar_size,
            whatToShow=what_to_show,
            useRTH=use_rth,
            formatDate=1,
        )

        return _ib_bars_to_bars(ib_bars)

    def _qualify_contract(self, symbol: str) -> Contract:
        """构造并解析合约。"""
        contract = self.make_contract(symbol)
        qualified = self._ib.qualifyContracts(contract)
        if not qualified:
            raise RuntimeError(f"无法解析合约: {contract}")
        return contract


def _ib_bars_to_bars(ib_bars) -> list[Bar]:
    """将 ib_insync BarData 列表转换为 Bar 列表。"""
    if not ib_bars:
        return []
    bars: list[Bar] = []
    for b in ib_bars:
        ts = b.date if isinstance(b.date, datetime) else util.parseIBDatetime(str(b.date))
        bars.append(Bar(
            ts=ts,
            open=b.open,
            high=b.high,
            low=b.low,
            close=b.close,
            volume=b.volume if b.volume > 0 else None,
        ))
    return bars


# ======================================================================
# IBKRConnection — 单例长连接（搜索 + 实时订阅）
# ======================================================================


class IBKRConnection:
    """IBKR 持久连接，供品种搜索和实时数据订阅共用。

    使用方式::

        conn = IBKRConnection.instance()
        conn.connect()
        results = conn.search_symbols("crude")
        conn.subscribe_realtime("CL")
        bars = conn.get_latest_bars("CL")
    """

    _instance: IBKRConnection | None = None
    _cls_lock = threading.Lock()

    def __init__(self) -> None:
        self._ib = IB()
        self._connected = False
        self._conn_loop: asyncio.AbstractEventLoop | None = None  # 连接时的 event loop
        self._lock = threading.RLock()  # 保护所有 IB API 调用
        # 实时数据缓冲: symbol -> deque of Bar
        self._rt_buffers: dict[str, collections.deque] = {}
        # 实时订阅 handle: symbol -> RealTimeBarList
        self._rt_subs: dict[str, object] = {}
        # 后台事件循环线程
        self._bg_thread: threading.Thread | None = None
        self._stop_event = threading.Event()

    @classmethod
    def instance(cls) -> IBKRConnection:
        """获取单例实例。"""
        if cls._instance is None:
            with cls._cls_lock:
                if cls._instance is None:
                    cls._instance = cls()
        return cls._instance

    # ------------------------------------------------------------------
    # 连接管理
    # ------------------------------------------------------------------

    @property
    def connected(self) -> bool:
        return self._connected and self._ib.isConnected()

    def _ensure_loop(self) -> None:
        """确保当前线程使用连接时建立的 asyncio event loop。"""
        if self._conn_loop is not None:
            asyncio.set_event_loop(self._conn_loop)
        else:
            try:
                asyncio.get_event_loop()
            except RuntimeError:
                asyncio.set_event_loop(asyncio.new_event_loop())

    def connect(
        self,
        host: str | None = None,
        port: int | None = None,
        client_id: int | None = None,
    ) -> None:
        """连接 IBKR（如果尚未连接）。"""
        if self.connected:
            return
        h = host or IB_HOST
        p = port or IB_PORT
        import random
        cid = client_id if client_id is not None else random.randint(100, 999)
        try:
            self._ib.connect(h, p, clientId=cid, readonly=True)
            self._connected = True
            self._conn_loop = asyncio.get_event_loop()
        except Exception:
            self._connected = False
            raise

    def disconnect(self) -> None:
        """断开连接。"""
        for sym in list(self._rt_subs.keys()):
            self._cancel_rt(sym)
        if self._ib.isConnected():
            self._ib.disconnect()
        self._connected = False

    def pump(self, secs: float = 0.1) -> None:
        """驱动 ib_insync 处理挂起事件（由 API 调用触发）。"""
        if not self.connected:
            return
        try:
            self._ensure_loop()
            self._ib.sleep(secs)
        except Exception:
            pass

    # ------------------------------------------------------------------
    # 品种搜索
    # ------------------------------------------------------------------

    def search_symbols(self, keyword: str) -> list[dict]:
        """通过 IBKR reqMatchingSymbols 搜索品种。"""
        if not self.connected:
            print(f"[search] 未连接，跳过 IB 搜索", flush=True)
            return []
        try:
            self._ensure_loop()
            descs = self._ib.reqMatchingSymbols(keyword)
            print(f"[search] 返回 {len(descs) if descs else 0} 条", flush=True)
        except Exception as e:
            print(f"[search] reqMatchingSymbols 错误: {e}", flush=True)
            import traceback; traceback.print_exc()
            return []
        if not descs:
            return []
        results = []
        for d in descs[:16]:
            c = d.contract
            results.append({
                "symbol": c.symbol,
                "secType": c.secType,
                "exchange": c.primaryExchange or c.exchange or "",
                "currency": c.currency,
                "description": ", ".join(getattr(d, "derivativeSecTypes", []) or []),
            })
        return results

    # ------------------------------------------------------------------
    # 历史数据拉取（复用长连接）
    # ------------------------------------------------------------------

    def fetch_and_cache(
        self,
        symbol: str,
        interval: str = "1min",
        duration: str = "2 D",
        what_to_show: str = "TRADES",
    ) -> "pd.DataFrame | None":
        """用已有长连接拉取最新历史数据，写缓存并返回 DataFrame。"""
        if not self.connected:
            return None

        bar_size = _BAR_SIZE_MAP.get(interval)
        if bar_size is None:
            return None

        ib_bars = self._fetch_ib_bars(symbol, bar_size, duration, what_to_show)
        if not ib_bars:
            return None

        bars = _ib_bars_to_bars(ib_bars)
        return self._cache_bars(bars, symbol, interval)

    def _fetch_ib_bars(
        self, symbol: str, bar_size: str, duration: str, what_to_show: str,
    ):
        """通过长连接拉取 IB 原始 bar 数据。"""
        try:
            self._ensure_loop()
            contract = IBKRProvider.make_contract(symbol)
            self._ib.qualifyContracts(contract)
            return self._ib.reqHistoricalData(
                contract,
                endDateTime="",
                durationStr=duration,
                barSizeSetting=bar_size,
                whatToShow=what_to_show,
                useRTH=False,
                formatDate=1,
            )
        except Exception as e:
            print(f"[fetch_and_cache] {symbol} 失败: {e}", flush=True)
            return None

    @staticmethod
    def _cache_bars(bars: list[Bar], symbol: str, interval: str) -> "pd.DataFrame":
        """将 Bar 列表转为 DataFrame 并写入缓存。"""
        from newchan.convert import bars_to_df
        from newchan.cache import save_df

        df = bars_to_df(bars)
        cache_name = f"{symbol.upper()}_{interval}_raw"
        save_df(cache_name, df)
        return df

    # ------------------------------------------------------------------
    # 实时数据订阅
    # ------------------------------------------------------------------

    def subscribe_realtime(self, symbol: str, what_to_show: str = "TRADES") -> bool:
        """订阅 5 秒实时 bar。"""
        symbol = symbol.upper()
        if symbol in self._rt_subs:
            return True  # 已订阅
        if not self.connected:
            return False

        try:
            self._ensure_loop()
            contract = IBKRProvider.make_contract(symbol)
            self._ib.qualifyContracts(contract)

            self._rt_buffers[symbol] = collections.deque(maxlen=500)

            bars = self._ib.reqRealTimeBars(
                contract, barSize=5, whatToShow=what_to_show, useRTH=False,
            )
            bars.updateEvent += lambda bars_list, hasNew, sym=symbol: self._on_rt_bar(sym, bars_list, hasNew)
            self._rt_subs[symbol] = bars
            return True
        except Exception as e:
            print(f"[realtime] 订阅 {symbol} 失败: {e}", flush=True)
            return False

    def _on_rt_bar(self, symbol: str, bars_list, has_new: bool) -> None:
        """实时 bar 回调，追加到缓冲区。"""
        if not has_new or not bars_list:
            return
        b = bars_list[-1]
        # RealTimeBar 属性: time, open_, high, low, close, volume, wap, count
        ts = b.time if isinstance(b.time, datetime) else datetime.now()
        bar = Bar(
            ts=ts,
            open=float(b.open_),
            high=float(b.high),
            low=float(b.low),
            close=float(b.close),
            volume=float(b.volume) if b.volume > 0 else None,
        )
        buf = self._rt_buffers.get(symbol)
        if buf is not None:
            buf.append(bar)
            if len(buf) % 12 == 1:  # 每分钟打印一次
                print(f"[rt] {symbol} {ts} O={bar.open} H={bar.high} L={bar.low} C={bar.close} V={bar.volume} buf={len(buf)}", flush=True)

    def unsubscribe(self, symbol: str) -> None:
        """取消实时订阅。"""
        symbol = symbol.upper()
        self._cancel_rt(symbol)

    def _cancel_rt(self, symbol: str) -> None:
        handle = self._rt_subs.pop(symbol, None)
        if handle is not None:
            try:
                self._ib.cancelRealTimeBars(handle)
            except Exception:
                pass
        self._rt_buffers.pop(symbol, None)

    def get_latest_bars(self, symbol: str, since_idx: int = 0) -> tuple[list[dict], int]:
        """获取最新缓冲 bars（自 since_idx 以来）。

        Returns
        -------
        (bars_list, new_idx)
            bars_list: [{time, open, high, low, close, volume}]
            new_idx: 下次传入的 since_idx
        """
        symbol = symbol.upper()
        buf = self._rt_buffers.get(symbol)
        if buf is None:
            return [], since_idx
        all_bars = list(buf)
        new_bars = all_bars[since_idx:]
        result = []
        for b in new_bars:
            ts = b.ts
            if hasattr(ts, "tz") and ts.tzinfo is not None:
                ts = ts.replace(tzinfo=None)
            result.append({
                "time": ts.strftime("%Y-%m-%d %H:%M:%S"),
                "open": round(b.open, 6),
                "high": round(b.high, 6),
                "low": round(b.low, 6),
                "close": round(b.close, 6),
                "volume": round(b.volume, 0) if b.volume else 0,
            })
        return result, len(all_bars)

    def get_subscriptions(self) -> list[str]:
        """返回当前已订阅的品种列表。"""
        return list(self._rt_subs.keys())
