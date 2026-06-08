"""双树法 K 线 PH 引擎验证 —— Part B（700 对比单树）+ Part D（MOS/OKLO 复跑）。

存在论位置
----------
本脚本对比 `a_dual_merge_tree.DualMergeTree`（K 线 HL 双树）与 `a_online_persistence`
单树（close）在真实日线上的 settle 阶梯差异，并产出双树独有的**顶 settle 阶梯**（跌破
确认顶死）与**笔候选**。

对比维度（B）
-------------
1. 底 settle 阶梯：单树（喂 close，反弹涨过确认）vs 双树 T_low（喂 low，反弹涨过确认）。
   low 比 close 更深 → 捕捉的底更低、阶梯更完整。
2. 顶 settle 阶梯：**单树结构上给不出**（close-sublevel 看不见顶）；双树 T_high 独有
   （high 跌破确认顶死）。这是双树相对单树的**信息增量**。
3. 笔候选：双树交替极值序列 vs 缠论指标的笔（缠论指标需 TV-MCP，本脚本只产出 PH 候选
   与统计；与缠论指标的逐笔对齐留 L2 对比，未连 TV 时不编造，llm-role-boundary）。

引擎调用约定
------------
- DualMergeTree.update(high, low) 逐根喂入（high 内部取负进 T_high，low 进 T_low）。
- settle_triggers_high/low() 在 finalize 前调（读 alive 分量）。
- 单树对照：settle_triggers_from_prices(closes)（喂 close，与历史报告同口径）。

认识论等级（formalization-validity-domain）
-------------------------------------------
- 双树构建 / 取负还原 / settle 方向翻转：L0（确定性）。
- "low 阶梯比 close 更完整"：L2（真实数据可观测的对比，本脚本产出证据）。
- "笔候选 = 缠论笔"：候选同构，需 TV 缠论指标 L2 对比（本脚本不声称已验证）。

用法
----
    cd <repo> && .venv/bin/python -u analysis/dual_merge_tree_validation.py
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

from newchan.a_dual_merge_tree import DualMergeTree  # noqa: E402
from newchan.a_settle_trigger import settle_triggers_from_prices  # noqa: E402

CACHE = ROOT / "analysis" / "data_cache"
OUT_FILE = CACHE / "dual_merge_tree_compare.json"


def _load(symbol_file: str) -> dict:
    return json.loads((CACHE / symbol_file).read_text())


def _peak_idx(closes: list[float]) -> int:
    """closes 上 argmax（见顶定位，与 700 子窗口报告同口径）。"""
    return max(range(len(closes)), key=lambda i: closes[i])


def _ladder_rows(triggers, *, limit: int = 8) -> list[dict]:
    """settle 阶梯行（可 settle 的按 |gap| 升序取近端 limit 条 + 全局/主导分量）。"""
    settleable = [t for t in triggers if t.can_settle and t.trigger_price is not None]
    settleable.sort(key=lambda t: abs(t.gap))
    rows = []
    for t in settleable[:limit]:
        rows.append(
            {
                "birth_idx": t.birth_idx,
                "extreme": round(t.extreme_price, 3),
                "trigger": round(t.trigger_price, 3),
                "gap": round(t.gap, 3),
                "pers": round(t.trigger_persistence, 3),
                "dominant": t.is_dominant,
            }
        )
    glob = [t for t in triggers if not t.can_settle]
    grows = [
        {
            "birth_idx": t.birth_idx,
            "extreme": round(t.extreme_price, 3),
            "trigger": None,
            "reversal_amplitude": round(t.reversal_amplitude, 3)
            if t.reversal_amplitude is not None
            else None,
            "dominant": t.is_dominant,
            "global": True,
        }
        for t in glob
    ]
    return rows + grows


def analyze(symbol_file: str, *, sub_window_from_peak: bool = False) -> dict:
    data = _load(symbol_file)
    closes = [float(x) for x in data["closes"]]
    highs = [float(x) for x in data["highs"]]
    lows = [float(x) for x in data["lows"]]
    dates = data.get("dates", [])

    start = 0
    note = "full series"
    if sub_window_from_peak:
        start = _peak_idx(closes)
        note = f"sub-window from close-peak @idx{start} ({dates[start] if dates else '?'})"

    w_closes = closes[start:]
    w_highs = highs[start:]
    w_lows = lows[start:]

    # 双树（high/low）
    dt = DualMergeTree.from_ohlc(w_highs, w_lows)
    top_ladder = dt.settle_triggers_high()
    bot_ladder_low = dt.settle_triggers_low()
    strokes = dt.strokes()
    containment = dt.containment_pairs()

    # 单树（close）—— 历史报告同口径
    single_close = settle_triggers_from_prices(w_closes)

    # 单树 close 只给"底 settle"（close 反弹涨过）；适配成同一阶梯结构对照
    single_rows = []
    settleable = [
        t for t in single_close if t.can_settle_by_rebound and t.settle_price is not None
    ]
    settleable.sort(key=lambda t: abs(t.gap_to_settle))
    for t in settleable[:8]:
        single_rows.append(
            {
                "birth_idx": t.birth_idx,
                "valley_close": round(t.birth_price, 3),
                "settle_close": round(t.settle_price, 3),
                "gap": round(t.gap_to_settle, 3),
                "pers": round(t.settle_persistence, 3),
                "dominant": t.is_dominant,
            }
        )

    last_strokes = [
        {
            "dir": s.direction,
            "from": f"{s.start_kind}@{s.start_idx}={round(s.start_price, 2)}",
            "to": f"{s.end_kind}@{s.end_idx}={round(s.end_price, 2)}",
            "amp": round(s.amplitude, 3),
            "confirmed": s.start_confirmed and s.end_confirmed,
        }
        for s in strokes[-6:]
    ]

    return {
        "symbol": data.get("symbol", symbol_file),
        "window": note,
        "n_bars": len(w_closes),
        "last": {
            "close": round(w_closes[-1], 3),
            "high": round(w_highs[-1], 3),
            "low": round(w_lows[-1], 3),
        },
        # 底 settle 阶梯：单树 close vs 双树 low
        "bottom_ladder_single_close": single_rows,
        "bottom_ladder_dual_low": _ladder_rows(bot_ladder_low),
        # 顶 settle 阶梯：双树独有（单树给不出）
        "top_ladder_dual_high": _ladder_rows(top_ladder),
        # 笔候选 + 包含
        "n_strokes": len(strokes),
        "n_strokes_confirmed": sum(
            1 for s in strokes if s.start_confirmed and s.end_confirmed
        ),
        "last_strokes": last_strokes,
        "n_containment_pairs": len(containment),
        # 信息增量度量
        "n_alive_tops": len(dt.alive_components_high()),
        "n_alive_bottoms": len(dt.alive_components_low()),
        "n_settled_tops": len(dt.settled_components_high()),
        "n_settled_bottoms": len(dt.settled_components_low()),
        "n_single_close_alive": sum(1 for t in single_close),
    }


def main() -> None:
    results = {
        "700_decline_window": analyze("HK700_1d_2y.json", sub_window_from_peak=True),
        "700_full": analyze("HK700_1d_2y.json"),
        "MOS": analyze("MOS_1d_5y.json"),
        "OKLO": analyze("OKLO_1d_max.json"),
    }
    OUT_FILE.write_text(json.dumps(results, indent=2, ensure_ascii=False))

    for key, r in results.items():
        print(f"\n{'=' * 70}\n{key}: {r['symbol']} | {r['window']} | n={r['n_bars']}")
        print(
            f"  last close={r['last']['close']} high={r['last']['high']} "
            f"low={r['last']['low']}"
        )
        print(
            f"  分量: 顶 alive={r['n_alive_tops']} settled={r['n_settled_tops']} | "
            f"底 alive={r['n_alive_bottoms']} settled={r['n_settled_bottoms']} | "
            f"单树close alive={r['n_single_close_alive']}"
        )
        print(f"  笔候选: {r['n_strokes']} (已确认 {r['n_strokes_confirmed']}) | "
              f"包含对: {r['n_containment_pairs']}")
        print("  — 底 settle 阶梯·近端（单树 close）:")
        for row in r["bottom_ladder_single_close"][:4]:
            print(f"      valley={row['valley_close']} settle={row['settle_close']} "
                  f"gap={row['gap']} {'[dom]' if row['dominant'] else ''}")
        print("  — 底 settle 阶梯·近端（双树 low）:")
        for row in r["bottom_ladder_dual_low"][:4]:
            if row.get("global"):
                print(f"      valley={row['extreme']} [全局/不可反弹settle] "
                      f"reversal={row.get('reversal_amplitude')}")
            else:
                print(f"      valley={row['extreme']} settle={row['trigger']} "
                      f"gap={row['gap']} {'[dom]' if row['dominant'] else ''}")
        print("  — 顶 settle 阶梯·近端（双树 high，单树给不出）:")
        for row in r["top_ladder_dual_high"][:4]:
            if row.get("global"):
                print(f"      peak={row['extreme']} [全局/不可跌破settle] "
                      f"reversal={row.get('reversal_amplitude')}")
            else:
                print(f"      peak={row['extreme']} 跌破={row['trigger']} "
                      f"gap(还需跌)={row['gap']} {'[dom]' if row['dominant'] else ''}")
        print("  — 最近笔候选:")
        for s in r["last_strokes"]:
            print(f"      {s['dir']:4} {s['from']} -> {s['to']} amp={s['amp']} "
                  f"{'✓' if s['confirmed'] else '·'}")
    print(f"\n→ JSON: {OUT_FILE}")


if __name__ == "__main__":
    main()
