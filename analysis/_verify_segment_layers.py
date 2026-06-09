"""bit-exact 验证：orchestrator 线段级增量（中枢/走势/买卖点）≡ standalone 全量 batch oracle。

RecursiveOrchestrator.process_bar 内部对线段级中枢/走势/买卖点层做**增量**计算
（IncrementalSegZhongshu 续扫易变段尾 + IncrementalSegDivergences 窗口重算 + IncrementalBsp）。
oracle = 未改动的 standalone 函数全量管线（每 bar 在 current_strokes() 上重跑）：
  segments_from_strokes_v1 → zhongshu_from_segments → moves_from_zhongshus(+attach_persistence)
  → divergences_from_moves_v1 → buysellpoints_from_level

在**每个结构变化 bar**（move_epoch / bsp_epoch 自增）对比 orchestrator 三层输出 vs oracle。
变化 bar 集合完整覆盖 E 引擎（消费 current_moves 的 settle 事件）与 I 引擎（消费 bsp）的输入差异
——未变 bar 输出按构造逐位不变（accessor 按引用返回缓存），故验证变化 bar = 验证全部消费点。

运行：PYTHONPATH=src .venv/bin/python analysis/_verify_segment_layers.py [n_bars]
"""
from __future__ import annotations

import json
import sys
import time
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE.parent / "src"))
import newchan_rust as R  # noqa: E402

_DATA = _HERE / "data_cache" / "oklo_1m_databento.json"


def _seg_input(seg_tuples):
    """SegmentTuple → SegInput (dir, high, low, i0, i1)。s[0]=(s0,s1,i0,i1,dir,high,low,confirmed,kind)。"""
    return [(s[0][4], s[0][5], s[0][6], s[0][2], s[0][3]) for s in seg_tuples]


def _zs_for_zhongshu(seg_tuples):
    """SegmentTuple → zhongshu_from_segments 输入 (s0,s1,high,low,confirmed,kind=='settled')。"""
    return [(s[0][0], s[0][1], s[0][5], s[0][6], s[0][7], s[0][8] == "settled") for s in seg_tuples]


def oracle_zhongshu(seg_tuples):
    """全量 zhongshu_from_segments(当前线段)。验证 inc 中枢 == 全量（同一线段输入）。"""
    return R.zhongshu_from_segments(_zs_for_zhongshu(seg_tuples))


def oracle_moves(seg_tuples, zs_tuples):
    """全量 moves_from_zhongshus(当前中枢)+attach_persistence（同 orchestrator 走势层逻辑）。"""
    moves = R.moves_from_zhongshus(zs_tuples, len(seg_tuples))
    return R.attach_persistence(moves, zs_tuples)


def oracle_bsp(seg_tuples, zs_tuples, mv_tuples):
    """全量 buysellpoints_from_level(当前线段/中枢/走势)。验证 inc 买卖点 == 全量（同一 gated 输入）。

    关键：喂 orchestrator **自己的** current_segments/zhongshus/moves（gated 状态），
    隔离 bsp 增量正确性——不受 move_zs_key 门控陈旧性干扰（该门控行为旧代码同款，非本次改动）。
    """
    seg_in = _seg_input(seg_tuples)
    zs_div = [(z[0], z[1], z[2], z[3], z[5]) for z in zs_tuples]
    mv_in = [m[0] for m in mv_tuples]
    divs = R.divergences_from_moves_v1(seg_in, zs_div, mv_in, 1)
    zs_bsp = [(z[0], z[1], z[2], z[3], z[5], z[7], z[6]) for z in zs_tuples]
    div_in = [(d[0][0], d[0][1], d[0][7], d[0][5], d[0][6], d[1][0], d[1][1]) for d in divs]
    return R.buysellpoints_from_level(seg_in, zs_bsp, mv_in, div_in, 1)


def main():
    n = int(sys.argv[1]) if len(sys.argv) > 1 else 0
    raw = json.loads(_DATA.read_text())
    bars = raw["bars"]
    if n > 0:
        bars = bars[:n]
    o = [float(b["open"]) for b in bars]
    h = [float(b["high"]) for b in bars]
    l = [float(b["low"]) for b in bars]
    c = [float(b["close"]) for b in bars]
    nb = len(c)
    print(f"bars={nb:,}  对比 orchestrator 增量 vs standalone 全量（喂 orchestrator gated 输入，隔离门控）")

    orch = R.RecursiveOrchestrator(max_levels=6)  # enable_bsp=True, enable_macd=False → use_inc_bsp
    pm, pb = -1, -1
    n_zs_chk = n_mv_chk = n_bsp_chk = 0
    t0 = time.time()
    for i in range(nb):
        orch.process_bar(o[i], h[i], l[i], c[i])
        me, be = orch.move_epoch(), orch.bsp_epoch()

        # 中枢：每 bar 比对（zhongshu_from_segments 对 unconfirmed 末段不变 → zs_key 门控与
        # 影响中枢的变化精确对齐；若失配即暴露 inc 中枢 bug 或门控漏洞）。
        segs = orch.current_segments()
        izss = orch.current_zhongshus()
        ozss = oracle_zhongshu(segs)
        if izss != ozss:
            print(f"✗ ZHONGSHU 发散 @ bar {i}: inc={len(izss)} oracle={len(ozss)}")
            _first_diff(izss, ozss)
            return 1
        n_zs_chk += 1

        # 走势：仅 move_epoch 变化 bar（此时 moves 刚从当前中枢重算 → 与 oracle 同输入）。
        if me != pm:
            pm = me
            imoves = orch.current_moves()
            omoves = oracle_moves(segs, izss)
            if imoves != omoves:
                print(f"✗ MOVES 发散 @ bar {i}: inc={len(imoves)} oracle={len(omoves)}")
                _first_diff(imoves, omoves)
                return 1
            n_mv_chk += 1

        # 买卖点：仅 bsp_epoch 变化 bar（喂 orchestrator gated 输入，隔离 move 门控陈旧性）。
        if be != pb:
            pb = be
            ibsps = orch.current_buysellpoints()
            obsps = oracle_bsp(segs, izss, orch.current_moves())
            if ibsps != obsps:
                print(f"✗ BSP 发散 @ bar {i}: inc={len(ibsps)} oracle={len(obsps)}")
                _first_diff(ibsps, obsps)
                return 1
            n_bsp_chk += 1

    dt = time.time() - t0
    print(f"✓ bit-exact 全部逐位一致")
    print(f"   zhongshu={n_zs_chk}（每 bar） moves={n_mv_chk}（move_epoch变化） bsp={n_bsp_chk}（bsp_epoch变化）检查点")
    print(f"   末态: n_zs={len(orch.current_zhongshus())} n_mv={len(orch.current_moves())} "
          f"n_bsp={len(orch.current_buysellpoints())}  ({dt:.1f}s)")
    return 0


def _first_diff(a, b):
    for k in range(min(len(a), len(b))):
        if a[k] != b[k]:
            print(f"   首个差异 idx={k}:\n     inc   ={a[k]}\n     oracle={b[k]}")
            return
    print(f"   长度差异: inc={len(a)} oracle={len(b)}")


if __name__ == "__main__":
    sys.exit(main())
