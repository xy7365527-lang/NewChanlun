"""700.HK 见顶后下跌窗口 settle 阶梯 —— 聚焦 $677.5 历史顶之后的下跌段。

存在论位置
----------
本脚本与 `hk700_settle_ladder.py` 共用同一套引擎（OnlineMergeTree + settle_triggers），
但**聚焦点不同**：

  hk700_settle_ladder.py  —— 全量 2 年日线，锚点过滤（settled_after_anchor），
                              对照全局底锚点 vs 见顶回撤锚点。
  本脚本                  —— 只分析"$677.5 见顶之后的下跌段"，用两套互校方案读出
                              "这轮下跌逐级确认结束"的 settle 阶梯。

方案A（主）：从 peak_idx 切片到末尾，**新建一棵 OnlineMergeTree 逐根喂入**，所有
            alive 分量天然都是"这轮下跌的组成部分"（子窗口里没有更早的历史腿）。
方案B（交叉校验）：全量数据跑 OnlineMergeTree，再过滤 birth_idx >= peak_idx 的 alive
            分量。两套结果对比（数量/valley/settle_price），互相印证。

引擎调用约定（严格照搬 hk700_settle_ladder.py）
-----------------------------------------------
1. OnlineMergeTree() 无参构造；逐根 `tree.update(close)` 喂入——**只喂 close**
   （hk700_settle_ladder.py 第127/141/260-271 行均只喂 closes，highs/lows 不进树）。
2. settle_triggers(tree) **必须在 finalize 之前**调用（live 活树上读 alive 分量）。
3. tree.finalize() 封顶全局分量；finalize 后无 alive，只取 all_bars/settled_bars。
4. SettleTrigger 字段：birth_idx / birth_price(=valley) / settle_price /
   settle_persistence / current_cap / current_persistence / last_price /
   gap_to_settle / is_dominant / can_settle_by_rebound / reversal_amplitude。
5. MergeBar 字段：birth_price / death_price / persistence / birth_idx /
   death_idx / lo / hi / settled，property span / is_global。

三档分类（按 settle_persistence = |settle价 − valley|）
------------------------------------------------------
- 近端：最近诞生、最小 settle_persistence 的下跌腿（最近一段小回调腿）。
- 中端：中等 settle_persistence 的下跌腿（缠论次级别走势）。
- 远端：这轮下跌的主导分量（valley≈425，settle 需回到≈677.5 那条，缠论本级别）。
  注意主导分量通常是全局最低分量 → can_settle_by_rebound=False、settle_price=None，
  此时用 current_persistence / reversal_amplitude 表征（因果屏障：反弹不能 settle 它，
  只能反向否定）——绝不伪装成"反弹到 X 即 settle"（a_settle_trigger 模块顶部 090号）。

认识论等级（formalization-validity-domain）
-------------------------------------------
- 数据：真实 yfinance 0700.HK 日线（L2 底材，来自缓存）。
- OnlineMergeTree / settle_triggers 算法：L0（确定性，模块顶部已证）。
- "settle 阶梯 = 逐级确认下跌结束"的操盘翻译：L2（单标的，可否证——未来创新低改写阈值）。
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

from newchan.a_online_persistence import OnlineMergeTree  # noqa: E402
from newchan.a_settle_trigger import settle_triggers  # noqa: E402

CACHE = ROOT / "analysis" / "data_cache"
CACHE_FILE = CACHE / "HK700_1d_2y.json"
OUT_FILE = CACHE / "hk700_decline_window.json"


# ====================================================================
# 数据加载（只读缓存，不联网，不 import yfinance）
# ====================================================================


def load_cache() -> dict:
    """读 HK700_1d_2y.json。结构由 hk700_settle_ladder.pull() 写入：
    keys = symbol / closes / highs / lows / opens / dates（并行数组）。"""
    data = json.loads(CACHE_FILE.read_text())
    return data


def describe_cache(data: dict) -> list[str]:
    lines = [f"缓存文件: {CACHE_FILE}"]
    lines.append(f"top keys: {list(data.keys())}")
    for k, v in data.items():
        if isinstance(v, list):
            head = v[:2]
            tail = v[-2:]
            lines.append(f"  {k}: list[{len(v)}]  head={head}  tail={tail}")
        else:
            lines.append(f"  {k} = {v!r}")
    return lines


# ====================================================================
# 见顶定位（在引擎消费的同一价格序列 closes 上 argmax）
# ====================================================================


def locate_peak(data: dict) -> dict:
    """在 closes（引擎实际喂入的序列）上取 argmax 定位见顶点。"""
    closes = data["closes"]
    dates = data["dates"]
    peak_idx = max(range(len(closes)), key=lambda i: closes[i])
    return {
        "peak_idx": peak_idx,
        "peak_date": dates[peak_idx],
        "peak_price": round(closes[peak_idx], 4),
    }


# ====================================================================
# alive 分量 → 结果行（照搬 SettleTrigger 字段名）
# ====================================================================


def trigger_row(t, peak_idx: int, dates_for_tree: list[str], birth_offset: int) -> dict:
    """把一个 SettleTrigger 转成 immutable dict。字段名严格照搬 a_settle_trigger。

    坐标系约定（修正 sub-window vs 全量的索引歧义）
    ------------------------------------------------
    OnlineMergeTree 报告的 ``birth_idx`` 是**相对喂入它的那条序列**的索引：
      - 方案A 喂的是 closes[peak_idx:] → birth_idx ∈ [0, sub_n)，对应日期 sub_dates。
      - 方案B 喂的是全量 closes      → birth_idx ∈ [0, n)，对应日期全量 dates。
    为了让 birth_date 正确、且两方案可按同一坐标对比，引入：
      - dates_for_tree : 与喂入树的序列平行的日期数组（A=sub_dates, B=全量 dates）。
      - birth_offset   : 把 tree 相对 idx 折算成**全量绝对 idx**（A=peak_idx, B=0）。
    ``abs_birth_idx`` = birth_offset + t.birth_idx 是跨方案的统一身份键。
    """
    rel = t.birth_idx
    bd = dates_for_tree[rel] if 0 <= rel < len(dates_for_tree) else "?"
    abs_idx = birth_offset + rel
    return {
        "birth_idx": rel,
        "abs_birth_idx": abs_idx,
        "birth_date": bd,
        "valley": round(t.birth_price, 2),
        "settle_price": None if t.settle_price is None else round(t.settle_price, 2),
        "settle_persistence": None if t.settle_persistence is None
        else round(t.settle_persistence, 2),
        "current_cap": round(t.current_cap, 2),
        "current_persistence": round(t.current_persistence, 2),
        "gap_to_settle": None if t.gap_to_settle is None else round(t.gap_to_settle, 2),
        "is_dominant": t.is_dominant,
        "can_settle_by_rebound": t.can_settle_by_rebound,
        "reversal_amplitude": None if t.reversal_amplitude is None
        else round(t.reversal_amplitude, 2),
        "born_after_peak": abs_idx >= peak_idx,
    }


# ====================================================================
# 方案A：子窗口（peak_idx → 末尾，新建树逐根喂入）
# ====================================================================


def plan_a_subwindow(data: dict, peak_idx: int) -> dict:
    """从 peak_idx 切片到末尾（含峰值那根），新建 OnlineMergeTree 逐根喂入。

    子窗口里没有更早的历史腿 → 所有 alive 分量天然是"这轮下跌的组成部分"。
    settle_triggers 在 finalize 之前调用（live 活树）。
    """
    closes = data["closes"]
    dates = data["dates"]
    sub_closes = closes[peak_idx:]          # 含峰值那根
    sub_dates = dates[peak_idx:]

    tree = OnlineMergeTree()
    for p in sub_closes:                     # 只喂 close（照搬约定）
        tree.update(p)

    trigs = settle_triggers(tree)            # 必须在 finalize 之前
    snap = tree.current_barcode()
    final = tree.finalize()                  # 封顶后只用于统计 settled

    # settle 阶梯：按 settle_price 升序（最先被反弹触及的在前），全局分量(None)殿后。
    # 方案A 喂的是子窗口 → 日期用 sub_dates，birth_offset=peak_idx（折算回绝对 idx）。
    rows = [trigger_row(t, peak_idx, sub_dates, peak_idx) for t in trigs]
    rows_sorted = sorted(
        rows, key=lambda r: (r["settle_price"] is None, r["settle_price"] or 0.0)
    )

    return {
        "sub_n_bars": len(sub_closes),
        "sub_date_start": sub_dates[0],
        "sub_date_end": sub_dates[-1],
        "sub_price_start": round(sub_closes[0], 2),
        "sub_price_end": round(sub_closes[-1], 2),
        "sub_price_min": round(min(sub_closes), 2),
        "sub_price_max": round(max(sub_closes), 2),
        "n_alive": len(trigs),
        "n_settleable": sum(1 for t in trigs if t.can_settle_by_rebound),
        "n_globals": sum(1 for t in trigs if not t.can_settle_by_rebound),
        "n_settled_after_finalize": len(final.settled_bars),
        "n_alive_live_snapshot": len(snap.alive_bars),
        "alive_ladder": rows_sorted,
    }


# ====================================================================
# 方案B：全量过滤（全量树 → 只留 birth_idx >= peak_idx 的 alive 分量）
# ====================================================================


def plan_b_fullfilter(data: dict, peak_idx: int) -> dict:
    """全量数据跑 OnlineMergeTree，settle_triggers 后只保留 birth >= peak 的分量。"""
    closes = data["closes"]
    dates = data["dates"]

    tree = OnlineMergeTree()
    for p in closes:                         # 全量，只喂 close
        tree.update(p)

    trigs_all = settle_triggers(tree)        # finalize 之前
    trigs_after = [t for t in trigs_all if t.birth_idx >= peak_idx]

    # 方案B 喂的是全量 → 日期用全量 dates，birth_offset=0（birth_idx 本就是绝对 idx）。
    rows = [trigger_row(t, peak_idx, dates, 0) for t in trigs_after]
    rows_sorted = sorted(
        rows, key=lambda r: (r["settle_price"] is None, r["settle_price"] or 0.0)
    )

    return {
        "n_alive_total_full": len(trigs_all),
        "n_alive_after_peak": len(trigs_after),
        "alive_ladder_after_peak": rows_sorted,
        # 完整全量阶梯（用于看主导/全局分量——它 birth 在 peak 之前，过滤会丢掉它）
        "alive_ladder_full": sorted(
            [trigger_row(t, peak_idx, dates, 0) for t in trigs_all],
            key=lambda r: (r["settle_price"] is None, r["settle_price"] or 0.0),
        ),
    }


# ====================================================================
# 方案对比
# ====================================================================


def compare_plans(plan_a: dict, plan_b: dict) -> dict:
    """对比方案A / 方案B 的 alive 分量（数量、valley、settle_price）。"""
    a_rows = plan_a["alive_ladder"]
    b_rows = plan_b["alive_ladder_after_peak"]
    # 用 abs_birth_idx 作统一身份键（方案A/B 折算到同一全量绝对坐标）。
    a_map = {r["abs_birth_idx"]: r for r in a_rows}
    b_map = {r["abs_birth_idx"]: r for r in b_rows}
    all_idx = sorted(set(a_map) | set(b_map))

    diffs = []
    for idx in all_idx:
        ra = a_map.get(idx)
        rb = b_map.get(idx)
        diffs.append({
            "abs_birth_idx": idx,
            "in_plan_a": ra is not None,
            "in_plan_b": rb is not None,
            "a_valley": None if ra is None else ra["valley"],
            "b_valley": None if rb is None else rb["valley"],
            "a_settle_price": None if ra is None else ra["settle_price"],
            "b_settle_price": None if rb is None else rb["settle_price"],
            "valley_match": (ra is not None and rb is not None
                             and ra["valley"] == rb["valley"]),
            "settle_match": (ra is not None and rb is not None
                             and ra["settle_price"] == rb["settle_price"]),
        })

    return {
        "n_plan_a": len(a_rows),
        "n_plan_b": len(b_rows),
        "count_match": len(a_rows) == len(b_rows),
        "rows": diffs,
    }


# ====================================================================
# 三档分类（近端 / 中端 / 远端）
# ====================================================================


def classify_three_bands(plan_a: dict, plan_b: dict, last_close: float) -> dict:
    """按 settle_persistence 把这轮下跌的腿分为近端/中端/远端。

    分类源 = 方案A 的 alive 阶梯（子窗口，天然全是这轮下跌的腿）。
    - 可反弹 settle 的腿（settle_persistence 非 None）按 settle_persistence 排序：
        最小 → 近端；中等 → 中端。
    - 远端 = 这轮下跌的主导分量。子窗口里主导=全局最低分量，settle_price=None，
      用 current_persistence 表征；同时给出全量树里对应的"回到≈677.5"主导分量
      （方案B alive_ladder_full 的全局分量），即缠论本级别下跌走势。
    """
    settleable = [r for r in plan_a["alive_ladder"]
                  if r["settle_persistence"] is not None]
    settleable.sort(key=lambda r: r["settle_persistence"])

    near = settleable[0] if settleable else None
    # 中端：去掉近端后，取中位（settle_persistence 中等者）
    mid = None
    if len(settleable) >= 2:
        rest = settleable[1:]
        mid = rest[len(rest) // 2]

    # 远端（子窗口主导/全局分量）：方案A 里 is_dominant 或 settle_price=None 那条
    far_subwindow = None
    for r in plan_a["alive_ladder"]:
        if r["is_dominant"] or r["settle_price"] is None:
            far_subwindow = r
            break

    # 远端（全量本级别）：方案B 全量阶梯里的全局/主导分量（valley≈最低，回到≈677.5）
    far_fulllevel = None
    for r in plan_b["alive_ladder_full"]:
        if r["is_dominant"] or r["settle_price"] is None:
            far_fulllevel = r
            break

    return {
        "near": near,
        "mid": mid,
        "far_subwindow": far_subwindow,
        "far_fulllevel": far_fulllevel,
        "last_close_used_for_gap": round(last_close, 2),
    }


# ====================================================================
# 打印
# ====================================================================


def fmt_row(r: dict) -> str:
    sp = "—(全局)" if r["settle_price"] is None else f"{r['settle_price']}"
    spers = "—" if r["settle_persistence"] is None else f"{r['settle_persistence']}"
    gap = "—" if r["gap_to_settle"] is None else f"{r['gap_to_settle']}"
    rev = "—" if r["reversal_amplitude"] is None else f"{r['reversal_amplitude']}"
    dom = "★主导" if r["is_dominant"] else ""
    return (f"birth={r['birth_date']}  valley={r['valley']:<8} "
            f"settle={sp:<8} settle幅={spers:<8} gap={gap:<8} "
            f"curP={r['current_persistence']:<8} rebound={r['can_settle_by_rebound']!s:<5} "
            f"rev幅={rev:<8} {dom}")


def main() -> None:
    data = load_cache()
    closes = data["closes"]
    lows = data["lows"]
    dates = data["dates"]

    print("=" * 90)
    print("700.HK 见顶后下跌窗口 settle 阶梯")
    print("=" * 90)

    print("\n--- [0] 缓存结构 ---")
    for line in describe_cache(data):
        print(line)

    peak = locate_peak(data)
    print("\n--- [1] 见顶定位（closes 上 argmax）---")
    print(f"peak_idx   = {peak['peak_idx']}")
    print(f"peak_date  = {peak['peak_date']}")
    print(f"peak_price = {peak['peak_price']}")

    last_close = closes[-1]
    last_low = lows[-1]
    last_date = dates[-1]
    print("\n--- [2] 当前价 ---")
    print(f"last_date  = {last_date}")
    print(f"last_close = {round(last_close, 2)}  (gap 全部用它算)")
    print(f"last_low   = {round(last_low, 2)}")
    print(f"自见顶回撤 = {round((last_close / peak['peak_price'] - 1) * 100, 1)}%")

    plan_a = plan_a_subwindow(data, peak["peak_idx"])
    print("\n--- [3] 方案A（主）子窗口：peak→末尾 新建树逐根喂入 ---")
    print(f"子窗口: {plan_a['sub_date_start']} → {plan_a['sub_date_end']}  "
          f"n={plan_a['sub_n_bars']}")
    print(f"子窗口价格: 起 {plan_a['sub_price_start']} → 终 {plan_a['sub_price_end']}  "
          f"[{plan_a['sub_price_min']}, {plan_a['sub_price_max']}]")
    print(f"alive 分量: {plan_a['n_alive']}  "
          f"({plan_a['n_settleable']} 可反弹 settle + {plan_a['n_globals']} 全局)")
    print("alive 阶梯（settle_price 升序，全局殿后）：")
    for r in plan_a["alive_ladder"]:
        print("   " + fmt_row(r))

    plan_b = plan_b_fullfilter(data, peak["peak_idx"])
    print("\n--- [4] 方案B（交叉校验）全量树 → 过滤 birth_idx >= peak ---")
    print(f"全量 alive 分量: {plan_b['n_alive_total_full']}  "
          f"其中 birth>=peak: {plan_b['n_alive_after_peak']}")
    print("方案B 过滤后（birth>=peak）阶梯：")
    for r in plan_b["alive_ladder_after_peak"]:
        print("   " + fmt_row(r))
    print("方案B 全量阶梯（含 birth<peak 的主导/全局分量——它是缠论本级别下跌腿）：")
    for r in plan_b["alive_ladder_full"]:
        print("   " + fmt_row(r))

    cmp = compare_plans(plan_a, plan_b)
    print("\n--- [5] 方案A vs 方案B 对比 ---")
    print(f"方案A alive 数={cmp['n_plan_a']}  方案B(过滤后) alive 数={cmp['n_plan_b']}  "
          f"数量一致={cmp['count_match']}")
    print("逐分量对比（abs_birth_idx / valley / settle_price）：")
    for d in cmp["rows"]:
        print(f"   abs_idx={d['abs_birth_idx']:<4} A={d['in_plan_a']!s:<5} B={d['in_plan_b']!s:<5} "
              f"valley A={d['a_valley']} B={d['b_valley']} (match={d['valley_match']})  "
              f"settle A={d['a_settle_price']} B={d['b_settle_price']} (match={d['settle_match']})")

    bands = classify_three_bands(plan_a, plan_b, last_close)
    print("\n--- [6] 三档分类（按 settle_persistence）---")

    def show_band(name: str, r: dict | None) -> None:
        print(f"  [{name}]")
        if r is None:
            print("     （无）")
            return
        print("     " + fmt_row(r))

    show_band("近端（最近一段小回调腿，最小 settle_persistence）", bands["near"])
    show_band("中端（缠论次级别走势，中等 settle_persistence）", bands["mid"])
    show_band("远端·子窗口主导（这轮下跌主导分量，反弹不能 settle）",
              bands["far_subwindow"])
    show_band("远端·全量本级别（valley≈最低，settle 需回到≈677.5，缠论本级别下跌）",
              bands["far_fulllevel"])

    result = {
        "symbol": data["symbol"],
        "cache_file": str(CACHE_FILE),
        "n_bars_total": len(closes),
        "date_start": dates[0],
        "date_end": dates[-1],
        "peak": peak,
        "current": {
            "last_date": last_date,
            "last_close": round(last_close, 2),
            "last_low": round(last_low, 2),
            "drawdown_from_peak_pct": round(
                (last_close / peak["peak_price"] - 1) * 100, 1),
        },
        "plan_a_subwindow": plan_a,
        "plan_b_fullfilter": plan_b,
        "compare": cmp,
        "three_bands": bands,
        "epistemic_level": {
            "data": "L2 (真实 yfinance 0700.HK 日线, 缓存)",
            "online_merge_tree_algo": "L0 (确定性)",
            "settle_ladder_trading_translation": "L2 (单标的, 可否证)",
        },
    }
    OUT_FILE.write_text(json.dumps(result, ensure_ascii=False, indent=2))
    print(f"\n已写 {OUT_FILE}")


if __name__ == "__main__":
    main()
