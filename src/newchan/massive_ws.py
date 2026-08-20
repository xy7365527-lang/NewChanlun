#!/usr/bin/env python3
"""Massive 个股实时流落地（issue #1044，ADR 0023 §三·实时流接入四条）。

wss://socket.massive.com/stocks?ticker=X 的 T(rades)/Q(uotes) 事件 →
与历史同 schema 的 Parquet(ZSTD) 落地（per-symbol/per-day 分区），
断线重连 + 按 sip_timestamp 区间 REST 回补对账。

D4 #1033 四条边界落实：
1. schema 基 = REST 字段，WS 短码映射入同一 schema；WS 缺的字段置 null
   （trades 缺 correction；quotes 缺 participant_timestamp）；
2. 时间统一纳秒：WS 毫秒(t/pt/trft) ×1e6；实时行 ns 低位=0（数据源精度）；
3. 实时落盘：当日分区边写边 flush（.tmp 后缀），日终/退出转正（rename）；
4. 断线重连后按 sip_timestamp 区间从 REST 回补对账（回补 = 该日全量重拉，幂等覆盖）。

用法：
  python -m newchan.massive_ws --ticker AAPL --schemas T,Q        # 实时流落地
  python -m newchan.massive_ws --ticker AAPL --backfill-date 2026-08-14  # 单日回补
"""
from __future__ import annotations

import argparse
import asyncio
import json
import os
import ssl
import time
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path

import pyarrow as pa
import pyarrow.parquet as pq
import requests

try:
    import websockets
except ImportError:  # 沙盒/裸环境：由调用方保证依赖（本仓 .venv 已含）
    websockets = None

WS_URL = "wss://socket.massive.com/stocks"
TRADES_COLUMNS = [
    "id", "ticker", "conditions", "exchange", "participant_timestamp",
    "sip_timestamp", "trf_timestamp", "price", "sequence_number", "size",
    "tape", "trf_id", "decimal_size", "correction",
]
QUOTES_COLUMNS = [
    "ticker", "sip_timestamp", "participant_timestamp", "sequence_number",
    "tape", "ask_exchange", "ask_price", "ask_size", "bid_exchange",
    "bid_price", "bid_size", "conditions", "indicators",
]
# round-lot 口径：WS Q 事件 bs/as 为 round lots（×100=股数）。
# flat-files quotes 文档：2025-11-03 起 SEC MDI 直报股数——若 WS 侧同步切换，
# 此 ×100 须按标定票（#1051 系）订正；本模块先按文档自述 round lots 落地。
ROUND_LOT = 100
_STRING_COLS = frozenset({"id", "ticker", "decimal_size"})
_LIST_COLS = frozenset({"conditions", "indicators"})
_FLOAT_COLS = frozenset({"price", "ask_price", "bid_price"})


def env_key() -> str:
    key = os.environ.get("MASSIVE_API_KEY")
    if not key:
        envf = Path(__file__).resolve().parents[3] / ".env"
        if envf.exists():
            for line in envf.read_text().splitlines():
                if line.startswith("MASSIVE_API_KEY="):
                    key = line.split("=", 1)[1].strip()
    if not key:
        raise RuntimeError("MASSIVE_API_KEY 未找到（env 或仓根 .env）")
    return key


def map_trade(ev: dict) -> dict:
    """WS T 事件 → REST trades schema（D4 #1033 映射表）。缺列置 null。"""
    return {
        "id": ev.get("i"),
        "ticker": ev.get("sym"),
        "conditions": ev.get("c"),
        "exchange": ev.get("x"),
        "participant_timestamp": ev.get("pt") * 1_000_000 if ev.get("pt") else None,
        "sip_timestamp": ev.get("t") * 1_000_000 if ev.get("t") else None,
        "trf_timestamp": ev.get("trft") * 1_000_000 if ev.get("trft") else None,
        "price": ev.get("p"),
        "sequence_number": ev.get("q"),
        "size": ev.get("s"),
        "tape": ev.get("z"),
        "trf_id": ev.get("trfi"),
        "decimal_size": ev.get("ds"),
        "correction": None,  # WS 无此字段
    }


def map_quote(ev: dict) -> dict:
    """WS Q 事件 → REST quotes schema。bs/as 按 round lots ×100；缺列置 null。"""
    return {
        "ticker": ev.get("sym"),
        "sip_timestamp": ev.get("t") * 1_000_000 if ev.get("t") else None,
        "participant_timestamp": None,  # WS Q 事件无 pt
        "sequence_number": ev.get("q"),
        "tape": ev.get("z"),
        "ask_exchange": ev.get("ax"),
        "ask_price": ev.get("ap"),
        "ask_size": ev.get("as") * ROUND_LOT if ev.get("as") is not None else None,
        "bid_exchange": ev.get("bx"),
        "bid_price": ev.get("bp"),
        "bid_size": ev.get("bs") * ROUND_LOT if ev.get("bs") is not None else None,
        "conditions": [ev["c"]] if ev.get("c") is not None else None,
        "indicators": ev.get("i"),
    }


def ns_date(ns: int) -> str:
    return datetime.fromtimestamp(ns / 1e9, tz=timezone.utc).strftime("%Y-%m-%d")


def rest_fetch_day(ticker: str, date: str, key: str) -> list[dict]:
    """REST /v3/trades 单日全量分页（回补用）。"""
    rows: list[dict] = []
    nxt = (
        f"https://api.massive.com/v3/trades/{ticker}"
        f"?timestamp={date}&limit=50000"
    )
    headers = {"Authorization": f"Bearer {key}"}
    while nxt:
        r = requests.get(nxt, headers=headers, timeout=30)
        r.raise_for_status()
        j = r.json()
        for it in j.get("results", []):
            rows.append({
                "id": it.get("id"),
                "ticker": ticker,
                "conditions": it.get("conditions"),
                "exchange": it.get("exchange"),
                "participant_timestamp": it.get("participant_timestamp"),
                "sip_timestamp": it.get("sip_timestamp"),
                "trf_timestamp": it.get("trf_timestamp"),
                "price": it.get("price"),
                "sequence_number": it.get("sequence_number"),
                "size": it.get("size"),
                "tape": it.get("tape"),
                "trf_id": it.get("trf_id"),
                "decimal_size": it.get("decimal_size"),
                "correction": it.get("correction"),
            })
        nxt = j.get("next_url")
    return rows


@dataclass
class DayWriter:
    """当日分区缓冲：边写边 flush 到 .tmp，finalize 转正。"""

    ticker: str
    root: Path
    schemas: tuple[str, ...] = ("T",)
    _date: str | None = None
    _writers: dict = field(default_factory=dict)
    _paths: dict = field(default_factory=dict)

    def _partition(self, date: str, schema: str) -> Path:
        name = {"T": "trades.parquet", "Q": "quotes.parquet"}[schema]
        return self.root / self.ticker / f"dt={date}" / name

    def _abandon_open_writers(self) -> None:
        """关闭未转正的 .tmp，不覆盖已落盘分区（断线 REST 回补前调用）。"""
        for w in self._writers.values():
            if w is not None:
                w.close()
        for tmp in self._paths.values():
            if tmp is not None and tmp.exists():
                tmp.unlink()
        self._writers = {}
        self._paths = {}

    def _promote_tmp(self, tmp: Path, final: Path) -> None:
        """将当日 .tmp 转正。若 REST 回补已写入 final，则拼接而不是覆盖。"""
        if final.exists():
            merged = pa.concat_tables([pq.read_table(final), pq.read_table(tmp)])
            merge_tmp = final.with_name(final.name + ".merge")
            pq.write_table(merged, merge_tmp, compression="zstd")
            merge_tmp.replace(final)
            tmp.unlink(missing_ok=True)
        else:
            tmp.replace(final)

    def _finalize(self, date: str) -> None:
        for schema in list(self._writers):
            w = self._writers[schema]
            if w is not None:
                w.close()
            tmp = self._paths.get(schema)
            if tmp is not None and tmp.exists():
                self._promote_tmp(tmp, self._partition(date, schema))
        self._writers = {}
        self._paths = {}

    def overlay_rest(self, rows: list[dict], date: str, schema: str = "T") -> None:
        """断线回补：丢弃当日未完成 WS .tmp，用 REST 全日重拉幂等覆盖分区。

        回补前未转正的 live 行视为 REST 子集（ADR 0023 §三「该日全量重拉」）。
        回补后新的 WS 行写入新的 .tmp，finalize 时与 REST 分区拼接。
        """
        if self._date and self._date != date:
            self._finalize(self._date)
        self._abandon_open_writers()
        self._date = date
        if not rows:
            return
        part = self._partition(date, schema)
        part.parent.mkdir(parents=True, exist_ok=True)
        tmp = part.parent / f".{part.name}.rest.{os.getpid()}"
        pq.write_table(
            pa.Table.from_pylist(rows, schema=_schema_for(schema)),
            tmp, compression="zstd",
        )
        tmp.replace(part)

    def _roll(self, date: str) -> None:
        if self._date is None:
            self._date = date
            return
        if date != self._date:
            self._finalize(self._date)
            self._date = date

    def write(self, row: dict, schema: str) -> None:
        sip = row.get("sip_timestamp")
        if not sip:
            return
        date = ns_date(sip)
        self._roll(date)
        part = self._partition(date, schema)
        part.parent.mkdir(parents=True, exist_ok=True)
        if schema not in self._writers:
            tmp = part.parent / (part.name + ".tmp")
            self._writers[schema] = pq.ParquetWriter(
                tmp, pa.Table.from_pylist([row], schema=_schema_for(schema)).schema
            )
            self._paths[schema] = tmp
        self._writers[schema].write_table(
            pa.Table.from_pylist([row], schema=_schema_for(schema))
        )

    def close(self) -> None:
        if self._date:
            self._finalize(self._date)
        self._date = None


def _pa_type(col: str) -> pa.DataType:
    if col in _STRING_COLS:
        return pa.string()
    if col in _LIST_COLS:
        return pa.list_(pa.int64())
    if col in _FLOAT_COLS:
        return pa.float64()
    return pa.int64()


def _schema_for(schema: str) -> pa.Schema:
    cols = TRADES_COLUMNS if schema == "T" else QUOTES_COLUMNS
    return pa.schema([(c, _pa_type(c)) for c in cols])


async def run_live(ticker: str, root: Path, schemas: str, key: str) -> None:
    if websockets is None:
        raise RuntimeError("websockets 未安装")
    writer = DayWriter(ticker=ticker, root=root, schemas=tuple(schemas.split(",")))
    backfilled: set[str] = set()
    while True:
        try:
            async with websockets.connect(
                f"{WS_URL}?ticker={ticker}",
                ssl=ssl.create_default_context(),
                open_timeout=15,
            ) as sock:
                await sock.send(json.dumps({"action": "auth", "params": key}))
                for schema in schemas.split(","):
                    await sock.send(json.dumps({
                        "action": "subscribe",
                        "params": f"{schema}.{ticker}",
                    }))
                async for msg in sock:
                    for ev in json.loads(msg):
                        etype = ev.get("ev")
                        if etype == "T":
                            writer.write(map_trade(ev), "T")
                        elif etype == "Q":
                            writer.write(map_quote(ev), "Q")
        except Exception:
            # 断线：回补最近未完成日（含断线当日），再重连。
            # REST 失败不得杀死直播环——网络闪断时 REST 同样可能超时。
            today = datetime.now(timezone.utc).strftime("%Y-%m-%d")
            if today not in backfilled:
                try:
                    rows = rest_fetch_day(ticker, today, key)
                    writer.overlay_rest(rows, today, "T")
                    backfilled.add(today)
                except Exception:
                    pass
            await asyncio.sleep(5)


def run_backfill(ticker: str, date: str, root: Path, key: str) -> None:
    rows = rest_fetch_day(ticker, date, key)
    part = root / ticker / f"dt={date}" / "trades.parquet"
    part.parent.mkdir(parents=True, exist_ok=True)
    pq.write_table(
        pa.Table.from_pylist(rows, schema=_schema_for("T")),
        part, compression="zstd",
    )
    print(f"backfilled {ticker} {date}: {len(rows)} rows -> {part}")


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--ticker", required=True)
    ap.add_argument("--schemas", default="T")
    ap.add_argument("--backfill-date", default=None)
    ap.add_argument("--root", default=str(Path("analysis/data_cache/massive_tick")))
    args = ap.parse_args()
    key = env_key()
    root = Path(args.root)
    if args.backfill_date:
        run_backfill(args.ticker, args.backfill_date, root, key)
        return
    asyncio.run(run_live(args.ticker, root, args.schemas, key))


if __name__ == "__main__":
    main()
