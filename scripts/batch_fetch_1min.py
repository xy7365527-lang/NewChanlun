#!/usr/bin/env python3
"""批量拉取所有支持品种的 1分钟 K线数据。

使用 data_databento.py 中的 fetch_and_cache() 函数，
将数据以 Parquet 格式缓存到 .cache/ 目录。

用法:
    # 仅测试 (5天 CL 数据，验证 API 可用)
    python scripts/batch_fetch_1min.py --test-only

    # 全量拉取 (所有品种, 2020-01-01 ~ 2025-12-31)
    python scripts/batch_fetch_1min.py

    # 自定义日期范围
    python scripts/batch_fetch_1min.py --start 2023-01-01 --end 2024-12-31
"""

from __future__ import annotations

import argparse
import logging
import sys
import time
from datetime import datetime, timedelta
from pathlib import Path

# 确保项目根目录在 sys.path 中
_project_root = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(_project_root / "src"))

# 加载 .env（config.py 内部也会加载，但在 import 前先确保一次）
from dotenv import load_dotenv
load_dotenv(_project_root / ".env")

from newchan.data_databento import fetch_and_cache, _FUTURES_MAP, _STOCK_TICKERS

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
    datefmt="%Y-%m-%d %H:%M:%S",
)
logger = logging.getLogger(__name__)

# ====================================================================
# 品种分组
# ====================================================================

FUTURES_SYMBOLS: list[str] = sorted(_FUTURES_MAP.keys())
# 从 _STOCK_TICKERS 拆分出 ETF 和个股
_ETF_SET = {"SPY", "QQQ", "IWM"}
ETF_SYMBOLS: list[str] = sorted(_ETF_SET)
STOCK_SYMBOLS: list[str] = sorted(_STOCK_TICKERS - _ETF_SET)

ALL_SYMBOLS: list[str] = FUTURES_SYMBOLS + STOCK_SYMBOLS + ETF_SYMBOLS


def run_test_fetch() -> bool:
    """测试拉取：仅取 CL 最近 5天 1min 数据，验证 API key 可用。"""
    logger.info("=" * 60)
    logger.info("测试模式：拉取 CL (WTI原油) 最近5天 1min 数据")
    logger.info("=" * 60)

    end_date = (datetime.utcnow() - timedelta(days=1)).strftime("%Y-%m-%d")
    start_date = (datetime.utcnow() - timedelta(days=6)).strftime("%Y-%m-%d")

    logger.info("日期范围: %s → %s", start_date, end_date)

    try:
        t0 = time.time()
        cache_name, row_count = fetch_and_cache(
            symbol="CL",
            interval="1min",
            start=start_date,
            end=end_date,
        )
        elapsed = time.time() - t0

        logger.info("测试成功!")
        logger.info("  缓存名称: %s", cache_name)
        logger.info("  数据行数: %d", row_count)
        logger.info("  耗时: %.1f 秒", elapsed)
        return True

    except Exception as e:
        logger.error("测试失败: %s", e, exc_info=True)
        return False


def run_batch_fetch(
    start: str = "2020-01-01",
    end: str = "2025-12-31",
) -> dict[str, dict]:
    """批量拉取所有品种的 1min 数据。

    Returns
    -------
    dict[str, dict]
        {symbol: {"cache_name": str, "rows": int, "elapsed": float, "error": str|None}}
    """
    results: dict[str, dict] = {}
    total = len(ALL_SYMBOLS)

    logger.info("=" * 60)
    logger.info("批量拉取 %d 个品种, 1min K线", total)
    logger.info("日期范围: %s → %s", start, end)
    logger.info("期货 (%d): %s", len(FUTURES_SYMBOLS), ", ".join(FUTURES_SYMBOLS))
    logger.info("美股 (%d): %s", len(STOCK_SYMBOLS), ", ".join(STOCK_SYMBOLS))
    logger.info("ETF  (%d): %s", len(ETF_SYMBOLS), ", ".join(ETF_SYMBOLS))
    logger.info("=" * 60)

    for i, symbol in enumerate(ALL_SYMBOLS, 1):
        logger.info("[%d/%d] 拉取 %s ...", i, total, symbol)
        try:
            t0 = time.time()
            cache_name, row_count = fetch_and_cache(
                symbol=symbol,
                interval="1min",
                start=start,
                end=end,
            )
            elapsed = time.time() - t0

            results[symbol] = {
                "cache_name": cache_name,
                "rows": row_count,
                "elapsed": elapsed,
                "error": None,
            }
            logger.info(
                "  ✓ %s: %d 行, %.1f 秒", symbol, row_count, elapsed
            )

        except Exception as e:
            elapsed = time.time() - t0
            results[symbol] = {
                "cache_name": f"{symbol}_1min_raw",
                "rows": 0,
                "elapsed": elapsed,
                "error": str(e),
            }
            logger.error("  ✗ %s 失败: %s", symbol, e)

    return results


def print_summary(results: dict[str, dict]) -> None:
    """打印拉取结果汇总表。"""
    logger.info("")
    logger.info("=" * 60)
    logger.info("拉取结果汇总")
    logger.info("=" * 60)

    success_count = 0
    fail_count = 0
    total_rows = 0
    total_time = 0.0

    # 表头
    header = f"{'品种':<8} {'行数':>12} {'耗时(s)':>10} {'状态':<8} {'备注'}"
    logger.info(header)
    logger.info("-" * 70)

    for symbol in ALL_SYMBOLS:
        if symbol not in results:
            continue
        r = results[symbol]
        if r["error"] is None:
            status = "成功"
            success_count += 1
            total_rows += r["rows"]
            note = r["cache_name"]
        else:
            status = "失败"
            fail_count += 1
            note = r["error"][:40]

        total_time += r["elapsed"]
        line = f"{symbol:<8} {r['rows']:>12,} {r['elapsed']:>10.1f} {status:<8} {note}"
        logger.info(line)

    logger.info("-" * 70)
    logger.info(
        "合计: %d 成功, %d 失败, %d 总行数, %.1f 秒总耗时",
        success_count, fail_count, total_rows, total_time,
    )


def main() -> None:
    parser = argparse.ArgumentParser(description="批量拉取 1min K线数据")
    parser.add_argument(
        "--test-only",
        action="store_true",
        help="仅执行测试拉取 (5天 CL 数据)",
    )
    parser.add_argument(
        "--start",
        default="2020-01-01",
        help="起始日期 YYYY-MM-DD (默认: 2020-01-01)",
    )
    parser.add_argument(
        "--end",
        default="2025-12-31",
        help="结束日期 YYYY-MM-DD (默认: 2025-12-31)",
    )
    args = parser.parse_args()

    # 测试模式
    if args.test_only:
        ok = run_test_fetch()
        sys.exit(0 if ok else 1)

    # 先做测试拉取
    logger.info("先执行测试拉取，验证 API 连通性...")
    if not run_test_fetch():
        logger.error("测试拉取失败，中止批量操作。")
        sys.exit(1)

    logger.info("")
    logger.info("测试通过，开始批量拉取...")
    logger.info("")

    results = run_batch_fetch(start=args.start, end=args.end)
    print_summary(results)


if __name__ == "__main__":
    main()
