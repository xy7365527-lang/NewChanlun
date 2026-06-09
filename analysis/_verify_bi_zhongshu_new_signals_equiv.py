"""L3 跨标的 bit-exact 差异测试：O(N) 增量 delta 接口 vs O(S²) 全量重算接口。

## 验证对象

`bi_zhongshu_new_signals(level_id)`（新，O(window) 摊还）逐位等价于调用方原路径
`_scan_new(current_bi_zhongshu_buysellpoints(level_id), seg_seen)`（旧，O(strokes²) 全量重算）。

## 方法

两个**独立** RecursiveOrchestrator 喂相同 bar 序列（旧路径只读、新路径有状态，分离实例
保证内部 seen-set 互不污染）。在与 m1_i_rust_engine 完全一致的门控 `sc > last_stroke_n`
下，逐 stroke-growth bar 比对 `(seg_buy1, seg_sell1, seg_sell_any, seg_buy_any)` 四元组。

## 认识论等级

L1（管线等价性验证：差异测试器在真实 OHLC 上守卫两实现逐位一致，不验证回测假设）。
真实数据跨标的（OKLO/ES/GC）→ 输入分布覆盖高 churn，但结论是"两接口等价"非"信号有效"。

用法：PYTHONPATH=src .venv/bin/python analysis/_verify_bi_zhongshu_new_signals_equiv.py
"""

from __future__ import annotations

import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as R  # noqa: E402

from m1_i_rust_backtest import DATA_DIR, load_ohlc  # noqa: E402
from m1_i_rust_engine import _scan_new  # noqa: E402
from fugue_version_i import MAX_LEVELS  # noqa: E402
from per_level_bsp import BI_ZHONGSHU_LEVEL_ID  # noqa: E402

# 标的 → (文件, 限制 bar 数 None=全量)
CASES = [
    ("OKLO", DATA_DIR / "oklo_1m_databento_full.json", None),      # 333K 全量（已验证基准）
    ("ES",   DATA_DIR / "es_1m_databento_10y.json",   500_000),    # 高 churn 期货前 500K
    ("GC",   DATA_DIR / "gc_1m_databento_10y.json",   500_000),    # 期货前 500K
]


def verify_symbol(symbol: str, path: Path, max_bars: int | None) -> dict:
    """两接口逐 stroke-growth bar 比对，返回差异报告。"""
    opens, highs, lows, closes, _ = load_ohlc(path)
    if max_bars is not None:
        opens, highs, lows, closes = (
            opens[:max_bars], highs[:max_bars], lows[:max_bars], closes[:max_bars])
    n = len(closes)

    orch_old = R.RecursiveOrchestrator(max_levels=MAX_LEVELS)
    orch_new = R.RecursiveOrchestrator(max_levels=MAX_LEVELS)
    seg_seen: set = set()  # 旧路径的调用方 seen-set（新路径下沉 Rust 内部）

    last_old = 0
    last_new = 0
    scan_events = 0          # stroke-growth 触发的比对次数
    mismatches: list[tuple] = []
    sig_old = 0              # 旧路径累计触发的信号位（任一 True 计 1）
    sig_new = 0

    t0 = time.time()
    last_report = 0
    for i in range(n):
        o, h, l, c = opens[i], highs[i], lows[i], closes[i]
        orch_old.process_bar(o, h, l, c)
        orch_new.process_bar(o, h, l, c)

        sc_old = orch_old.stroke_count()
        if sc_old > last_old:
            last_old = sc_old
            old_tuple = _scan_new(
                orch_old.current_bi_zhongshu_buysellpoints(BI_ZHONGSHU_LEVEL_ID), seg_seen)
        else:
            old_tuple = (False, False, False, False)

        sc_new = orch_new.stroke_count()
        if sc_new > last_new:
            last_new = sc_new
            new_tuple = orch_new.bi_zhongshu_new_signals(BI_ZHONGSHU_LEVEL_ID)
        else:
            new_tuple = (False, False, False, False)

        # 门控同步性：两 orch 喂相同 bar → stroke_count 确定性同步（分歧即记差异）。
        if sc_old != sc_new:
            mismatches.append((i, "stroke_count", sc_old, sc_new))

        if old_tuple != new_tuple:
            mismatches.append((i, "delta", old_tuple, new_tuple))
        if any(old_tuple):
            sig_old += 1
        if any(new_tuple):
            sig_new += 1
        if old_tuple != (False, False, False, False) or new_tuple != (False, False, False, False):
            scan_events += 1

        if i - last_report >= 100_000:
            el = time.time() - t0
            print(f"    [{symbol}] {i:>7,}/{n:,} bars | strokes={sc_new:>6,} "
                  f"| 信号事件={scan_events} | 差异={len(mismatches)} | {el:5.1f}s", flush=True)
            last_report = i

    return {
        "symbol": symbol,
        "n_bars": n,
        "strokes": last_new,
        "scan_events": scan_events,
        "sig_old": sig_old,
        "sig_new": sig_new,
        "mismatches": mismatches[:20],  # 截前 20 条
        "n_mismatch": len(mismatches),
        "elapsed_s": round(time.time() - t0, 1),
        "bit_exact": len(mismatches) == 0,
    }


def main() -> None:
    print("=" * 70)
    print("L3 跨标的 bit-exact 验证：bi_zhongshu_new_signals (O(N)) vs 全量 (O(S²))")
    print("=" * 70)
    results = []
    for symbol, path, max_bars in CASES:
        if not path.exists():
            print(f"  [{symbol}] 跳过：{path.name} 不存在")
            continue
        print(f"\n[{symbol}] {path.name} (max_bars={max_bars})")
        r = verify_symbol(symbol, path, max_bars)
        results.append(r)
        status = "✅ BIT-EXACT" if r["bit_exact"] else f"❌ {r['n_mismatch']} 处差异"
        print(f"  {status} | {r['n_bars']:,} bars | {r['strokes']:,} strokes "
              f"| 信号事件 {r['scan_events']} | 旧={r['sig_old']} 新={r['sig_new']} "
              f"| {r['elapsed_s']}s")
        if not r["bit_exact"]:
            for m in r["mismatches"]:
                print(f"      DIFF bar {m[0]}: {m[1]} old={m[2]} new={m[3]}")

    print("\n" + "=" * 70)
    all_exact = all(r["bit_exact"] for r in results)
    print(f"汇总：{len(results)} 标的，{'全部 BIT-EXACT ✅' if all_exact else '存在差异 ❌'}")
    for r in results:
        print(f"  {r['symbol']:5s} | {r['n_bars']:>9,} bars | {r['strokes']:>6,} strokes "
              f"| 信号 {r['sig_new']:>4} | {'✅' if r['bit_exact'] else '❌'}")
    sys.exit(0 if all_exact else 1)


if __name__ == "__main__":
    main()
