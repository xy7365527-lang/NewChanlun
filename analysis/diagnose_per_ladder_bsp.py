"""每个 ladder 的 BSP 产出诊断 —— 验证「全递归每级必有买卖点」全称命题。

任务（2026-06-15）：用户全称命题——走势终完美 ⇒ 每个走势-级别必有买卖点。若某个
走势-ladder 有走势完美（settled move）但无 BSP ⇒ 信号层 bug。

本脚本对 ES 1s 全流跑完后的**最终递归状态**，逐 ladder 计算 confirmed BSP：
  - ladder2 笔中枢：current_bi_zhongshu_buysellpoints(1)
  - ladder3 move(L1)：current_buysellpoints()
  - ladder4+ recL：organic_signals._level_bsps_with_divs（信号层同路径）
并与该 ladder 的 settled 走势完美数 / 中枢数对照。

BSP append-only（moves/zhongshu settled 前缀冻结 ⇒ confirmed BSP 前缀冻结）⇒
最终状态 confirmed BSP 数 = 全流累计 distinct confirmed BSP 数。

BspTuple head = (kind, side, level_id, seg_idx, move_seg_start, confirmed, settled)

用法：.venv/bin/python analysis/diagnose_per_ladder_bsp.py [BT_MAX_BARS]
"""
from __future__ import annotations

import json
import sys
import time
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as R  # noqa: E402
from organic_signals import _level_bsps_with_divs  # noqa: E402

DATA = ROOT / "analysis" / "data_cache" / "es_1s_databento_1y.json"

H_KIND, H_DIR, H_SS, H_SE, H_ZS0, H_ZS1, H_ZSC, H_SETTLED = range(8)


def settled_moves(moves: list) -> int:
    return sum(1 for m in moves if (m[0] if isinstance(m[0], tuple) else m)[H_SETTLED])


def bsp_breakdown(bsps: list) -> dict:
    """BspTuple 列表 → {confirmed: {type:side: n}, candidate: total, total}。"""
    conf = Counter()
    cand = 0
    for bp in bsps:
        head = bp[0]
        kind, side, confirmed = head[0], head[1], head[5]
        if confirmed:
            conf[f"{kind}/{side}"] += 1
        else:
            cand += 1
    return dict(confirmed=dict(conf), confirmed_total=sum(conf.values()),
                candidate_total=cand, total=len(bsps))


def main() -> int:
    max_bars = int(sys.argv[1]) if len(sys.argv) > 1 else 0
    t0 = time.time()
    print(f"[load] {DATA.name} ...", flush=True)
    raw = json.loads(DATA.read_text())
    opens, highs, lows, closes = raw["opens"], raw["highs"], raw["lows"], raw["closes"]
    n = len(closes)
    if max_bars > 0:
        n = min(n, max_bars)
    print(f"[load] {n:,} bars in {time.time()-t0:.1f}s", flush=True)

    orch = R.RecursiveOrchestrator(max_levels=8)
    t1 = time.time()
    for i in range(n):
        o, h, l, c = opens[i], highs[i], lows[i], closes[i]
        if o > 0 and h > 0 and l > 0 and c > 0:
            orch.process_bar(o, h, l, c)
    print(f"[run] {n:,} bars in {time.time()-t1:.0f}s", flush=True)

    out = {}

    # ── ladder2 笔中枢 BSP ──
    bi_bsp = orch.current_bi_zhongshu_buysellpoints(1)
    out["ladder2_笔中枢"] = dict(bsp=bsp_breakdown(bi_bsp))

    # ── ladder3 move(L1) BSP ──
    mv_l1 = orch.current_moves()
    l1_bsp = orch.current_buysellpoints()
    out["ladder3_move(L1)"] = dict(
        settled_走势完美=settled_moves(mv_l1),
        n_zhongshu=len(orch.current_zhongshus()),
        bsp=bsp_breakdown(l1_bsp))

    # ── ladder4+ 递归层 BSP（信号层同路径）──
    rec = orch.current_recursive()
    level_moves_map = {lid: mvs for (lid, _z, mvs) in rec}
    for (lid, zhs, mvs) in rec:
        ladder = lid + 2
        prev = mv_l1 if lid == 2 else level_moves_map.get(lid - 1, [])
        bsps_l, div_rows_l = _level_bsps_with_divs(prev, zhs, mvs, lid)
        out[f"ladder{ladder}_recL{lid}"] = dict(
            settled_走势完美=settled_moves(mvs),
            n_zhongshu=len(zhs),
            n_背驰=len(div_rows_l),
            bsp=bsp_breakdown(bsps_l))

    print("\n" + "=" * 80)
    print("  每 ladder BSP 产出 vs 走势完美 —— 全称命题验证（ES 1s 11.77M 最终状态）")
    print("=" * 80)
    print(json.dumps(out, ensure_ascii=False, indent=2))

    print("\n" + "-" * 80)
    print("  全称命题核对表：每个走势-ladder 是否产出 confirmed BSP？")
    print("-" * 80)
    print(f"  {'ladder':18s} {'走势完美':>8s} {'中枢':>6s} {'背驰':>6s} {'confBSP':>8s}  判定")
    for k, v in out.items():
        sm = v.get("settled_走势完美", "—")
        nz = v.get("n_zhongshu", "—")
        nd = v.get("n_背驰", "—")
        cb = v["bsp"]["confirmed_total"]
        if isinstance(sm, int) and sm > 0:
            verdict = "✓ 有BSP" if cb > 0 else "✗✗ 走势完美但无BSP=BUG"
        else:
            verdict = "(笔中枢:无settled口径)" if cb > 0 else "—"
        print(f"  {k:18s} {str(sm):>8s} {str(nz):>6s} {str(nd):>6s} {cb:>8d}  {verdict}")

    outp = ROOT / "analysis" / "data_cache" / "per_ladder_bsp_ES1S.json"
    outp.write_text(json.dumps(dict(n_bars=n, out=out), ensure_ascii=False, indent=1))
    print(f"\n写入 {outp}  总耗时 {time.time()-t0:.0f}s")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
