"""探针2：验证两个根因机制（只读）。

A. ladder≥4：每个 trend move 的 c 段 [zs_last.seg_end+1, mv.seg_end] 是否恒空。
B. ladder2/3：瞬态 type1 的 seg_idx 是否钉在段数组末端（pending move 尾锚），
   以及 settle 后 type1 存活率。
用法：PYTHONPATH=src .venv/bin/python analysis/_bsp_gap_probe2.py
"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as R  # noqa: E402

from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from fugue_version_i import MAX_LEVELS  # noqa: E402
from per_level_bsp import BI_ZHONGSHU_LEVEL_ID  # noqa: E402


def main() -> None:
    opens, highs, lows, closes, _years = load_ohlc(SYMBOL_FILES["OKLO"])
    n = len(closes)

    orch = R.RecursiveOrchestrator(max_levels=MAX_LEVELS)

    # B 跟踪：ladder2 每次 stroke 增长时记录 (type1 seg_idx, n_segs) 距离分布
    tail_dist: dict[int, int] = {}
    last_sc = 0
    sample_every = 1  # stroke 增长 bar 全采
    for i in range(n):
        orch.process_bar(opens[i], highs[i], lows[i], closes[i])
        sc = orch.stroke_count()
        if sc > last_sc:
            last_sc = sc
            bsps2 = orch.current_bi_zhongshu_buysellpoints_inc(BI_ZHONGSHU_LEVEL_ID)
            # confirmed 笔数 = 引擎喂入增量引擎的段数（最后一笔 unconfirmed 不入）
            n_segs2 = sc - 1
            for b in bsps2:
                if b[0][0] == "type1":
                    d = n_segs2 - 1 - b[0][3]   # 距末段距离
                    d = min(d, 5)               # 桶：0..4, ≥5 归 5
                    tail_dist[d] = tail_dist.get(d, 0) + 1
        if i % 100_000 == 0:
            print(f"  bar {i:,}", flush=True)

    print("\n== B. ladder2 type1 锚距末段距离分布（每 stroke-增长bar × 在列 type1 计数）==")
    for d in sorted(tail_dist):
        label = f"{d}" if d < 5 else ">=5"
        print(f"  距末段 {label}: {tail_dist[d]}")

    # A：递归层终态 c 段空性
    recursive = orch.current_recursive()
    for (lid, zhs, mvs) in recursive:
        ladder = lid + 2
        print(f"\n== A. ladder{ladder} (level_id={lid}) trend move c 段 ==")
        # zhs: z[2]=seg_start z[3]=seg_end z[5]=settled
        for mi, m in enumerate(mvs):
            head = m[0]  # (kind, dir, seg_start, seg_end, zs_start, zs_end, zs_count, settled)
            kind, _dir, seg_s, seg_e, zs_s, zs_e, zs_c, settled = head
            if kind != "trend" or zs_c < 2:
                continue
            settled_idx = [k for k in range(zs_s, min(zs_e + 1, len(zhs)))
                           if zhs[k][5]]
            if len(settled_idx) < 2:
                print(f"  mv#{mi}: settled zs <2 → div 不可能")
                continue
            zlast = zhs[settled_idx[-1]]
            c_start, c_end = zlast[3] + 1, seg_e
            print(f"  mv#{mi} dir={_dir} settled={settled} zs_count={zs_c} "
                  f"zs_last.seg_end={zlast[3]} mv.seg_end={seg_e} "
                  f"→ c=[{c_start},{c_end}] {'空!' if c_start > c_end else '非空'}")


if __name__ == "__main__":
    main()
