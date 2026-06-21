"""高级别 BSP 稀疏性诊断 —— 递归层走势完美/中枢的扇入级联结构分解。

任务（2026-06-15）：ES 1s 11.77M bar，候选分布 move(L1)=11554 / recL2=545 /
recL3=43 / recL4=6（95% 落 move L1）。诊断 recL2+ 为何这么少。

与 unn_1s_a0_verdict.md 的 nest_arms（候选武装事件计数）不同，本脚本读**结构骨架**：
每个递归层的 **distinct 中枢数** + **distinct 走势完美数（settled move）** + **走势完美中
trend(≥2同向中枢) vs consolidation 拆分**。settled 组件 append-only ⇒ 最终
current_recursive() 的 settled 计数 = 全流累计 distinct 数（无需逐 bar 累加）。

ladder 映射核对（Q3）：current_recursive() 的 lid → ladder = lid+2。
  lid=2 → recL2(ladder4)，lid=3 → recL3(ladder5)，lid=4 → recL4(ladder6)。
ladder2/3 的「segment/move(L1)」骨架另读 current_zhongshus()/current_moves()。

用法：.venv/bin/python analysis/diagnose_high_level_bsp.py [BT_MAX_BARS]
"""
from __future__ import annotations

import json
import os
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as R  # noqa: E402

DATA = ROOT / "analysis" / "data_cache" / "es_1s_databento_1y.json"

# move head 元组 index：(kind, direction, seg_start, seg_end, zs_start, zs_end, zs_count, settled)
H_KIND, H_DIR, H_SS, H_SE, H_ZS0, H_ZS1, H_ZSC, H_SETTLED = range(8)


def count_moves(moves: list) -> dict:
    """move 列表 → {total, settled, settled_trend, settled_consol}。

    moves 元素 = (head_tuple, ...) 或 head_tuple；统一取 head。
    走势完美 = settled=True；trend ⟺ zs_count≥2（17课：≥2 同向中枢）。
    """
    total = settled = settled_trend = settled_consol = 0
    for m in moves:
        h = m[0] if isinstance(m[0], tuple) else m
        total += 1
        if h[H_SETTLED]:
            settled += 1
            if h[H_ZSC] >= 2:
                settled_trend += 1
            else:
                settled_consol += 1
    return dict(total=total, settled=settled,
                settled_trend=settled_trend, settled_consol=settled_consol)


def count_zhongshus(zhs: list) -> dict:
    """中枢列表 → {total, settled}。LevelZhongshu 元组 idx5 = settled（per_level_bsp 口径）。

    current_zhongshus()（ladder2 笔中枢 / 实为 segment 标签）与 recursive zhs 共用
    (zd, zg, comp_start, comp_end, comp_count, settled, ...) 布局，idx5=settled。
    """
    total = len(zhs)
    settled = sum(1 for z in zhs if (z[5] if len(z) > 5 else False))
    return dict(total=total, settled=settled)


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
    step = max(1, n // 20)
    for i in range(n):
        o, h, l, c = opens[i], highs[i], lows[i], closes[i]
        if o > 0 and h > 0 and l > 0 and c > 0:
            orch.process_bar(o, h, l, c)
        if i and i % step == 0:
            print(f"  [{i/n*100:4.0f}%] bar {i:,}  {time.time()-t1:.0f}s", flush=True)
    elapsed = time.time() - t1
    print(f"[run] {n:,} bars in {elapsed:.0f}s ({n/elapsed/1e6:.2f}M bar/s)", flush=True)

    # ── 结构骨架（最终状态 = 全流累计 distinct，因 settled append-only）──
    report = {}

    # ladder1 笔：无中枢/无 BSP（仅 PH），只读笔数
    n_strokes = orch.stroke_count()
    report["ladder1_bi"] = dict(n_strokes=n_strokes)

    # ladder2「segment」= 笔中枢（current_zhongshus 是线段中枢；笔中枢另在
    # bi_zhongshu 引擎。此处读两者区分）
    seg_zhs = orch.current_zhongshus()          # 线段中枢（segment→zhongshu）
    report["seg_zhongshu(线段中枢)"] = count_zhongshus(seg_zhs)

    bi_zhs_bsp = orch.current_bi_zhongshu_buysellpoints(1)  # 笔中枢 BSP（level_id=1）
    report["ladder2_bi_zhongshu_bsp"] = dict(n_bsp=len(bi_zhs_bsp))

    # ladder3 move(L1)：线段→中枢→走势
    mv_l1 = orch.current_moves()
    report["ladder3_move_L1"] = count_moves(mv_l1)
    report["ladder3_move_L1"]["n_seg_zhongshu"] = len(seg_zhs)
    bsp_l1 = orch.current_buysellpoints()
    report["ladder3_move_L1"]["n_bsp"] = len(bsp_l1)

    # ladder4+ 递归层：current_recursive() = [(lid, zhs, mvs), ...]
    rec = orch.current_recursive()
    for (lid, zhs, mvs) in rec:
        ladder = lid + 2
        key = f"ladder{ladder}_recL{lid}"
        d = count_moves(mvs)
        d["n_zhongshu"] = len(zhs)
        d["n_zhongshu_settled"] = count_zhongshus(zhs)["settled"]
        report[key] = d

    print("\n" + "=" * 78)
    print("  ES 1s 高级别 BSP 稀疏性诊断 —— 递归层结构骨架（distinct, 全流累计）")
    print("=" * 78)
    print(json.dumps(report, ensure_ascii=False, indent=2))

    # ── 扇入级联表 ──
    print("\n" + "-" * 78)
    print("  扇入级联（每层 settled 走势完美数 + 相对下层压缩比）")
    print("-" * 78)
    chain = [("move(L1)", report["ladder3_move_L1"]["settled"])]
    for (lid, zhs, mvs) in rec:
        chain.append((f"recL{lid}", report[f"ladder{lid+2}_recL{lid}"]["settled"]))
    prev = None
    for name, s in chain:
        ratio = f"  扇入 {prev/s:.1f}:1" if (prev and s) else ""
        print(f"  {name:12s} settled走势完美 = {s:6d}{ratio}")
        prev = s

    outp = ROOT / "analysis" / "data_cache" / "high_level_bsp_diag_ES1S.json"
    outp.write_text(json.dumps(dict(n_bars=n, elapsed=elapsed, report=report,
                                    chain=chain), ensure_ascii=False, indent=1))
    print(f"\n写入 {outp}  总耗时 {time.time()-t0:.0f}s")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
