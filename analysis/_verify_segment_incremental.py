"""差分验证：RecursiveOrchestrator 线段层增量续算 ≡ 全量重算（bit-exact）。

逐 bar 比对：
  inc  = orch.current_segments()                                  （增量复用前缀）
  full = newchan_rust.segments_from_strokes_v1(orch.current_strokes())  （独立全量 oracle）

全量 oracle 是 PyO3 暴露的独立批量函数，与 orchestrator 内部增量路径无共享状态——
故二者逐 bar 相等 ⟹ 增量 resume≡full（实现正确性，非领域假设）。

认识论等级：实现正确性差分测试（resume≡full 的 bit-exact 契约）。真实数据 OKLO 447K
覆盖真实笔型/缺口/古怪线段分布——非合成，暴露 segment resume 回归类发散。

用法：
  PYTHONPATH=src .venv/bin/python analysis/_verify_segment_incremental.py [N_BARS] [STRIDE]
  N_BARS：验证的 bar 数（默认全部 447K）。
  STRIDE：oracle 比对步长（默认 1=逐 bar）。STRIDE>1 时仅在每第 STRIDE 个
          "线段已变化"的 bar 上比对（降低 O(B·S) oracle 成本，覆盖度下降）。
"""

from __future__ import annotations

import json
import os
import sys
import time
from pathlib import Path

NOSKIP = os.environ.get("NOSKIP") == "1"

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "src"))

import newchan_rust  # noqa: E402

DATA = Path(
    os.environ.get(
        "SEG_DATA", str(ROOT / "analysis" / "data_cache" / "oklo_1m_databento.json")
    )
)

# RecursiveOrchestrator 内部 BiEngine 默认参数（lib.rs 默认 stroke_mode="wide"）。
_MAX_LEVELS = 6


def load_447k():
    """加载 OHLC。支持两种缓存格式（逐位等价无关，仅取 OHLC 四列）：
      1. OKLO bars-of-dicts：raw["bars"][i] = {"open","high","low","close"}
      2. databento opens-arrays：raw["opens"/"highs"/"lows"/"closes"] 为并列数组
    NaN 行（databento 期货含少量 nan）直接删除——与 m1_e_rust_backtest 一致，避免污染笔型。
    """
    import math

    raw = json.loads(DATA.read_text())
    if "bars" in raw:
        bars = raw["bars"]
        o = [float(b["open"]) for b in bars]
        h = [float(b["high"]) for b in bars]
        l = [float(b["low"]) for b in bars]
        c = [float(b["close"]) for b in bars]
    else:
        o = [float(x) for x in raw["opens"]]
        h = [float(x) for x in raw["highs"]]
        l = [float(x) for x in raw["lows"]]
        c = [float(x) for x in raw["closes"]]

    def _ok(i: int) -> bool:
        return not (
            math.isnan(o[i]) or math.isnan(h[i]) or math.isnan(l[i]) or math.isnan(c[i])
        )

    keep = [i for i in range(len(o)) if _ok(i)]
    if len(keep) != len(o):
        print(f"  删除 {len(o) - len(keep)} 个 NaN bar（{len(o)}→{len(keep)}）")
        o = [o[i] for i in keep]
        h = [h[i] for i in keep]
        l = [l[i] for i in keep]
        c = [c[i] for i in keep]
    return o, h, l, c


def main() -> None:
    n_bars = int(sys.argv[1]) if len(sys.argv) > 1 else None
    stride = int(sys.argv[2]) if len(sys.argv) > 2 else 1

    print(f"加载 {DATA.name} ...")
    opens, highs, lows, closes = load_447k()
    n_total = len(opens)
    n = min(n_bars, n_total) if n_bars else n_total
    print(f"总 {n_total} bars，验证前 {n} bars，stride={stride}")

    orch = newchan_rust.RecursiveOrchestrator(max_levels=_MAX_LEVELS)

    prev_inc = None
    compared = 0
    changed = 0
    t0 = time.time()

    for i in range(n):
        orch.process_bar(opens[i], highs[i], lows[i], closes[i])
        inc = orch.current_segments()

        # 仅在 inc 线段列表相对上一 bar 变化时比对（无变化 ⟹ 上一 bar 已验证过同一前缀）。
        # NOSKIP=1 关闭此优化，逐 bar 强制 oracle 比对——排除"增量错误滞留"盲点。
        if not NOSKIP and inc == prev_inc:
            continue
        if inc != prev_inc:
            changed += 1
        prev_inc = inc

        if stride > 1 and (changed % stride != 0) and i != n - 1:
            continue

        strokes = orch.current_strokes()
        full = newchan_rust.segments_from_strokes_v1(strokes, 3, "strict")
        compared += 1

        if inc != full:
            print(f"\n❌ 发散 @ bar {i}（n_strokes={len(strokes)}）")
            print(f"   inc  段数={len(inc)}  full 段数={len(full)}")
            # 定位首个不同段
            m = min(len(inc), len(full))
            for k in range(m):
                if inc[k] != full[k]:
                    print(f"   首个不同段 idx={k}:")
                    print(f"     inc ={inc[k]}")
                    print(f"     full={full[k]}")
                    break
            else:
                if len(inc) != len(full):
                    print(f"   前 {m} 段相同，段数不同（尾部差异）")
                    tail = inc[m:] if len(inc) > len(full) else full[m:]
                    print(f"     多出尾段[0]={tail[0]}")
            sys.exit(1)

        if i % 20000 == 0:
            el = time.time() - t0
            print(
                f"  bar {i:>7}/{n}  线段变化={changed}  已比对={compared}  "
                f"段数={len(inc)}  strokes={len(strokes)}  {el:.1f}s"
            )

    el = time.time() - t0
    print(
        f"\n✅ bit-exact：{n} bars 全部相等。"
        f"线段变化 bar={changed}，oracle 比对次数={compared}，耗时 {el:.1f}s"
    )
    print(f"   最终段数={len(orch.current_segments())}")


if __name__ == "__main__":
    main()
