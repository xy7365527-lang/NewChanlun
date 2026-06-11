"""C段边界修复 前/后 快照捕获（B2 + 修复A 验证脚本）。

捕获两类数据：
1. **不变层校验和**（修复约束：笔/线段/中枢层逐位不变）：
   strokes / segments / zhongshus（线段中枢）/ 笔中枢 的 md5。
2. **漂移层计数**（修复目标：背驰/买卖点解锁）：
   ladder2（笔中枢）/ ladder3（走势级 L1）/ ladder4+（递归层）的
   div 计数 + BSP (kind, side, confirmed) 计数 + 递归层 move/zs 结构计数。

用法：PYTHONPATH=src .venv/bin/python analysis/_c_seg_fix_capture.py before|after
输出：analysis/data_cache/c_seg_fix_{before,after}.json
"""
from __future__ import annotations

import hashlib
import json
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as R  # noqa: E402

from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from fugue_version_i import MAX_LEVELS  # noqa: E402


def md5_of(obj) -> str:
    return hashlib.md5(repr(obj).encode()).hexdigest()


def bsp_counts(bsps: list) -> dict:
    c = Counter((b[0][0], b[0][1], b[0][5]) for b in bsps)
    return {f"{k}|{s}|{conf}": n for (k, s, conf), n in sorted(c.items())}


def div_counts(divs: list) -> dict:
    c = Counter((d[0][0], d[0][1]) for d in divs)
    return {f"{k}|{d}": n for (k, d), n in sorted(c.items())}


def main() -> None:
    tag = sys.argv[1]
    opens, highs, lows, closes, _years = load_ohlc(SYMBOL_FILES["OKLO"])
    n = len(closes)
    orch = R.RecursiveOrchestrator(max_levels=MAX_LEVELS)
    for i in range(n):
        orch.process_bar(opens[i], highs[i], lows[i], closes[i])
    print(f"engine done: {n} bars")

    out: dict = {"tag": tag, "n_bars": n}

    # ── 1. 不变层校验和 ──
    strokes = orch.current_strokes()
    segments = orch.current_segments()
    zhongshus = orch.current_zhongshus()
    out["strokes"] = {"n": len(strokes), "md5": md5_of(strokes)}
    out["segments"] = {"n": len(segments), "md5": md5_of(segments)}
    out["zhongshus_seg_level"] = {"n": len(zhongshus), "md5": md5_of(zhongshus)}

    # 笔中枢层（独立于走势级中枢，也必须不变）
    confirmed = [s for s in strokes if s[7]]
    zs_in = [(s[0], s[1], s[3], s[4], True) for s in confirmed]
    bi_zhs = R.zhongshu_from_strokes(zs_in)
    out["zhongshus_bi_level"] = {"n": len(bi_zhs), "md5": md5_of(bi_zhs)}

    # ── 2. 漂移层 ──
    # L1 moves（level-1 构造未改 → 应不变，作为额外守卫记录）
    moves_l1 = orch.current_moves()
    out["moves_l1"] = {"n": len(moves_l1), "md5": md5_of(moves_l1)}

    # ladder2：笔中枢路径（与 probe3 Sim2 同口径）
    segs2 = [(s[2], s[3], s[4], s[0], s[1]) for s in confirmed]
    mvs2 = R.moves_from_zhongshus([tuple(z) for z in bi_zhs], len(segs2))
    zs5 = [(z[0], z[1], z[2], z[3], z[5]) for z in bi_zhs]
    mv_in2 = [m[0] for m in mvs2]
    divs2 = R.divergences_from_moves_v1(segs2, zs5, mv_in2, 1)
    bsps2 = orch.current_bi_zhongshu_buysellpoints(1)
    out["ladder2"] = {
        "n_moves": len(mvs2),
        "div": div_counts(divs2),
        "n_div": len(divs2),
        "bsp": bsp_counts(bsps2),
        "n_bsp": len(bsps2),
    }

    # ladder3：走势级（orchestrator L1 管线）
    bsps3 = orch.current_buysellpoints()
    out["ladder3"] = {"bsp": bsp_counts(bsps3), "n_bsp": len(bsps3)}

    # ladder4+：递归层（probe3 Sim1 同口径，但不做任何 seg_end 改写——引擎原样）
    recursive = orch.current_recursive()
    level_moves_map = {lid: mvs for (lid, _z, mvs) in recursive}
    rec_out = {}
    for (lid, zhs, mvs) in recursive:
        ladder = lid + 2
        prev = moves_l1 if lid - 1 == 1 else level_moves_map.get(lid - 1, [])
        # per_level_bsp 口径：segments = 前级别 moves 全列表（settled 为前缀，索引自洽）
        seg_in = [(m[0][1], m[1][0], m[1][1], m[1][2], m[1][3]) for m in prev]
        zs5r = [(z[0], z[1], z[2], z[3], z[5]) for z in zhs]
        zs7r = [(z[0], z[1], z[2], z[3], z[5], z[7], z[6]) for z in zhs]
        mv_in = [m[0] for m in mvs]
        divs = R.divergences_from_moves_v1(seg_in, zs5r, mv_in, lid)
        div_in = [(d[0][0], d[0][1], d[0][7], d[0][5], d[0][6], d[1][0], d[1][1])
                  for d in divs]
        bsps = R.buysellpoints_from_level(seg_in, zs7r, mv_in, div_in, lid)
        rec_out[f"ladder{ladder}"] = {
            "n_zs": len(zhs),
            "n_moves": len(mvs),
            "moves_md5": md5_of(mvs),
            "zs_md5": md5_of(zhs),
            "div": div_counts(divs),
            "n_div": len(divs),
            "bsp": bsp_counts(bsps),
            "n_bsp": len(bsps),
        }
    out["recursive"] = rec_out

    dest = ROOT / "analysis" / "data_cache" / f"c_seg_fix_{tag}.json"
    dest.write_text(json.dumps(out, indent=1, ensure_ascii=False))
    print(f"written: {dest}")
    print(json.dumps({k: v for k, v in out.items() if k != "tag"}, indent=1)[:2000])


if __name__ == "__main__":
    main()
