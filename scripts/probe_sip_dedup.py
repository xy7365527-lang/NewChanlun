#!/usr/bin/env python3
"""SIP 去重/条件码过滤探针（issue #1041 交付件二：「有数才有权定稿」的读数器）。

对 Massive 原样落盘（#1040：`analysis/data_cache/massive_tick/{TICKER}/dt={YYYY-MM-DD}/trades.parquet`）
抽 5 个交易日 × 3 标的，统计：

1. **per-venue 重复率**：各 exchange 码行数占比 + TRF 打印占比 + TRF 打印里「与主所打印
   同笔」的比例（= 真重复 vs 独一场外成交量）；
2. **条件码占比**：各 condition 码计数与占比、各 correction 取值计数与占比；
3. **消歧键对拍**：候选键 `(sip_timestamp, sequence_number)` 与 `participant_timestamp`
   各自的分组去重读数（唯一组数 / 总行数 / 组内价格一致性）——item 3 定稿的数据依据。

用法：
    uv run python scripts/probe_sip_dedup.py [--symbols AAPL,COST,NVDA] [--days 5] [--out <report.json>]

数据未落地（#1040 未产 parquet）时以明确报错退出（不静默、不编数）。
"""

from __future__ import annotations

import argparse
import json
import sys
from collections import Counter
from pathlib import Path

import pandas as pd

from newchan.sip_consolidate import (
    KEY_PARTICIPANT,
    KEY_SIP_SEQ,
    conditions_series,
)

DEFAULT_ROOT = Path("analysis/data_cache/massive_tick")
DEFAULT_SYMBOLS = ["AAPL", "COST", "NVDA"]

REQUIRED_COLUMNS = [
    "id",
    "ticker",
    "conditions",
    "exchange",
    "participant_timestamp",
    "sip_timestamp",
    "price",
    "sequence_number",
    "size",
    "tape",
    "trf_id",
]


def _collect_symbol_days(root: Path) -> dict[str, list[tuple[str, Path]]]:
    """扫描数据目录：{ticker: [(date, parquet_path), ...]}（日期升序）。"""
    found: dict[str, list[tuple[str, Path]]] = {}
    if not root.exists():
        return found
    for ticker_dir in sorted(root.iterdir()):
        if not ticker_dir.is_dir():
            continue
        days: list[tuple[str, Path]] = []
        for dt_dir in sorted(ticker_dir.glob("dt=*")):
            if not dt_dir.is_dir():
                continue
            p = dt_dir / "trades.parquet"
            if p.exists():
                days.append((dt_dir.name.removeprefix("dt="), p))
        if days:
            found[ticker_dir.name] = days
    return found


def read_day(path: Path) -> pd.DataFrame:
    """读单日 parquet 并核 schema 完整性（fail-loud，缺字段即报错）。"""
    df = pd.read_parquet(path)
    missing = [c for c in REQUIRED_COLUMNS if c not in df.columns]
    if missing:
        raise ValueError(f"{path} 缺字段: {missing}")
    return df


def _dup_reading(df: pd.DataFrame, key: tuple[str, ...]) -> dict:
    """单键去重读数：唯一组数、总行数、组内价格是否一致（对拍键是否误并不同价成交）。"""
    sub = df[list(key) + ["price"]]
    grouped = sub.groupby(list(key), dropna=False)
    n_groups = grouped.ngroups
    n_rows = len(df)
    # 组内价格一致率：组内价格唯一值的组占比。同笔多报应当价格一致；键若误并不同价
    # 成交（键太粗），该比例会明显 < 1。
    price_agree = grouped["price"].nunique().le(1).sum()
    return {
        "key": list(key),
        "n_rows": int(n_rows),
        "n_unique_keys": int(n_groups),
        "dup_rate": round(1.0 - n_groups / n_rows, 6) if n_rows else 0.0,
        "price_agree_groups": int(price_agree),
        "price_agree_rate": round(price_agree / n_groups, 6) if n_groups else 1.0,
    }


def probe_day(path: Path, ticker: str, date: str) -> dict:
    """单标的单日的全部读数。"""
    df = read_day(path)
    df = df.copy()
    df["__conds"] = conditions_series(df)
    trf_id_num = pd.to_numeric(df["trf_id"], errors="coerce")
    df["__is_trf"] = trf_id_num.notna() & (trf_id_num != 0)

    n = len(df)
    venue_counts = (
        df.groupby("exchange").size().sort_values(ascending=False).to_dict()
    )
    n_trf = int(df["__is_trf"].sum())

    # TRF 打印里「与主所打印同笔」的比例：participant_timestamp 在非 TRF 行里出现过的
    # 即视为同笔重复（这是 item 3 的 participant_timestamp 候选在 TRF↔主所匹配面的用法）。
    exchange_pts = set(df.loc[~df["__is_trf"], "participant_timestamp"].dropna())
    trf_pts = df.loc[df["__is_trf"], "participant_timestamp"].dropna()
    trf_matched = int(trf_pts.isin(exchange_pts).sum()) if n_trf else 0

    # 条件码占比（只列前 15 高频）。
    cond_counter: Counter = Counter()
    for s in df["__conds"]:
        for c in s:
            cond_counter[c] += 1
    cond_total = sum(cond_counter.values())
    top_conditions = [
        {"code": int(c), "count": cnt, "ratio": round(cnt / cond_total, 6) if cond_total else 0.0}
        for c, cnt in cond_counter.most_common(15)
    ]

    # correction 取值分布（可空列）。
    corr_counts: dict = {}
    if "correction" in df.columns:
        corr_series = pd.to_numeric(df["correction"], errors="coerce")
        corr_counts = {
            str(k): int(v) for k, v in corr_series.value_counts(dropna=False).items()
        }

    return {
        "ticker": ticker,
        "date": date,
        "n_rows": int(n),
        "venue_counts": {str(k): int(v) for k, v in venue_counts.items()},
        "n_trf": n_trf,
        "trf_ratio": round(n_trf / n, 6) if n else 0.0,
        "trf_matched_to_exchange": trf_matched,
        "trf_matched_ratio": round(trf_matched / n_trf, 6) if n_trf else 0.0,
        "top_conditions": top_conditions,
        "correction_counts": corr_counts,
        "dup_by_key": [
            _dup_reading(df, KEY_SIP_SEQ),
            _dup_reading(df, KEY_PARTICIPANT),
        ],
    }


def probe(root: Path, symbols: list[str], days: int) -> list[dict]:
    """抽 days 个交易日 × symbols 个标的（优先最近 days 日）。"""
    found = _collect_symbol_days(root)
    if not found:
        raise SystemExit(
            f"无 Massive 原样数据：{root} 不存在或为空。\n"
            "请先完成 #1040（scripts/fetch_massive_tick.py 落盘 per-symbol/per-day 分区），"
            "本探针不编数。"
        )
    records: list[dict] = []
    for sym in symbols:
        days_list = found.get(sym)
        if not days_list:
            print(f"[skip] {sym} 无数据分区", file=sys.stderr)
            continue
        picked = days_list[-days:]  # 升序，取最近 days 日
        for date, path in picked:
            records.append(probe_day(path, sym, date))
    return records


def summarize(records: list[dict]) -> dict:
    """跨标的/跨日聚合（去重读数与 TRF 占比取总行数加权）。"""
    if not records:
        return {}
    total_rows = sum(r["n_rows"] for r in records)
    total_trf = sum(r["n_trf"] for r in records)
    total_trf_matched = sum(r["trf_matched_to_exchange"] for r in records)

    agg_dup = {}
    for key_name, key in (("sip_seq", KEY_SIP_SEQ), ("participant", KEY_PARTICIPANT)):
        sub_rows = 0
        sub_keys = 0
        for r in records:
            for d in r["dup_by_key"]:
                if tuple(d["key"]) == key:
                    sub_rows += d["n_rows"]
                    sub_keys += d["n_unique_keys"]
        agg_dup[key_name] = {
            "key": list(key),
            "n_rows": int(sub_rows),
            "n_unique_keys": int(sub_keys),
            "dup_rate": round(1.0 - sub_keys / sub_rows, 6) if sub_rows else 0.0,
        }

    return {
        "n_symbol_days": len(records),
        "n_rows": int(total_rows),
        "n_trf": int(total_trf),
        "trf_ratio": round(total_trf / total_rows, 6) if total_rows else 0.0,
        "trf_matched_ratio": (
            round(total_trf_matched / total_trf, 6) if total_trf else 0.0
        ),
        "dup_by_key": agg_dup,
    }


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--root", type=Path, default=DEFAULT_ROOT)
    ap.add_argument("--symbols", type=str, default=",".join(DEFAULT_SYMBOLS))
    ap.add_argument("--days", type=int, default=5)
    ap.add_argument("--out", type=Path, default=None)
    args = ap.parse_args()

    symbols = [s.strip().upper() for s in args.symbols.split(",") if s.strip()]
    records = probe(args.root, symbols, args.days)
    report = {"summary": summarize(records), "records": records}

    print(json.dumps(report, indent=2, ensure_ascii=False))
    if args.out:
        args.out.write_text(json.dumps(report, indent=2, ensure_ascii=False), encoding="utf-8")
        print(f"\n报告已写 {args.out}")


if __name__ == "__main__":
    main()
