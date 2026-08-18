#!/usr/bin/env python3
"""K4 Databento 线全史 tick 拉取管线（issue #1089，ADR 0023 §一/§三，SPEC #1085 S5）
——薄 CLI + 六边 M/P/C/R 标的清单 + 三交易所地板。

把 Databento 期货逐笔成交（trades schema）**全字段原样**落盘为 Parquet(ZSTD)，
per-symbol/per-day 分区：

    analysis/data_cache/k4_databento_tick/{SYMBOL}/dt={YYYY-MM-DD}/trades.parquet

- Databento SDK `timeseries.get_range`（schema=trades, stype_in=continuous）→
  `DBNStore.to_parquet` 直出（SDK 流式分块写 pyarrow，不物化 pandas；
  pretty_ts=True 保 timestamp[ns] 历史段全精度；map_symbols=True 补 symbol 列；
  trades schema 全字段保留）。API key 从 env 或仓库根 .env 读 `DATABENTO_API_KEY`
  （只进内存、不写任何文件）。
- 六边 M/P/C/R 标的清单与三交易所地板见 `K4_SYMBOLS`（逐条标顶点/边归属 + 诚实缺口）：
  CME 2010-06-06 / ICE 2018-12-23 / Eurex 2025-03-10。
- 断点续拉：已存在且行数>0 的分区跳过（pyarrow 读 num_rows 判定）；429/5xx/超时指数退避
  （带抖动、读 Retry-After）；422 数据越界判空跳过；其余 4xx 致命（401/403 密钥、404 标的）。
- 消费层聚合照 ADR 0023（日级 regime、顶点 1m/1h）——数据粒度（tick）与消费聚合是两回事，
  本管线只落 tick 原样数据，不做聚合。

传输 + 落盘政策在 `k4_databento_fetch.py` 政策模块（#1089 / #1063 seam 保持）；
本脚本只留 CLI（main/plan）+ 六边标的清单 + 地板解析。

用法：
    # 单标的（ES 量最大，验收样例可先拉 FESX / DX 小量）
    uv run python scripts/fetch_k4_databento_tick.py --symbols fesx
    # 六边全清单后台拉取
    nohup uv run python scripts/fetch_k4_databento_tick.py \
        > analysis/data_cache/k4_databento_tick_fetch.log 2>&1 &
    # 只打印计划（不联网、不读 key）：哪些日期会拉 / 哪些分区已存在会跳过
    uv run python scripts/fetch_k4_databento_tick.py --plan
"""

from __future__ import annotations

import argparse
import datetime as dt
import os
import sys
from dataclasses import dataclass
from pathlib import Path

from dotenv import load_dotenv

from k4_databento_fetch import (
    fetch_symbol,
    iter_days,
    log,
    partition_done,
    partition_path,
)

_ROOT = Path(__file__).resolve().parents[1]

DEFAULT_ROOT = Path("analysis/data_cache/k4_databento_tick")

# 三交易所 tick 地板（ADR 0023 §一 / D3 #1032 metadata 实测；与
# scripts/fetch_1m_databento_10y.py、scripts/fetch_k4_global.py 同源）。
FLOOR_GLBX = "2010-06-06"   # CME GLBX.MDP3
FLOOR_ICE = "2018-12-23"    # ICE IFEU.IMPACT / IFUS.IMPACT
FLOOR_XEUR = "2025-03-10"   # Eurex XEUR.EOBI


@dataclass(frozen=True, slots=True)
class K4Symbol:
    """K4 线六边 M/P/C/R 一个标的的 Databento tick 规格。"""

    symbol: str       # 分区键 / 标的根符号（ES/GC/...）
    dataset: str      # Databento dataset
    db_symbol: str    # 连续合约符号（stype_in=continuous，.v.0 成交量展期）
    seat: str         # K4 顶点/边归属
    floor: str        # 该 dataset tick 地板（YYYY-MM-DD）
    note: str         # 诚实标注（代理关系 / 缺口）


# ── 六边 M/P/C/R 标的清单（实施中列明，SPEC #1085 S5 / ADR 0023 §一）────────
#
# K4 拓扑 = 四顶点 M/P/C/R 六条边（graph.py：3 独立边 P/M·C/M·R/M + 3 派生边
# P/C·P/R·C/R）。顶点正典代理（topology/data_mapping.py 单一真相源）：
#   M=UUP（美元 ETF）/ P=ES（标普期货）/ C=DBC（商品 ETF）/ R=VNQ（不动产 ETF）。
# 其中仅 P=ES 是 CME 期货、有 Databento tick 源；M/C/R 正典代理均为 ETF（无 tick）。
# 本清单给 K4 线落在 Databento 上的期货侧观测代理（recursive_decomposition_tree.md:762
# 期货折叠通道 {ES,GC,CL,$=DX} 同族 + fetch_1m_databento_10y.py 全期货 7 标的），
# 逐条标顶点/边归属与代理关系；无期货代理的正典顶点（R=VNQ）诚实标缺口，不伪造。
K4_SYMBOLS: list[K4Symbol] = [
    K4Symbol("ES", "GLBX.MDP3", "ES.v.0", "P 顶点",
             FLOOR_GLBX, "标普 E-mini（生产资本金融化）。正典代理即 ES，tick 直出。"),
    K4Symbol("GC", "GLBX.MDP3", "GC.v.0", "Au 折叠通道 C↔M",
             FLOOR_GLBX, "黄金 $ 相位（292号 Au 全局共享折叠通道观测）。"),
    K4Symbol("CL", "GLBX.MDP3", "CL.v.0", "Oil 折叠通道 C→P / C 顶点",
             FLOOR_GLBX, "WTI 原油 $ 相位；亦作 C 顶点能源组件（正典 C=DBC ETF 无 tick）。"),
    K4Symbol("ZN", "GLBX.MDP3", "ZN.v.0", "R 顶点（国债代理，非正典）",
             FLOOR_GLBX, "10 年期国债。正典 R=VNQ（不动产 ETF）无期货 tick；ZN 为 1m 期货实验"
             "既定 R 代理（ROADMAP.md:166 编排者显式覆盖，非正典，不可与正典 K4 比较）。"),
    K4Symbol("6E", "GLBX.MDP3", "6E.v.0", "货币边 M_EUR/M_US",
             FLOOR_GLBX, "EUR/USD FX（254号层0 结算尺，跨国对齐用）。"),
    K4Symbol("DX", "IFUS.IMPACT", "DX.v.0", "M 顶点",
             FLOOR_ICE, "美元指数（正典 M=UUP ETF 无期货 tick，DX 为期货代理）。"),
    K4Symbol("BRN", "IFEU.IMPACT", "BRN.v.0", "C 顶点（ICE）",
             FLOOR_ICE, "Brent 原油（ICE Europe，Oil 通道 ICE 腿）。"),
    K4Symbol("FESX", "XEUR.EOBI", "FESX.v.0", "P 顶点（EU 截面）",
             FLOOR_XEUR, "STOXX50 期货（Eurex，跨国 EU P；Eurex tick 仅 2025-03 起，按现状接受）。"),
]


def symbol_by_key(key: str) -> K4Symbol | None:
    key = key.strip().upper()
    for s in K4_SYMBOLS:
        if s.symbol.upper() == key:
            return s
    return None


def parse_date(s: str) -> dt.date:
    try:
        return dt.date.fromisoformat(s)
    except ValueError as e:
        raise SystemExit(f"非法日期 {s!r}（应为 YYYY-MM-DD）") from e


def load_api_key() -> str:
    load_dotenv(_ROOT / ".env")  # 不覆盖已设环境变量
    key = os.environ.get("DATABENTO_API_KEY") or os.environ.get("DATABENTO_KEY")
    key = (key or "").strip()
    if not key:
        raise SystemExit(
            "未找到 DATABENTO_API_KEY：请 `export DATABENTO_API_KEY=...` 或在仓库根 .env 写一行\n"
            "`DATABENTO_API_KEY=...`（该文件已 gitignore、应 chmod 600）。密钥只进内存，不写任何文件。"
        )
    return key


def resolve_start(spec: K4Symbol, start_override: dt.date | None) -> dt.date:
    """实际历史起点 = max(该标的 dataset 地板, 用户指定起点)（缺省用各自地板）。"""
    floor = parse_date(spec.floor)
    if start_override is None:
        return floor
    return max(floor, start_override)


def plan(root, specs: list[K4Symbol], start_override, end: dt.date) -> None:
    """不联网、不读 key：按现有分区打印各标的计划（已存在分区 = 跳过）。"""
    print(f"计划（end={end} root={root}）")
    for spec in specs:
        start = resolve_start(spec, start_override)
        n_days = n_existing = 0
        for d in iter_days(start, end):
            n_days += 1
            if partition_done(partition_path(root, spec.symbol, d.isoformat())):
                n_existing += 1
        print(
            f"{spec.symbol:6s} {spec.seat:22s} {spec.dataset:12s} 地板 {spec.floor}: "
            f"需处理 {n_days} 天，已有分区 {n_existing} 天，将拉 {n_days - n_existing} 天"
        )


def main(argv=None) -> int:
    default_keys = ",".join(s.symbol for s in K4_SYMBOLS)
    ap = argparse.ArgumentParser(
        description="K4 Databento 线全史 tick 拉取管线（#1089）",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    ap.add_argument("--symbols", default=default_keys,
                    help=f"逗号分隔标的（默认六边全清单：{default_keys}）")
    ap.add_argument("--start", default=None,
                    help="历史起点（默认各标的 dataset 地板：CME 2010-06-06 / ICE 2018-12-23 / "
                         "Eurex 2025-03-10；早于地板自动钳到地板）")
    ap.add_argument("--end", default=None,
                    help="结束日（默认昨天 UTC——避免把当日未完盘写成完整分区导致续拉永久跳过；"
                         "与 fetch_k4_global.py 的 end=utcnow-1d 同口径）")
    ap.add_argument("--root", type=Path, default=DEFAULT_ROOT, help="落盘根目录")
    ap.add_argument("--max-retries", type=int, default=6, help="429/5xx/超时重试次数")
    ap.add_argument("--backoff-base", type=float, default=1.0, help="指数退避基数（秒）")
    ap.add_argument("--max-backoff", type=float, default=60.0, help="指数退避封顶（秒）")
    ap.add_argument("--plan", action="store_true",
                    help="只打印拉取计划（不联网、不读 key）")
    args = ap.parse_args(argv)

    specs: list[K4Symbol] = []
    for raw in args.symbols.split(","):
        raw = raw.strip()
        if not raw:
            continue
        spec = symbol_by_key(raw)
        if spec is None:
            raise SystemExit(f"未知标的 {raw!r}，可选：{', '.join(s.symbol for s in K4_SYMBOLS)}")
        specs.append(spec)

    start_override = parse_date(args.start) if args.start else None
    # 默认 end = 昨天 UTC：当日数据未完盘，落成完整分区会让续拉永久跳过该日（半截数据）。
    # dt.date.today() 是**本地**日期，与「昨天 UTC」及 fetch_k4_global.py 的 utcnow-1d
    # 同口径不符——东时区机器会把「UTC 今天」当「昨天」，把未完盘当日落成完整分区。
    end = (
        parse_date(args.end)
        if args.end
        else dt.datetime.now(dt.timezone.utc).date() - dt.timedelta(days=1)
    )
    if start_override is not None and end < start_override:
        raise SystemExit(f"--end（{end}）早于 --start（{start_override}）")

    if args.plan:
        plan(args.root, specs, start_override, end)
        return 0

    key = load_api_key()
    import databento as db
    client = db.Historical(key=key)

    totals = {"fetched": 0, "skipped": 0, "empty": 0, "failed": 0}
    failed_days: list[tuple[str, str]] = []
    for spec in specs:
        start = resolve_start(spec, start_override)
        log(f"[{spec.symbol}] {spec.seat} | {spec.dataset} {spec.db_symbol} "
            f"trades tick [{start} → {end}]")
        stats = fetch_symbol(
            client, spec.symbol, dataset=spec.dataset, db_symbol=spec.db_symbol,
            stype_in="continuous", root=args.root, start=start, end=end,
            max_retries=args.max_retries, backoff_base=args.backoff_base,
            max_backoff=args.max_backoff,
        )
        for k in totals:
            totals[k] += stats[k]
        failed_days.extend(stats["failed_days"])
        log(
            f"[{spec.symbol}] 完成：拉 {stats['fetched']} 天 / 跳过 {stats['skipped']} 天 / "
            f"空 {stats['empty']} 天 / 失败 {stats['failed']} 天"
        )
    log(
        f"全部完成：拉 {totals['fetched']} 天 / 跳过 {totals['skipped']} 天 / "
        f"空 {totals['empty']} 天 / 失败 {totals['failed']} 天"
    )
    if failed_days:
        log(f"失败日清单（{len(failed_days)} 条，单日失败=续拉状态，退出码 0）：")
        for date_str, err in failed_days:
            log(f"  - {date_str}: {err}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
