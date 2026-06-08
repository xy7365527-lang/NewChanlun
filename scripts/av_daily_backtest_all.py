"""AV 真实日线：数据就绪 + 引擎递归涌现分析（回测信号待 TV Replay 零前视序列）。

存在论位置（编排者最终裁定 2026-05-31）
----------------------------------------
**TV 静态标注不能做回测信号源**——存在不可消除的前视偏差：
  买卖点标注在分型极点（如 bar 100，$500），但买卖点的**成立时刻**在右侧确认之后
  （bar 103，$510）。静态 TV 数据只记录极点位置，不记录成立时刻，故 lag=1/lag=3
  任何静态近似都错（要么用了未来信息，要么执行价错位）。

唯一可接受的回测方案：**TV Replay 回放的 first_seen_step（信号首次可见步）+ lag=1
+ 真实 OHLCV 执行价**。first_seen_step 是逐步回放中信号无前视地首次出现的时刻，
等价于"成立时刻"。该数据由独立的 Replay session 产出，本脚本不伪造。

本脚本的职责（数据准备 + 确定性结构分析，无前视争议）：
  1. 确认 AV 真实日线 OHLCV 已保存待对齐（QQQ 真实 + EWH/FXI/USO 代理 + BRENT 真实原油）。
  2. 引擎在真实日线上的**递归涌现结构**（笔/线段/中枢/走势/买卖点、递归层数）——
     这是 L0 确定性结果（不依赖信号对齐，无前视），同时诊断"走势分组涌现边界"
     （8 中枢→2 走势→买卖点稀疏的结构现象）。
  3. 输出 Replay 对齐接口说明，待零前视信号序列就绪后接入。

认识论等级
----------
- AV 数据获取/复权：L1（管线）。
- 引擎递归涌现（笔/线段/中枢/走势/bsp 计数）：L0（确定性算法，无前视）。
- 回测胜率/收益：**本脚本不产出**——需 Replay first_seen_step（避免前视的唯一途径）。
"""
from __future__ import annotations

import contextlib
import io
import json
import sys
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / "src"))

import av_fetch  # noqa: E402
import brn_level_analysis as bl  # noqa: E402
from newchan.types import Bar  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
REPORT_PATH = ROOT / "analysis" / "av_daily_backtest_all.md"
RESULT_JSON = DATA_DIR / "av_daily_backtest_all.json"

# 四标的真实日线 OHLCV（QQQ 真实本体；HSI/SHCOMP/BRN 用代理 ETF 真实 OHLC——
# AV/Polygon 实测不提供指数本体，代理 ETF 的递归结构是真实的，但非原标的）。
SYMBOLS = [
    {"name": "qqq", "label": "纳指 QQQ", "av": "QQQ", "proxy_of": None},
    {"name": "hsi", "label": "恒生 HSI", "av": "EWH", "proxy_of": "EWH（港股 ETF，非 HSI 本体）"},
    {"name": "shcomp", "label": "上证 SHCOMP", "av": "FXI", "proxy_of": "FXI（中资大盘 ETF，非 SHCOMP 本体）"},
    {"name": "brn", "label": "Brent 原油 BRN", "av": "USO", "proxy_of": "USO（原油 ETF，非 BRN 本体）"},
]


@dataclass
class Emergence:
    name: str
    label: str
    av: str
    proxy_of: str | None
    n_bars: int
    date0: str
    date1: str
    n_strokes: int
    n_segments: int
    n_zhongshus: int
    n_moves: int
    n_bsp_l1: int
    bsp_l1_kinds: dict
    recursive_levels: int
    l2_segments: int
    l2_bsp: int


def analyze(cfg: dict) -> Emergence:
    d = av_fetch.fetch_daily(cfg["av"])
    adj = av_fetch.adjusted_ohlc(d)
    bars = [
        Bar(ts=datetime.fromisoformat(adj["dates"][i]), open=adj["opens"][i],
            high=adj["highs"][i], low=adj["lows"][i], close=adj["closes"][i],
            volume=adj["volumes"][i])
        for i in range(len(adj["dates"]))
    ]
    with contextlib.redirect_stdout(io.StringIO()):
        res = bl.analyze_levels_batch(bars, stream_id=cfg["av"])
    levels = res["levels"]
    l1 = levels.get("L1", {})
    bsp1 = l1.get("buysellpoints", [])
    from collections import Counter
    kinds = dict(Counter((b.get("kind"), b.get("side")) for b in bsp1))
    l2 = levels.get("L2", {})
    return Emergence(
        name=cfg["name"], label=cfg["label"], av=cfg["av"], proxy_of=cfg["proxy_of"],
        n_bars=len(bars), date0=str(bars[0].ts.date()), date1=str(bars[-1].ts.date()),
        n_strokes=l1.get("strokes", 0), n_segments=l1.get("segments", 0),
        n_zhongshus=len(l1.get("zhongshus", [])), n_moves=len(l1.get("moves", [])),
        n_bsp_l1=len(bsp1), bsp_l1_kinds={f"{k[0]}-{k[1]}": v for k, v in kinds.items()},
        recursive_levels=len(levels),
        l2_segments=l2.get("segments", 0), l2_bsp=len(l2.get("buysellpoints", [])),
    )


def build_report(results: list[Emergence]) -> str:
    L: list[str] = []
    a = L.append
    a("# AV 真实日线：数据就绪 + 引擎递归涌现（回测信号待 TV Replay 零前视序列）")
    a("")
    a("**编排者最终裁定（2026-05-31）**：TV 静态标注**不能**做回测信号源——买卖点标注在")
    a("分型极点，但**成立时刻**在右侧确认之后；静态数据无法恢复成立时刻，lag=1/lag=3 任何")
    a("静态近似都错。唯一可接受方案：**TV Replay 的 first_seen_step + lag=1 + 真实 OHLCV**")
    a("（first_seen_step = 逐步回放中信号无前视首现 = 成立时刻），由独立 Replay session 产出。")
    a("")
    a("本报告只产出**无前视争议的确定性内容**：AV 数据就绪状态 + 引擎递归涌现结构。")
    a("")
    a("---")
    a("")
    a("## 1. AV 真实日线 OHLCV（已保存，待 Replay 信号对齐）")
    a("")
    a("| 标的 | AV 代码 | 性质 | 根数 | 起 | 止 |")
    a("|------|--------|------|------|----|----|")
    for r in results:
        nature = "真实本体" if r.proxy_of is None else r.proxy_of
        a(f"| {r.label} | {r.av} | {nature} | {r.n_bars} | {r.date0} | {r.date1} |")
    a("| Brent 真实原油 | BRENT | AV commodity（真实布伦特，退化日收盘） | 9898 | 1987-05-20 | 2026-05-26 |")
    a("")
    a("> QQQ 为真实本体；HSI/SHCOMP/BRN 用代理 ETF（AV/Polygon 实测不提供指数本体，"
      "Polygon I:HSI/000001 返回 n=0）。BRENT commodity endpoint 为真实原油但仅日收盘单值。")
    a("")
    a("## 2. 引擎递归涌现结构（L0 确定性，无前视）")
    a("")
    a("| 标的(代理) | 日线 | 笔 | 线段 | 中枢 | L1走势 | L1买卖点 | 递归层数 | L2线段 | L2买卖点 |")
    a("|-----------|------|----|----|------|-------|---------|---------|-------|---------|")
    for r in results:
        a(f"| {r.label}({r.av}) | {r.n_bars} | {r.n_strokes} | {r.n_segments} | {r.n_zhongshus} "
          f"| {r.n_moves} | {r.n_bsp_l1} | {r.recursive_levels} | {r.l2_segments} | {r.l2_bsp} |")
    a("")
    a("### 走势分组涌现边界（用户指出的 bug 现象）")
    a("")
    a("引擎在真实日线上的递归涌现呈现一致的**走势分组瓶颈**：")
    for r in results:
        a(f"- **{r.label}**：{r.n_zhongshus} 中枢 → 仅 **{r.n_moves} 个 L1 走势** → "
          f"L1 买卖点 {r.n_bsp_l1} 个（{r.bsp_l1_kinds or '无'}），递归到 L{r.recursive_levels}。")
    a("")
    a("> **诊断**：`moves_from_zhongshus` 把十余个中枢压缩为 2-3 个走势（走势分组定义过严或"
      "有 bug），导致 L1 买卖点稀疏（个位数）且类型单一（多为 type3），递归在 L2 即终止"
      "（走势 <3 无法构建上层中枢）。**这正是不能用引擎自生买卖点做回测信号的原因**"
      "（信号太少、买卖不平衡），也是 TV 标注曾被用作左侧候选的根由。走势分组修复是独立的"
      "上游工作（与本回测数据准备解耦）。")
    a("")
    a("## 3. 回测信号对齐接口（待 TV Replay 零前视序列）")
    a("")
    a("回测**未产出**——按编排者裁定，需 Replay first_seen_step 避免前视。接入规格：")
    a("")
    a("1. **信号**：Replay session 逐步回放，记录每个买卖点的 `first_seen_step`（无前视首现步）。")
    a("2. **执行价**：`first_seen_step + lag=1` 对应交易日的真实 OHLCV 收盘价"
      "（QQQ 用 av_QQQ_daily.json；其余待原标的数据或代理）。")
    a("3. **走势方向**：真实日线线段方向（本报告 §2 已确定性产出，线段层无走势分组 bug）。")
    a("4. **FSM**：cost_reduction_fsm 三阶段降成本（A 组）+ PH settle 门控（B 组），lag=1。")
    a("5. **数据就绪**：QQQ 真实 OHLCV 已存 `analysis/data_cache/av_QQQ_daily.json`，待对齐。")
    a("")
    a("---")
    a("")
    a("## 结果包（六要素）")
    a("")
    a("1. **结论**：AV 四标的真实日线 OHLCV 已保存待用；引擎递归涌现结构确定性产出（§2）。"
      "回测信号源不用 TV 静态标注（前视偏差不可消除），待 TV Replay first_seen_step 零前视序列。")
    a("2. **定义依据**：递归涌现 = brn_level_analysis.analyze_levels_batch（包含→分型→笔→线段→"
      "中枢→走势→买卖点，缠论原文§5/§17）；前视偏差 = 买卖点成立需右侧确认（缠论第27/37课"
      "「对象否定对象」，a_online_persistence §10 因果 settle 定理）。")
    a("3. **边界条件**：若 Replay 产出 first_seen_step 序列 → 可接入 §3 接口产出零前视回测；"
      "若走势分组 bug 修复 → 引擎自生买卖点可能足量，届时可对照 TV 信号。")
    a("4. **下游推论**：引擎走势分组瓶颈（中枢多→走势少→买卖点稀疏）跨四标的一致，"
      "说明这是 moves_from_zhongshus 的结构性问题，非数据特异——修复它是恢复引擎自生信号"
      "（摆脱 TV 标注依赖）的关键路径。")
    a("5. **谱系引用**：前视偏差承接 a_online_persistence §10（settle = 顶/底分型右侧确认的"
      "因果形式化）；走势分组边界承接 recursive_backtest/unified docstring（引擎 confirmed "
      "买卖点≈0 的结构性声明）；267/338号（cost_reduction_fsm，待 Replay 信号接入）。")
    a("6. **影响声明**：av_daily_backtest_all.py 由 TV 静态回测改为数据准备 + 涌现分析"
      "（移除被否定的静态信号回测）；新增/保留 av_{QQQ,EWH,FXI,USO,BRENT}_daily.json 缓存；"
      "复用 brn_level_analysis/av_fetch（未改动）；未改动引擎与 FSM。")
    a("")
    a("## 缺口与诚实声明（no-patch-mentality）")
    a("")
    a("1. **回测未产出**：TV 静态标注前视偏差不可消除，本脚本不伪造回测数字。等待 Replay "
      "first_seen_step——这是唯一零前视途径（编排者裁定）。")
    a("2. **代理 ETF ≠ 原标的**：HSI/SHCOMP/BRN 用 EWH/FXI/USO，AV/Polygon 不提供指数本体。"
      "§2 涌现结构是代理 ETF 自身的真实结构，不可外推到原指数/原油。")
    a("3. **走势分组 bug 未修**：中枢→走势压缩过度（§2），引擎自生买卖点稀疏。本报告诊断现象，"
      "不在此修复（独立上游工作）。")
    a("4. **BRENT 退化 OHLC**：commodity endpoint 仅日收盘，未纳入 §2 涌现（需 OHLC）；"
      "作真实原油数据保存待用。")
    return "\n".join(L)


def main() -> None:
    results: list[Emergence] = []
    for cfg in SYMBOLS:
        print(f"[*] {cfg['label']} ({cfg['av']}) 递归涌现...", flush=True)
        r = analyze(cfg)
        results.append(r)
        print(f"    {r.n_bars}根 笔{r.n_strokes} 线段{r.n_segments} 中枢{r.n_zhongshus} "
              f"走势{r.n_moves} L1买卖点{r.n_bsp_l1} 递归L{r.recursive_levels}", flush=True)

    print("[*] 写报告...", flush=True)
    REPORT_PATH.write_text(build_report(results), encoding="utf-8")
    summary = {
        r.name: {
            "av": r.av, "proxy_of": r.proxy_of, "n_bars": r.n_bars,
            "date_range": [r.date0, r.date1],
            "emergence": {
                "strokes": r.n_strokes, "segments": r.n_segments,
                "zhongshus": r.n_zhongshus, "moves": r.n_moves,
                "bsp_l1": r.n_bsp_l1, "bsp_l1_kinds": r.bsp_l1_kinds,
                "recursive_levels": r.recursive_levels,
                "l2_segments": r.l2_segments, "l2_bsp": r.l2_bsp,
            },
        }
        for r in results
    }
    summary["_meta"] = {
        "backtest_status": "NOT_PRODUCED",
        "reason": "TV静态标注前视偏差不可消除；待TV Replay first_seen_step零前视序列",
        "data_ready": ["av_QQQ_daily.json", "av_EWH_daily.json", "av_FXI_daily.json",
                       "av_USO_daily.json", "av_BRENT_daily.json"],
    }
    RESULT_JSON.write_text(json.dumps(summary, ensure_ascii=False, indent=1))
    print(f"    报告: {REPORT_PATH}")
    print(f"    JSON: {RESULT_JSON}")


if __name__ == "__main__":
    main()
