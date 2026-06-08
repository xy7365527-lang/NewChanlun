#!/usr/bin/env python3
"""诊断 moves_from_zhongshus 的聚合逻辑 — 中枢方向序列实证审计。

存在论位置 / 信息增量
---------------------
观察事实：5min 数据出 8 个中枢被 `moves_from_zhongshus` 收敛成 2 个走势 → L2=0。
质疑：聚合是否"太激进"？

本脚本不臆断，而是把引擎跑到末根，**dump 中枢序列 + 逐对相邻关系 + 走势分组**，
精确回答：8 个中枢是否真的全同向递升（若是 → 2 走势符合缠论趋势定义，L2=0 是真实结构
结果；若中枢方向有交替却被合并 → 聚合 bug）。

缠论定义依据
------------
- 趋势 = 至少 2 个**依次同向**中枢（第17课）。上涨：后枢 ZD > 前枢 ZG（区间递升不重叠）。
- 本级别中枢 = 至少 **3 个连续次级别走势类型**的重叠（第17课）→ L2 需 L1 有 ≥3 走势。
- `_is_ascending(c1,c2) = c2.zd > c1.zg`；`_is_descending(c1,c2) = c2.zg < c1.zd`；
  二者皆否 = 区间重叠（趋势截断点）。

认识论等级
----------
- 中枢/走势序列 dump：L2（真实 OHLCV 上的结构计数，可否证"聚合 bug"假设）。
- 反事实"每中枢一走势 → L2 能否起来"：L0（纯递归算术推演，不构成经验验证）。

约束：.venv/bin/python；不交互提问。

用法：
    .venv/bin/python scripts/diagnose_moves_aggregation.py --data qqq_5m_ohlcv_60d.json
    .venv/bin/python scripts/diagnose_moves_aggregation.py --data qqq_5m_av.json
"""
from __future__ import annotations

import argparse
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

from newchan.orchestrator.recursive import RecursiveOrchestrator
from newchan.types import Bar
from newchan.a_move_v1 import _is_ascending, _is_descending, _filter_settled

DATA = ROOT / "analysis" / "data_cache"


def load_bars(fname: str) -> list[Bar]:
    raw = json.loads((DATA / fname).read_text(encoding="utf-8"))
    bars: list[Bar] = []
    for r in raw["bars"]:
        ts = datetime.fromisoformat(r["ts"])
        if ts.tzinfo is not None:
            ts = ts.astimezone(timezone.utc).replace(tzinfo=None)
        bars.append(Bar(
            ts=ts, open=float(r["open"]), high=float(r["high"]),
            low=float(r["low"]), close=float(r["close"]),
            volume=float(r["volume"]) if r.get("volume") is not None else None,
        ))
    return bars


def pair_relation(c1, c2) -> str:
    """相邻中枢关系：ascending / descending / overlap。"""
    if _is_ascending(c1, c2):
        return "ascending"   # c2.zd > c1.zg，区间递升不重叠
    if _is_descending(c1, c2):
        return "descending"  # c2.zg < c1.zd，区间递降不重叠
    return "overlap"          # 区间重叠 → 趋势截断点


def diagnose(fname: str, max_levels: int = 6) -> dict:
    bars = load_bars(fname)
    orch = RecursiveOrchestrator(stream_id="diag", max_levels=max_levels,
                                 enable_macd_divergence=False)
    snap = None
    for bar in bars:
        snap = orch.process_bar(bar)
    assert snap is not None

    zss = list(snap.zs_snapshot.zhongshus)
    moves = list(snap.move_snapshot.moves)
    settled_indices, settled_zs = _filter_settled(zss)

    # 逐对相邻 settled 中枢关系
    relations: list[dict] = []
    for k in range(1, len(settled_zs)):
        rel = pair_relation(settled_zs[k - 1], settled_zs[k])
        relations.append({
            "from_idx": settled_indices[k - 1], "to_idx": settled_indices[k],
            "relation": rel,
            "c1": [round(settled_zs[k-1].zd, 2), round(settled_zs[k-1].zg, 2)],
            "c2": [round(settled_zs[k].zd, 2), round(settled_zs[k].zg, 2)],
        })

    n_asc = sum(1 for r in relations if r["relation"] == "ascending")
    n_desc = sum(1 for r in relations if r["relation"] == "descending")
    n_overlap = sum(1 for r in relations if r["relation"] == "overlap")

    # 方向翻转次数（asc↔desc 切换，overlap 也算断点）
    direction_flips = 0
    last_dir = ""
    for r in relations:
        d = r["relation"]
        if d in ("ascending", "descending") and last_dir and d != last_dir:
            direction_flips += 1
        if d != "overlap":
            last_dir = d

    # 递归级别有效深度
    rsnaps = {rs.level_id: rs for rs in snap.recursive_snapshots}
    level_struct = {
        1: {"zhongshus": len(zss), "settled_zhongshus": len(settled_zs),
            "moves": len(moves)},
    }
    for lvl in range(2, max_levels + 1):
        rs = rsnaps.get(lvl)
        if rs is None:
            continue
        level_struct[lvl] = {"zhongshus": len(rs.zhongshus), "moves": len(rs.moves)}

    return {
        "data": fname, "n_bars": len(bars),
        "span": f"{bars[0].ts} ~ {bars[-1].ts}",
        "total_zhongshus": len(zss), "settled_zhongshus": len(settled_zs),
        "moves": [
            {"kind": m.kind, "direction": m.direction,
             "zs_start": m.zs_start, "zs_end": m.zs_end, "zs_count": m.zs_count,
             "high": round(m.high, 2), "low": round(m.low, 2), "settled": m.settled}
            for m in moves
        ],
        "pair_relations": relations,
        "rel_counts": {"ascending": n_asc, "descending": n_desc, "overlap": n_overlap},
        "direction_flips": direction_flips,
        "level_struct": level_struct,
    }


def print_report(d: dict) -> None:
    print("═" * 70)
    print(f"数据: {d['data']}  |  {d['n_bars']} 根  |  {d['span']}")
    print("═" * 70)
    print(f"\nL1 中枢总数: {d['total_zhongshus']}  (settled: {d['settled_zhongshus']})")
    print(f"L1 走势数: {len(d['moves'])}")
    print(f"\n── 走势分组 ──")
    for i, m in enumerate(d["moves"]):
        print(f"  Move[{i}] {m['kind']:13s} dir={m['direction']:4s} "
              f"中枢[{m['zs_start']}..{m['zs_end']}] ×{m['zs_count']} "
              f"区间[{m['low']}, {m['high']}] settled={m['settled']}")
    print(f"\n── 相邻 settled 中枢关系（聚合判据）──")
    for r in d["pair_relations"]:
        tag = {"ascending": "↑递升", "descending": "↓递降", "overlap": "⊗重叠(截断)"}[r["relation"]]
        print(f"  中枢{r['from_idx']}{r['c1']} → 中枢{r['to_idx']}{r['c2']}: {tag}")
    rc = d["rel_counts"]
    print(f"\n── 关系统计 ──")
    print(f"  递升: {rc['ascending']}  递降: {rc['descending']}  重叠截断: {rc['overlap']}")
    print(f"  方向翻转次数: {d['direction_flips']}")
    print(f"\n── 递归级别结构 ──")
    for lvl, s in d["level_struct"].items():
        extra = f"  settled中枢={s.get('settled_zhongshus')}" if lvl == 1 else ""
        print(f"  L{lvl}: 中枢={s['zhongshus']}  走势={s['moves']}{extra}")

    # 判定
    print(f"\n── 诊断判定 ──")
    n_moves = len(d["moves"])
    if rc["overlap"] == 0 and d["direction_flips"] <= 1:
        print("  ✓ 中枢序列近乎单边同向（无重叠截断，方向翻转≤1）")
        print(f"    → {d['settled_zhongshus']} 中枢合并成 {n_moves} 走势 = 缠论趋势定义的真实结果")
        print("    → 聚合非 bug；L2=0 因单边行情缺乏足够异向走势类型")
    else:
        print(f"  ⚠ 中枢序列有 {rc['overlap']} 处重叠 + {d['direction_flips']} 次方向翻转")
        print(f"    → 但仍只产出 {n_moves} 走势，需核对截断是否生效")
    print("═" * 70)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--data", default="qqq_5m_ohlcv_60d.json")
    ap.add_argument("--json-out", default=None, help="可选：dump 诊断 JSON 到此路径")
    args = ap.parse_args()
    d = diagnose(args.data)
    print_report(d)
    if args.json_out:
        Path(args.json_out).write_text(json.dumps(d, ensure_ascii=False, indent=2),
                                       encoding="utf-8")
        print(f"[✓] JSON 写入 {args.json_out}")


if __name__ == "__main__":
    main()
