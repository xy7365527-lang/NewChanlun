#!/usr/bin/env python3
"""Databento 1min 10 年纯期货抓取（K4 顶点 + 折叠通道，全期货无 ETF）。

派生自 scripts/fetch_k4_global.py，但：
  1. **按年分段** get_range（避免单次 10yr 1min 请求过大 / 内存峰值），逐年拼接；
  2. 输出 `{symbol}_1m_databento_10y.json`（任务指定命名），列式格式与
     es_1m_databento.json 完全一致：{symbol,dates,opens,highs,lows,closes,volumes}；
  3. 连续合约 `.v.0`（成交量展期，非 .c.0——GC.c.0 仅 631 bars/月，见 fetch_k4_global 注释）。

成本（2026-06 实测 metadata.get_cost）：ES/GC/CL/ZN 10yr = **$0.00**（GLBX.MDP3 订阅覆盖）；
BRN = $0（IFEU.IMPACT 订阅，起点 2018-12）。DX 在 IFUS.IMPACT（ICE US，起点 2018-12）。

数据可用起点（metadata.get_dataset_range 实测）：
  GLBX.MDP3 ohlcv-1m 自 2010-06-06；IFEU.IMPACT / IFUS.IMPACT 自 2018-12-23。

用法：
  PYTHONPATH=src python scripts/fetch_1m_databento_10y.py es gc cl zn   # 指定
  PYTHONPATH=src python scripts/fetch_1m_databento_10y.py all
输出：analysis/data_cache/{symbol}_1m_databento_10y.json
"""

from __future__ import annotations

import json
import os
import sys
import time
from dataclasses import dataclass
from datetime import datetime, timedelta, timezone
from pathlib import Path

_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(_ROOT / "src"))

from dotenv import load_dotenv

load_dotenv(_ROOT / ".env")

import databento as db

CACHE_DIR = _ROOT / "analysis" / "data_cache"
TARGET_START = "2016-01-01"   # 10 年目标起点（CME 实际可到 2010，按需）
_GLBX = "2010-06-06"
_ICE = "2018-12-23"           # IFEU/IFUS 起点


@dataclass(frozen=True)
class Spec:
    key: str
    symbol: str
    dataset: str
    db_symbol: str
    label: str
    earliest: str
    invert: bool = False
    # 展期语义分流（实测）：.v.0 成交量展期的展期链分年切割稳健（边界无丢失，
    #   gc/cl/zn/usd6e/es/brn 分年均得到完整 2010/2018→2026 数据）；
    #   .c.0 日历展期的展期链对窗口起点敏感——1年窗口起点丢失约 5 周交割数据
    #   （DX 分年累积 794,092 条 vs 一次性 844,331 条，差 50,239 条边界数据）。
    #   故 .c.0 必须一次性拉取，不可分年。
    chunked: bool = True


SPECS: dict[str, Spec] = {
    # CME（GLBX.MDP3 订阅，$0，可至 2010）
    "es": Spec("es", "ES", "GLBX.MDP3", "ES.v.0", "标普 E-mini (CME) — P 顶点", _GLBX),
    "gc": Spec("gc", "GC", "GLBX.MDP3", "GC.v.0", "黄金 (CME) — Au 折叠通道", _GLBX),
    "cl": Spec("cl", "CL", "GLBX.MDP3", "CL.v.0", "WTI 原油 (CME) — Oil 折叠通道/C", _GLBX),
    "zn": Spec("zn", "ZN", "GLBX.MDP3", "ZN.v.0", "10年期国债 (CME) — R 顶点", _GLBX),
    # ICE Europe（IFEU 订阅，起点 2018-12）
    "brn": Spec("brn", "BRN", "IFEU.IMPACT", "BRN.v.0", "Brent 原油 (ICE Europe)", _ICE),
    # ICE US（IFUS 订阅，起点 2018-12）— 美元指数。
    # 用 DX.v.0（成交量展期）：.c.0（日历展期）在季度交割点机械滚到次活跃合约会
    #   产生数据空洞（实测 2025-09-12→11-07 有 55 天缺口；同段 .c.0 仅 3,918 bars
    #   vs .v.0 45,452 bars）——证伪了"金融指数期货无 .c.0 稀疏问题"的旧假设。
    #   成本：continuous 符号展期 resolution 免费，实际计费针对订阅内的底层 ohlcv-1m
    #   bars，.v.0 与 .c.0 计费相同（均 $0，IFUS 订阅覆盖）。旧注释"$.v.0 收费 $0.6/月"
    #   是 get_cost list-price 误读（get_cost 不反映订阅折扣）。
    "dx": Spec("dx", "DX", "IFUS.IMPACT", "DX.v.0", "美元指数 (ICE US) — M 顶点", _ICE,
               chunked=False),
    # 美元代理回退（若 DX 不可用）：6E 倒数（EUR/USD → USD/EUR），CME 免费。
    "usd6e": Spec("usd6e", "USD6E", "GLBX.MDP3", "6E.v.0",
                  "美元代理 = 6E 倒数 (CME) — M 顶点回退", _GLBX, invert=True),
}


def _client() -> db.Historical:
    key = os.environ.get("DATABENTO_API_KEY") or os.environ.get("DATABENTO_KEY")
    if not key:
        raise RuntimeError("DATABENTO_API_KEY / DATABENTO_KEY 环境变量未设置，拒绝执行")
    return db.Historical(key=key)


def _now_utc() -> datetime:
    return datetime.now(timezone.utc)


def _year_windows(start: str, end_dt: datetime) -> list[tuple[str, str]]:
    """[start, end) 按自然年切片 → [(s,e),...]（半开区间，UTC 日期串）。"""
    s = datetime.strptime(start, "%Y-%m-%d").replace(tzinfo=timezone.utc)
    out = []
    while s < end_dt:
        nxt = datetime(s.year + 1, 1, 1, tzinfo=timezone.utc)
        e = min(nxt, end_dt)
        out.append((s.strftime("%Y-%m-%d"), e.strftime("%Y-%m-%d")))
        s = nxt
    return out


def fetch_one(spec: Spec, start: str = TARGET_START) -> Path | None:
    if start < spec.earliest:
        start = spec.earliest
    client = _client()
    # end clamp 到数据集实际可用末尾：ICE 数据集（IFEU/IFUS）末尾滞后当下约 1 天，
    #   盲用 _now_utc() 会令最后窗口越界触发 422 dataset_unavailable_range（整段被跳过，
    #   丢失最近数据）。CME（GLBX）末尾即当下，clamp 无影响。
    try:
        drange = client.metadata.get_dataset_range(dataset=spec.dataset)
        ds_end = datetime.strptime(drange["end"][:10], "%Y-%m-%d").replace(
            tzinfo=timezone.utc)
    except Exception:  # noqa: BLE001
        ds_end = _now_utc()
    end_dt = min(_now_utc(), ds_end)
    out_path = CACHE_DIR / f"{spec.key}_1m_databento_10y.json"

    # 展期语义分流：.v.0 分年（控制单请求大小，展期链稳健）；
    #   .c.0 一次性（展期链对窗口起点敏感，分年会丢边界交割数据）。
    windows = (_year_windows(start, end_dt) if spec.chunked
               else [(start, end_dt.strftime("%Y-%m-%d"))])

    print(f"\n[{spec.key}] {spec.label} | {spec.dataset} {spec.db_symbol} "
          f"ohlcv-1m [{start} → {end_dt.date()}] "
          f"({'分年' if spec.chunked else '一次性'})", flush=True)

    rows: dict[str, list] = {}
    for ws, we in windows:
        for attempt in range(3):
            try:
                data = client.timeseries.get_range(
                    dataset=spec.dataset, symbols=[spec.db_symbol],
                    stype_in="continuous", schema="ohlcv-1m", start=ws, end=we,
                )
                df = data.to_df()
                break
            except Exception as e:  # noqa: BLE001
                msg = repr(e)[:140]
                if "data_start_before_available" in msg or "unavailable_range" in msg:
                    print(f"  [{ws}~{we}] 跳过（超出可用范围）：{msg}", flush=True)
                    df = None
                    break
                print(f"  [{ws}~{we}] 重试 {attempt+1}/3：{msg}", flush=True)
                time.sleep(5)
        else:
            df = None
        if df is None or df.empty:
            print(f"  [{ws}~{we}] 空", flush=True)
            continue
        df = df.sort_index()
        df = df[~df.index.duplicated(keep="last")]
        o = df["open"].astype(float).tolist()
        h = df["high"].astype(float).tolist()
        lo = df["low"].astype(float).tolist()
        c = df["close"].astype(float).tolist()
        v = df["volume"].astype("int64").tolist()
        if spec.invert:
            o = [1.0 / x for x in o]
            c = [1.0 / x for x in c]
            h, lo = [1.0 / x for x in lo], [1.0 / x for x in h]
        for i, ts in enumerate(df.index):
            rows[str(ts)] = [o[i], h[i], lo[i], c[i], v[i]]
        print(f"  [{ws}~{we}] +{len(df)} 累计 {len(rows)}", flush=True)

    if not rows:
        print(f"[{spec.key}] ✗ 无数据", flush=True)
        return None

    ordered = sorted(rows.items())
    payload = {
        "symbol": spec.symbol,
        "dates": [d for d, _ in ordered],
        "opens": [r[0] for _, r in ordered],
        "highs": [r[1] for _, r in ordered],
        "lows": [r[2] for _, r in ordered],
        "closes": [r[3] for _, r in ordered],
        "volumes": [r[4] for _, r in ordered],
    }
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    with open(out_path, "w") as f:
        json.dump(payload, f)
    size_mb = out_path.stat().st_size / 1024 / 1024
    print(f"[{spec.key}] ✓ {len(ordered)} 条 | {payload['dates'][0]} → "
          f"{payload['dates'][-1]} | {out_path.name} ({size_mb:.1f}MB)"
          + ("  [已倒数为美元]" if spec.invert else ""), flush=True)
    return out_path


def main() -> int:
    if len(sys.argv) < 2 or sys.argv[1] in ("-h", "--help"):
        print(__doc__)
        return 0
    args = [a.lower() for a in sys.argv[1:]]
    keys = list(SPECS) if args == ["all"] else args
    start = TARGET_START
    # 允许末尾传 start（YYYY-MM-DD）。
    if keys and keys[-1][:2].isdigit() and "-" in keys[-1]:
        start = keys.pop()
    summary = []
    for k in keys:
        spec = SPECS.get(k)
        if not spec:
            print(f"未知标的 {k}，可选：{list(SPECS)}")
            continue
        try:
            p = fetch_one(spec, start)
            summary.append(f"{k}: {'✓ ' + p.name if p else '✗ 无数据'}")
        except Exception as e:  # noqa: BLE001
            print(f"[{k}] ✗ {e!r}", flush=True)
            summary.append(f"{k}: ✗ {repr(e)[:80]}")
    print("\n=== 汇总 ===")
    for s in summary:
        print("  " + s)
    return 0


if __name__ == "__main__":
    sys.exit(main())
