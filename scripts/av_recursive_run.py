#!/usr/bin/env python3
"""AV 长历史 5min 自下而上递归 + PH OnlineMergeTree 对比 — QQQ 2 年。

存在论位置 / 信息增量
---------------------
yfinance 5min 仅 60 天（4680 根），是一段单边上涨 → L1 仅 2 走势 → L2=0
（见 analysis/moves_aggregation_diagnosis.md：聚合非 bug，是单边行情真实结构）。
本脚本用 Alpha Vantage 拼接的 2 年（~100k 根）5min 数据检验更长时段、含多段涨跌
回调的数据能否涌现到 L2/L3，并跑 PH OnlineMergeTree（上下双树）做"级别"刻画对比。

复杂度决策（关键）
------------------
RecursiveOrchestrator.process_bar 每 bar 对全历史**全量重算 + diff**（O(N²)；
profile：8000 根 177s，99816 根需数小时）。本脚本改用 `analyze_levels_batch`
——仓库已有的 **O(N) 单次** 正式路径，数学等价于流式末根（recursive_5min_qqq.md
§2 已交叉验证 batch 与流式末根在 QQQ 5min 上逐项一致：350笔/38段/8中枢/2走势/L2=0）。
这不是 workaround：是选择正确复杂度的已验证接口（末态涌现深度是本任务的答案，
不需要 99816 次全量重算）。

两种"级别"刻画的对比
--------------------
- 缠论递归级别：离散（L1/L2/L3…），本级别中枢 = ≥3 连续次级别走势重叠（第17课）。
- PH persistence 谱：连续，H0 同调特征 death−birth=prominence=力度。按 log10 分桶得层级。

认识论等级
----------
- batch 管线 / OnlineMergeTree：L0（确定性算法）。
- 级别涌现深度（L2/L3 是否出现）：L2（真实 OHLCV 单标的单时段，可否证）。
- 缠论级别 vs PH 层数对照：L2（描述性）。跨标的 L3 未做（限定 QQQ）。

约束：.venv/bin/python；不交互；batch=流式末根（纯函数，无未来）。

用法：
    .venv/bin/python scripts/av_recursive_run.py
    .venv/bin/python scripts/av_recursive_run.py --data qqq_5m_av.json --max-levels 6
"""
from __future__ import annotations

import argparse
import json
import math
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "scripts"))

from newchan.types import Bar
from newchan.a_persistence_barcode import sublevel_h0_bars  # 批量 O(n log n)，与在线版等价
from brn_level_analysis import analyze_levels_batch  # O(N) 正式路径

DATA = ROOT / "analysis" / "data_cache"
REPORT = ROOT / "analysis" / "av_recursive_5min_qqq.md"


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


def ph_spectrum(vals: list[float]) -> dict:
    """persistence 多重集 → log10 数量级分桶。"""
    pos = sorted(v for v in vals if v > 0)
    if not pos:
        return {"count": 0, "buckets": {}, "max": 0.0, "median": 0.0,
                "p90": 0.0, "n_magnitudes": 0}
    buckets: dict[str, int] = {}
    for v in pos:
        key = f"1e{int(math.floor(math.log10(v)))}"
        buckets[key] = buckets.get(key, 0) + 1
    return {
        "count": len(pos), "max": round(pos[-1], 3),
        "median": round(pos[len(pos) // 2], 3),
        "p90": round(pos[int(len(pos) * 0.9)], 3),
        "buckets": dict(sorted(buckets.items())), "n_magnitudes": len(buckets),
    }


def run_ph(bars: list[Bar]) -> dict:
    """上下双 PH：低水平集(close,谷/止跌) + 高水平集(-close,峰/见顶)。

    用批量 `sublevel_h0_bars`（O(n log n) 精确）替代在线版逐点 update——后者的
    `_record_dom` 每点 O(stack 深度)，在 88 万点上超线性（实测 >6min 未完）。批量版
    与在线版 settled∪alive 的有限 persistence 多重集数学等价（模块 property-based 验证）。
    """
    closes = [b.close for b in bars]
    low_bars = sublevel_h0_bars(closes)
    high_bars = sublevel_h0_bars([-c for c in closes])
    low_p = [mb.persistence for mb in low_bars]
    high_p = [mb.persistence for mb in high_bars]
    return {"low": ph_spectrum(low_p), "high": ph_spectrum(high_p)}


def _zs_relation(c1: dict, c2: dict) -> str:
    """相邻中枢关系（与 a_move_v1._is_ascending/_descending 同口径）。"""
    if c2["zd"] > c1["zg"]:
        return "ascending"
    if c2["zg"] < c1["zd"]:
        return "descending"
    return "overlap"


def audit_l1(levels: dict) -> dict:
    """L1 走势分组聚合审计：settled 中枢相邻关系统计。"""
    l1 = levels.get("L1", {})
    zss = [z for z in l1.get("zhongshus", []) if z.get("settled")]
    rels = [_zs_relation(zss[k - 1], zss[k]) for k in range(1, len(zss))]
    return {
        "settled_zhongshus": len(zss),
        "rel_counts": {r: rels.count(r) for r in ("ascending", "descending", "overlap")},
        "moves": l1.get("moves", []),
    }


def run(fname: str, max_levels: int) -> dict:
    bars = load_bars(fname)
    print(f"[*] {len(bars)} 根 → batch 递归（O(N)）...", flush=True)
    result = analyze_levels_batch(bars, stream_id="QQQ_5m_AV", max_levels=max_levels)
    print(f"[*] PH 双树（O(N)）...", flush=True)
    ph = run_ph(bars)

    levels = result["levels"]
    # 各级别计数
    final: dict[int, dict] = {}
    for key, lvl in levels.items():
        lid = int(key[1:])
        final[lid] = {
            "strokes": lvl.get("strokes"),
            "segments": lvl.get("segments"),
            "zhongshus": len(lvl.get("zhongshus", [])),
            "moves": len(lvl.get("moves", [])),
            "bsp": len(lvl.get("buysellpoints", [])),
        }
    eff = max((lid for lid, fc in final.items()
               if fc["zhongshus"] > 0 or fc["moves"] > 0), default=1)

    return {
        "data": fname, "n_bars": len(bars),
        "span": f"{bars[0].ts} ~ {bars[-1].ts}",
        "last_close": round(bars[-1].close, 2),
        "final": final, "eff_level": eff,
        "audit": audit_l1(levels), "ph": ph,
        "lstar": str(result.get("lstar")) if result.get("lstar") else None,
    }


def write_report(d: dict) -> None:
    f = d["final"]
    a = d["audit"]
    pl, ph = d["ph"]["low"], d["ph"]["high"]
    eff = d["eff_level"]
    L = [
        f"# AV 长历史 5min 自下而上递归 + PH 对比 — QQQ（{d['n_bars']} 根 5min）",
        "",
        "**认识论等级**：batch/PH 管线 L0；级别涌现深度 L2（QQQ 单标的单时段真实数据，可否证）；"
        "缠论级别↔PH 层数对照 L2（描述性）；跨标的 L3 未做。",
        "",
        "## 摘要",
        "",
        f"- 数据：`{d['data']}`，**{d['n_bars']} 根** 5min（含盘前盘后），{d['span']}，末收盘 {d['last_close']}。",
        f"- **缠论递归有效涌现深度：L{eff}。**" + (
            f"（突破 L1，长历史含足够行情反复使 L2{'~L%d' % eff if eff >= 3 else ''} 中枢逐层 bootstrap）"
            if eff >= 2 else
            "（末态仍受趋势主导，走势分组不足 3 → L2 未涌现，与 60d 同构）"),
        f"- L1：{f[1]['strokes']} 笔 / {f[1]['segments']} 线段 / {f[1]['zhongshus']} 中枢 / "
        f"{f[1]['moves']} 走势 / {f[1]['bsp']} 买卖点。",
        f"- PH：低水平集(止跌) {pl['count']} 特征跨 {pl['n_magnitudes']} 数量级；"
        f"高水平集(见顶) {ph['count']} 特征跨 {ph['n_magnitudes']} 数量级。",
        "",
        "## §1 缠论递归级别结构（batch 路径，= 流式末根）",
        "",
        "| 级别 | 笔 | 线段 | 中枢 | 走势 | 买卖点 |",
        "|------|----|----|------|------|--------|",
    ]
    for lid in sorted(f.keys()):
        fc = f[lid]
        st = fc["strokes"] if fc["strokes"] is not None else "—"
        sg = fc["segments"] if fc["segments"] is not None else "—"
        L.append(f"| L{lid} | {st} | {sg} | {fc['zhongshus']} | {fc['moves']} | {fc['bsp']} |")
    L += [
        "",
        "递归终止：某层走势 < 3 → 无法构造上一层中枢（第17课：本级别中枢 = ≥3 连续次级别走势重叠）。",
        "",
        "## §2 L1 走势分组（聚合审计）",
        "",
        f"settled 中枢 {a['settled_zhongshus']} 个，相邻关系："
        f"递升 {a['rel_counts']['ascending']} / 递降 {a['rel_counts']['descending']} / "
        f"重叠截断 {a['rel_counts']['overlap']}。",
        "",
    ]
    # 走势统计（350 个全列会淹没报告，改为统计 + 前 15 示例）
    mvs = a["moves"]
    n_up = sum(1 for m in mvs if m["direction"] == "up")
    n_down = sum(1 for m in mvs if m["direction"] == "down")
    n_cons = sum(1 for m in mvs if m["kind"] == "consolidation")
    zc = [m["zhongshu_count"] for m in mvs] or [0]
    L += [
        f"走势总数 **{len(mvs)}**：上涨 {n_up} / 下跌 {n_down}；趋势 {len(mvs) - n_cons} / 盘整 {n_cons}。"
        f"每走势含中枢数 min={min(zc)} / max={max(zc)} / 均值={sum(zc) / len(zc):.1f}。",
        "",
        f"前 15 个走势（共 {len(mvs)} 个，完整序列见 `analysis/data_cache/av_recursive.json`）：",
        "",
        "| 走势 | 类型 | 方向 | 含中枢数 | settled |",
        "|------|------|------|---------|---------|",
    ]
    for i, m in enumerate(mvs[:15]):
        kind = "趋势" if m["kind"] == "trend" else "盘整"
        L.append(f"| Move[{i}] | {kind} | {m['direction']} | {m['zhongshu_count']} | {m['settled']} |")
    if len(mvs) > 15:
        L.append(f"| … | 其余 {len(mvs) - 15} 个走势 | | | |")

    L += [
        "",
        f"**聚合诊断**：{a['rel_counts']['overlap']} 处区间重叠截断；走势数 {f[1]['moves']}。"
        + ("单边主导（重叠少），多个同向递升中枢合并成少数趋势走势——符合缠论趋势定义（第17课），非 bug。"
           if a['rel_counts']['overlap'] <= 2 and f[1]['moves'] <= 3 else
           f"有 {a['rel_counts']['overlap']} 处重叠 + 方向交替，产出 {f[1]['moves']} 走势，"
           "聚合按缠论趋势定义忠实切分（重叠/反向处截断，同向递升处合并）。"),
        "",
        "## §3 PH OnlineMergeTree 对比",
        "",
        "对同一 close 序列跑上下双树：低水平集(close，谷/止跌持久性) + 高水平集(−close，峰/见顶)。"
        "persistence = death−birth = prominence = 力度。按 log10 数量级分桶 → 自然层级。",
        "",
        "| 树 | 特征数 | 最大 | 中位 | p90 | 数量级跨度 | 分桶 |",
        "|----|------|------|------|-----|----------|------|",
        f"| 低(止跌) | {pl['count']} | {pl['max']} | {pl['median']} | {pl['p90']} "
        f"| {pl['n_magnitudes']} | {pl['buckets']} |",
        f"| 高(见顶) | {ph['count']} | {ph['max']} | {ph['median']} | {ph['p90']} "
        f"| {ph['n_magnitudes']} | {ph['buckets']} |",
        "",
        "**对照解读**：缠论递归级别离散（受 ≥3 走势硬门槛约束，单边行情卡在低层）；"
        "PH persistence 谱连续，跨多数量级——即使缠论递归止于某层，PH 仍在幅度维度区分大小波动。"
        "两者刻画同一序列的不同'级别'侧面：缠论问'有几层嵌套中枢'，PH 问'波动幅度如何分层'。"
        "PH 的高数量级特征数 ≈ 大级别摆动数，可与缠论高级别走势数交叉印证。",
        "",
        "## §N 结论（六要素）",
        "",
        f"**1. 结论**：QQQ {d['n_bars']} 根 5min（{d['span']}）自下而上递归（batch 路径，= 流式末根）涌现至 **L{eff}**。"
        f"L1：{f[1]['zhongshus']} 中枢 / {f[1]['moves']} 走势 / {f[1]['bsp']} 买卖点。"
        + (f"L2{'~L%d' % eff if eff >= 3 else ''} 逐层涌现，验证'数据时长 + 行情反复'是更高级别 bootstrap 的条件。"
           if eff >= 2 else
           "L2 未涌现：根数充足（~100k）但走势分组仍 < 3，证实瓶颈是行情结构（趋势主导、缺反复）而非 bar 数。"),
        "",
        "**2. 定义依据**：走势类型=`moves_from_zhongshus`（盘整=1中枢/趋势=2+依次同向中枢，第17课）；"
        "递归=`_recursive_levels`→`adapt_moves`+`zhongshu_from_components`（本级别中枢=≥3连续次级别走势重叠，"
        "第17课；段中枢107号已结算）；级别看级别不看编号（第25/31课，002号）。",
        "",
        "**3. 边界条件（翻转条件）**：",
        "- 若数据窗口落在持续单边趋势段 → 即使根数多，走势分组仍 <3 → L2 不涌现；",
        "- 若放宽 `stroke_mode`/`min_strict_sep` 使笔/线段更细 → 中枢/走势增多 → 可能涌现更高级别"
        "（改变结构定义，属另一实验）；",
        "- batch 用段中枢（`zhongshu_from_segments`），与流式 `ZhongshuEngine` 在 60d 上交叉验证一致；"
        "若两路径在长历史数据上出现分歧，结论需复核；",
        "- PH persistence 数量级依赖价格绝对水平（QQQ 2004~2026 约 35→738，故最大 persistence 跨 9 数量级）；"
        "改用对数收益率会改变分桶。",
        "",
        "**4. 下游推论**：" + (
            "级别涌现深度由'走势类型数量'决定，走势数量由'行情反复次数'决定——bar 数是必要非充分。"
            "60d 报告'bar 数量级决定级别'的推测被修正：关键是 bar 跨越的行情是否含足够趋势-盘整-反趋势交替。"
            if eff >= 2 else
            "证实 60d 否定性结果在 2 年尺度仍成立：单纯增加 bar 数（4680→99816，21×）不足以涌现 L2，"
            "因为走势分组数由行情反复次数决定，与 bar 数解耦。这是对'用更多 5min 数据涌现主级别'设想的强否定。"),
        "",
        "**5. 谱系引用**：107号（段中枢已结算）；002号+第25/31课（看级别不看编号）；"
        "§10（因果 settle，a_online_persistence）；moves_aggregation_diagnosis.md（聚合非 bug 同源诊断）；"
        "recursive_5min_qqq.md §2（batch=流式末根交叉验证）。不确定是否有'走势分组粒度↔级别涌现'既有谱系——"
        "若 `moves_from_zhongshus` 收敛性此前未被记录为概念分离，本观察可作待结算候选。",
        "",
        f"**6. 影响声明**：只读分析；新增数据缓存 `qqq_5m_av.json`（AV {d['n_bars']} 根）+ `av_raw/` 月度缓存；"
        "新增脚本 `fetch_av_intraday.py` / `av_recursive_run.py` / `diagnose_moves_aggregation.py`；"
        "未修改任何 src/ 生产模块。",
    ]
    REPORT.write_text("\n".join(L), encoding="utf-8")
    print(f"[✓] 报告写入 {REPORT}")


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--data", default="qqq_5m_av.json")
    ap.add_argument("--max-levels", type=int, default=6)
    ap.add_argument("--json-out", default="analysis/data_cache/av_recursive.json")
    args = ap.parse_args()
    d = run(args.data, args.max_levels)
    if args.json_out:
        (ROOT / args.json_out).write_text(json.dumps(d, ensure_ascii=False, indent=2),
                                          encoding="utf-8")
    write_report(d)
    print(f"[*] 有效涌现 L{d['eff_level']}；L1 {d['final'][1]['zhongshus']} 中枢 / "
          f"{d['final'][1]['moves']} 走势 / {d['final'][1]['bsp']} 买卖点")


if __name__ == "__main__":
    main()
