#!/usr/bin/env python3
"""consolidated 正本产出步（issue #1048，ADR 0023 §四-1 接线）。

消费 #1040 原样落盘（`analysis/data_cache/massive_tick/{TICKER}/dt={YYYY-MM-DD}/trades.parquet`），
跑 `newchan.sip_consolidate.consolidate()`（`filter_trades` → `drop_trf_prints` →
`dedup_by_key` 三判据流水），把 consolidated 正本落盘为 Parquet(ZSTD)：

    analysis/data_cache/massive_tick_consolidated/{TICKER}/dt={YYYY-MM-DD}/trades.parquet

- **可复现**：`consolidate()` 确定（稳定排序 + keep first + 恢复原行序）；正本分区已存在且
  行数>0 则跳过（幂等重跑）；落盘原子写（临时文件 + rename）。
- **消歧键**（#1041 待定稿项）：产出正本时 `--key` **必选**（`sip_seq` 或 `participant`），
  不替操作者选键。`--calibrate-key` 对采样分区跑双键读数（重复率 + 组内价格一致率）并出
  推荐，读数落 `_dedup_key_calibration.json`；最终定稿（写死默认）须实测数据（见
  `analysis/sip_consolidated_pipeline_raw_disposition.md`）。
- **raw staging → transient 处置**（#1048 第二件）：`--retire-raw` 只把「正本已产出且过
  行数校验」的原样分区**移动**到 transient 区
  （`analysis/data_cache/massive_tick_raw_transient/`），不删除——删除留在保留期后由人审
  （策略见 `analysis/sip_consolidated_pipeline_raw_disposition.md`）。

用法：
    # 计划（只打印会产出哪些分区，不读数据不改盘）
    uv run python scripts/consolidate_massive_tick.py --plan
    # 十标全量产出正本（--key 必选）
    uv run python scripts/consolidate_massive_tick.py --key sip_seq
    # 单标的 + participant_timestamp 键
    uv run python scripts/consolidate_massive_tick.py --symbols COST --key participant
    # 消歧键标定（采样 5 日 × 十标，双键读数 + 推荐）
    uv run python scripts/consolidate_massive_tick.py --calibrate-key --days 5
    # raw → transient（只移动已产出正本且校验通过的分区）
    uv run python scripts/consolidate_massive_tick.py --retire-raw
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import shutil
import sys
import time
from collections.abc import Sequence
from pathlib import Path

import pandas as pd

from newchan.sip_consolidate import (
    KEY_PARTICIPANT,
    KEY_SIP_SEQ,
    consolidate,
    drop_trf_prints,
    filter_trades,
)

try:
    import pyarrow.parquet as pq
except ImportError:  # pragma: no cover - pyarrow 是 pyproject 显式依赖
    pq = None

RAW_ROOT = Path("analysis/data_cache/massive_tick")
CONSOLIDATED_ROOT = Path("analysis/data_cache/massive_tick_consolidated")
RAW_TRANSIENT_ROOT = Path("analysis/data_cache/massive_tick_raw_transient")

DEFAULT_SYMBOLS = [
    "AAPL", "AMZN", "COST", "GOOGL", "LLY",
    "META", "MSFT", "NFLX", "NVDA", "TSLA",
]

# CLI 键名 → 消歧键列（#1041 候选二选一）。
KEY_CHOICES: dict[str, tuple[str, ...]] = {
    "sip_seq": KEY_SIP_SEQ,
    "participant": KEY_PARTICIPANT,
}

# 消歧键标定阈值（#1041 §4 定稿判据的机械版，见 analysis/sip_consolidated_pipeline_raw_disposition.md §3）。
PRICE_AGREE_FLOOR = 0.999
DUP_RATE_GAP = 0.05


def log(msg: str) -> None:
    print(f"[{dt.datetime.now():%Y-%m-%d %H:%M:%S}] {msg}", flush=True)


def collect_partitions(root: Path) -> dict[str, list[tuple[str, Path]]]:
    """扫描原样分区：{ticker: [(date_str, path), ...]}（日期升序）。"""
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


def partition_done(path: Path) -> bool:
    """断点续跑判据：分区存在且行数>0 视为已完成；损坏/空分区视为未完成。"""
    if not path.exists():
        return False
    if pq is None:
        return False
    try:
        return pq.read_metadata(str(path)).num_rows > 0
    except Exception:
        return False


def consolidated_path(root: Path, ticker: str, date_str: str) -> Path:
    return Path(root) / ticker / f"dt={date_str}" / "trades.parquet"


def _validate_key_columns(df: pd.DataFrame, key: Sequence[str]) -> None:
    """消歧键列必须存在且无空值（键是判据承重件，缺失/空值不可静默跳过）。"""
    missing = [c for c in key if c not in df.columns]
    if missing:
        raise ValueError(f"消歧键列缺失: {missing}")
    for c in key:
        if df[c].isna().any():
            raise ValueError(f"消歧键列含空值: {c!r}")


def write_partition(root: Path, ticker: str, date_str: str, df: pd.DataFrame) -> Path | None:
    """落盘单日正本分区（原子写：临时文件 + rename）。空 DataFrame 返回 None。"""
    if df is None or len(df) == 0:
        return None
    day_dir = Path(root) / ticker / f"dt={date_str}"
    day_dir.mkdir(parents=True, exist_ok=True)
    tmp = day_dir / f".trades.parquet.tmp.{os.getpid()}.{int(time.time() * 1000)}"
    final = day_dir / "trades.parquet"
    try:
        df.to_parquet(tmp, engine="pyarrow", compression="zstd", index=False)
        tmp.replace(final)
    except Exception:
        if tmp.exists():
            tmp.unlink(missing_ok=True)
        raise
    return final


def consolidate_partition(
    raw_path: Path, out_root: Path, ticker: str, date_str: str, key: Sequence[str]
) -> dict:
    """单分区：原样读入 → 校验键列 → consolidate → 原子落正本。返回统计。"""
    df = pd.read_parquet(raw_path)
    _validate_key_columns(df, key)
    out, stats = consolidate(df, key)
    write_partition(out_root, ticker, date_str, out)
    return stats


def key_readings(df: pd.DataFrame, key: Sequence[str]) -> dict:
    """单键去重读数（#1041 probe 同口径）：唯一组数/总行数/重复率/组内价格一致率。

    与 `consolidate()` 的 dedup 层一致，读数在 filter + venue 之后的帧上取（该帧才是
    dedup 的实际输入）。
    """
    sub = df[list(key) + ["price"]]
    grouped = sub.groupby(list(key), dropna=False)
    n_groups = grouped.ngroups
    n_rows = len(df)
    price_agree = grouped["price"].nunique().le(1).sum()
    return {
        "key": list(key),
        "n_rows": int(n_rows),
        "n_unique_keys": int(n_groups),
        "dup_rate": round(1.0 - n_groups / n_rows, 6) if n_rows else 0.0,
        "price_agree_groups": int(price_agree),
        "price_agree_rate": round(price_agree / n_groups, 6) if n_groups else 1.0,
    }


def _summarize_readings(records: list[dict]) -> dict:
    """跨分区聚合双键读数（行数加权）。"""
    agg: dict[str, dict] = {}
    for key_name, key in (("sip_seq", KEY_SIP_SEQ), ("participant", KEY_PARTICIPANT)):
        sub_rows = sub_keys = sub_agree = 0
        for r in records:
            for d in r["dup_by_key"]:
                if tuple(d["key"]) == key:
                    sub_rows += d["n_rows"]
                    sub_keys += d["n_unique_keys"]
                    sub_agree += d["price_agree_groups"]
        agg[key_name] = {
            "key": list(key),
            "n_rows": int(sub_rows),
            "n_unique_keys": int(sub_keys),
            "dup_rate": round(1.0 - sub_keys / sub_rows, 6) if sub_rows else 0.0,
            "price_agree_groups": int(sub_agree),
            "price_agree_rate": round(sub_agree / sub_keys, 6) if sub_keys else 1.0,
        }
    return agg


def recommend_key(summary: dict) -> tuple[Sequence[str] | None, str]:
    """消歧键标定决策规则（#1041 §4 定稿判据的机械版，最终定稿由编排层裁）。

    1. 两键都要有读数（缺样本 → 不定稿）；
    2. `price_agree_rate < 0.999` 的键判「误并不同价成交」，出局；
    3. 恰一个键合格 → 推荐它；
    4. 两键都合格：
       - 重复率差异显著（> 0.05）→ 取重复率更高者（同笔多报折叠更充分）；
       - 差异不显著 → 取 sip_seq（#1041 §4 平手条款：含 sequence_number 唯一性更强）。
    """
    a = summary.get("sip_seq")
    b = summary.get("participant")
    if not a or not b or not a["n_rows"] or not b["n_rows"]:
        return None, "读数不足（缺样本），不能定稿"
    ea = a["price_agree_rate"] >= PRICE_AGREE_FLOOR
    eb = b["price_agree_rate"] >= PRICE_AGREE_FLOOR
    if not ea and not eb:
        return None, "两键均误并不同价成交（price_agree_rate < 0.999），不能定稿"
    if ea and not eb:
        return KEY_SIP_SEQ, "participant 键误并不同价成交，sip_seq 唯一合格"
    if eb and not ea:
        return KEY_PARTICIPANT, "sip_seq 键误并不同价成交，participant 唯一合格"
    if abs(a["dup_rate"] - b["dup_rate"]) > DUP_RATE_GAP:
        winner = KEY_PARTICIPANT if b["dup_rate"] > a["dup_rate"] else KEY_SIP_SEQ
        return winner, "两键均合格且重复率差异显著，取重复率更高者"
    return KEY_SIP_SEQ, "两键读数差异不显著，取 sip_seq（#1041 §4 平手条款）"


def calibrate(raw_root: Path, symbols: list[str], days: int) -> dict:
    """消歧键标定：采样每标的最近 days 日，双键读数 + 推荐（#1048 第三件）。"""
    found = collect_partitions(raw_root)
    if not found:
        return {"error": f"无原样数据：{raw_root} 不存在或为空，标定不编数"}
    records: list[dict] = []
    for sym in symbols:
        days_list = found.get(sym)
        if not days_list:
            log(f"[calibrate] {sym} 无数据分区，跳过")
            continue
        for date, path in days_list[-days:]:
            try:
                df = pd.read_parquet(path)
                f, _ = filter_trades(df)
                v, _ = drop_trf_prints(f)
            except Exception as e:
                log(f"[calibrate] 读 {path} 失败：{e}")
                continue
            records.append({
                "ticker": sym,
                "date": date,
                "dup_by_key": [
                    key_readings(v, KEY_SIP_SEQ),
                    key_readings(v, KEY_PARTICIPANT),
                ],
            })
    summary = _summarize_readings(records)
    rec_key, reason = recommend_key(summary)
    return {
        "n_partitions": len(records),
        "summary": summary,
        "recommended_key": list(rec_key) if rec_key else None,
        "recommendation_reason": reason,
        "records": records,
    }


def verify_partition(raw_path: Path, out_path: Path) -> tuple[bool, str]:
    """raw → transient 前的最小校验闸：正本存在、非空、行数 ≤ 原样（不超原样）。"""
    if not partition_done(out_path):
        return False, "正本分区缺失/为空"
    try:
        raw_rows = pq.read_metadata(str(raw_path)).num_rows
        out_rows = pq.read_metadata(str(out_path)).num_rows
    except Exception as e:
        return False, f"行数读取失败: {e}"
    if not (0 < out_rows <= raw_rows):
        return False, f"行数校验失败（out={out_rows} raw={raw_rows}）"
    return True, f"ok（out={out_rows} raw={raw_rows}）"


def move_to_transient(raw_path: Path, transient_root: Path, ticker: str, date_str: str) -> Path:
    """移动原样分区到 transient 区（不删除；目标已存在则报错，不覆盖）。"""
    dest = Path(transient_root) / ticker / f"dt={date_str}" / "trades.parquet"
    dest.parent.mkdir(parents=True, exist_ok=True)
    if dest.exists():
        raise FileExistsError(f"transient 目标已存在，拒绝覆盖：{dest}")
    shutil.move(str(raw_path), str(dest))
    return dest


def consolidate_all(
    raw_root: Path,
    out_root: Path,
    symbols: list[str],
    key: Sequence[str],
) -> dict:
    """十标全量（或指定标的）：原样 → consolidate → 正本，断点续跑。返回合计统计。"""
    found = collect_partitions(raw_root)
    totals = {"consolidated": 0, "skipped": 0, "failed": 0, "raw_rows": 0, "out_rows": 0}
    for sym in symbols:
        days_list = found.get(sym)
        if not days_list:
            log(f"[{sym}] 无原样分区，跳过")
            continue
        n_cons = n_skip = n_fail = 0
        for date, raw_path in days_list:
            out_path = consolidated_path(out_root, sym, date)
            if partition_done(out_path):
                n_skip += 1
                continue
            try:
                stats = consolidate_partition(raw_path, out_root, sym, date, key)
            except Exception as e:
                n_fail += 1
                log(f"[{sym} {date}] 失败（跳过，下次续跑重试）：{e}")
                continue
            n_cons += 1
            totals["raw_rows"] += stats["consolidate_in"]
            totals["out_rows"] += stats["consolidate_out"]
            log(
                f"[{sym} {date}] raw={stats['consolidate_in']} → "
                f"正本={stats['consolidate_out']}（重复率 {stats['duplicate_rate']:.4f}）"
            )
        log(f"[{sym}] 产出 {n_cons} / 跳过 {n_skip} / 失败 {n_fail}")
        totals["consolidated"] += n_cons
        totals["skipped"] += n_skip
        totals["failed"] += n_fail
    return totals


def retire_raw(
    raw_root: Path,
    out_root: Path,
    transient_root: Path,
    symbols: list[str],
) -> dict:
    """raw → transient：只移动「正本已产出且过行数校验」的原样分区。"""
    found = collect_partitions(raw_root)
    totals = {"moved": 0, "skipped_verify": 0, "failed": 0}
    for sym in symbols:
        days_list = found.get(sym)
        if not days_list:
            continue
        for date, raw_path in days_list:
            out_path = consolidated_path(out_root, sym, date)
            ok, _ = verify_partition(raw_path, out_path)
            if not ok:
                totals["skipped_verify"] += 1
                continue
            try:
                move_to_transient(raw_path, transient_root, sym, date)
                totals["moved"] += 1
            except Exception as e:
                totals["failed"] += 1
                log(f"[{sym} {date}] 移动失败：{e}")
    return totals


def plan(raw_root: Path, out_root: Path, symbols: list[str]) -> None:
    """只读计划：每个标的的原样分区数与正本待产出数。"""
    found = collect_partitions(raw_root)
    print(f"计划（raw_root={raw_root} out_root={out_root}）")
    for sym in symbols:
        days_list = found.get(sym, [])
        pending = sum(
            1 for date, _ in days_list
            if not partition_done(consolidated_path(out_root, sym, date))
        )
        print(f"{sym}: 原样分区 {len(days_list)} 天，正本待产出 {pending} 天")


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(
        description="consolidated 正本产出步（#1048，ADR 0023 §四-1 接线）",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    ap.add_argument("--symbols", default=",".join(DEFAULT_SYMBOLS),
                    help=f"逗号分隔标的（默认十标：{','.join(DEFAULT_SYMBOLS)}）")
    ap.add_argument("--root", type=Path, default=RAW_ROOT, help="原样落盘根目录")
    ap.add_argument("--out-root", type=Path, default=CONSOLIDATED_ROOT, help="正本落盘根目录")
    ap.add_argument("--transient-root", type=Path, default=RAW_TRANSIENT_ROOT,
                    help="raw transient 区根目录")
    ap.add_argument("--key", choices=sorted(KEY_CHOICES), default=None,
                    help="消歧键（产出正本时必选：sip_seq 或 participant；定稿待实测）")
    ap.add_argument("--days", type=int, default=5, help="--calibrate-key 每标的采样天数")
    ap.add_argument("--plan", action="store_true", help="只打印计划（不读数据、不改盘）")
    ap.add_argument("--calibrate-key", action="store_true",
                    help="消歧键标定：双键读数 + 推荐，读数落 _dedup_key_calibration.json")
    ap.add_argument("--retire-raw", action="store_true",
                    help="raw → transient：移动已产出正本且校验通过的原样分区（不删除）")
    args = ap.parse_args(argv)

    symbols = [s.strip().upper() for s in args.symbols.split(",") if s.strip()]

    modes = sum([args.plan, args.calibrate_key, args.retire_raw])
    if modes > 1:
        raise SystemExit("--plan / --calibrate-key / --retire-raw 互斥，一次只跑一个")

    if args.plan:
        plan(args.root, args.out_root, symbols)
        return 0

    if args.calibrate_key:
        report = calibrate(args.root, symbols, args.days)
        if "error" in report:
            log(report["error"])
            return 1
        args.out_root.mkdir(parents=True, exist_ok=True)
        cal_path = args.out_root / "_dedup_key_calibration.json"
        cal_path.write_text(
            json.dumps(report, indent=2, ensure_ascii=False), encoding="utf-8"
        )
        print(json.dumps(
            {k: v for k, v in report.items() if k != "records"},
            indent=2, ensure_ascii=False,
        ))
        log(f"标定读数已写 {cal_path}（含逐分区明细）")
        return 0

    if args.retire_raw:
        totals = retire_raw(args.root, args.out_root, args.transient_root, symbols)
        log(
            f"retire 完成：移动 {totals['moved']} / 校验未过保留 {totals['skipped_verify']} / "
            f"失败 {totals['failed']}"
        )
        return 0

    if args.key is None:
        raise SystemExit(
            "产出正本须显式指定 --key（sip_seq|participant）；消歧键定稿待实测，不替操作者选键"
        )
    key = KEY_CHOICES[args.key]

    if not collect_partitions(args.root):
        log(
            f"无原样数据：{args.root} 不存在或为空。\n"
            "请先完成 #1040（scripts/fetch_massive_tick.py 落盘 per-symbol/per-day 分区），"
            "本脚本不编数。"
        )
        return 1

    totals = consolidate_all(args.root, args.out_root, symbols, key)
    log(
        f"全部完成：产出 {totals['consolidated']} / 跳过 {totals['skipped']} / "
        f"失败 {totals['failed']}；raw={totals['raw_rows']} → 正本={totals['out_rows']}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
