"""全量普适组合 v2 消融矩阵——fusion_btra{mods}_s34 八标的（编排者补充
H1 候选冻结 + Sequence38 后的重审轮）。

上游：
- `analysis/universal_combination_results.md`（v1 判决：P1 6/8 / nest 保号性）
- 编排者 v2 指令（2026-06-12）：加入 H1（candidate 冻结）与 Seq38（38课
  严格形式）两重审模块；每模块独立消融臂（叠加/不叠加的差）避免正交互
  伪影；非全局正者标注但保留在消融矩阵中（门组合非单调改进在册教训：
  域门+成本门双门负交互）。

══════════════ 模块挂载形态（结构判决，先于运行声明）══════════════

v1 的 H4 域空判决在 v2 被消融矩阵形态消解：H4 不可单独挂载（其对象 =
osc 腿开腿准入），但 **u 臂 = 统一 osc 层**（OscRouting::Unified，三门
含 H4 逐字门②）给 H4 提供承载位。osc × short 的翻转断面筹码冲突由
**slice custody 分区**消解（在册先例：U×counter_sub"同一笔筹码不能同时
按两种节奏在外"互斥语义的层粒度形式）：翻空白名单层 {3,4} 的 slice 由
双向词汇独占，u/q 子机制仅非白名单层激活——零已结算定义变更。

| 模块 | 字母 | 挂载 | 在册出处 |
|------|------|------|---------|
| nest_forward | n | 削减/回复/翻空/平空触发时机（v1 已实装） | 027课程序定理 |
| 统一 osc 层（H4 承载） | u | 非白名单层 osc 腿，三门含 H4 ② | 035:30 H4 逐字 |
| H1 candidate 冻结 | f | u 层路由门（H4 之后，LOU 链序逐字），要求 u | 49:68 当下判据 |
| Sequence38 子腿 | q | 非白名单层 SubOut 载具 + 38课程式开闭 | 38:36 + 答疑:296 |

消融矩阵（全部同一配置应用八标的，零标的级参数）：
  base = fusion_btra_s34；+n = btran；+u = btrau；+u+f = btrauf；
  +q = btraq；full = btranufq。
模块边际：Δn = btran−base；Δu = btrau−base；Δf = btrauf−btrau；
  Δq = btraq−base；交互残差 = full − base − (Δn+Δu+Δf+Δq)。

══════════════ 预注册判据（先于数据声明）══════════════

V1 模块边际符号表：四模块 Δ 逐标的符号 + 与在册单轴白名单对照
   （nest 白名单正域 {OKLO,ES,BRN,DX}——v1 已验 8/8 保号；H1 在册
   有效域 = OKLO 1/5（僵尸开窗口拦截）；Seq38 在册 OKLO+10.2/BRN+4.1
   全正；H4 osc 摩擦面 5/5 全正——零摩擦面下无预期符号）。
V2 H1 标的无关性（编排者重审假设）：Δf 符号是否跨八标的一致——
   全 ≥0 ⇒ "candidate 未决=方向未定"机制普适成立，OKLO 专属判决推翻；
   分裂 ⇒ regime 函数家族新例，机制结构性但效应标的依赖。
V3 非单调标注义务：任何模块对（如 n×u）的合取增量 ≠ 边际和 ⇒ 交互
   残差直读并标注（不因非全局正而剔出矩阵）。
V4 full 臂 vs v1 最优：full ≥ max(base, btran) 逐标的标注（普适配置
   的收益面上限是否由更多模块单调推进——在册教训预期：否）。
V5 门拦截可观测：h1 freezes / seq38 opens+closes 三岔 / osc 路由拒因
   逐标的非零判别（判别力零的门按 H2 先例标注退化）。
V6 零标的级参数：同一 mode 串八标的（构造性）。

guard-a：tape_fp 与 p7_conj 在册逐项一致。
guard-b：hold26 / fusion_tr 逐位复现 p7_conj 在册（0.05pp）。
guard-c：BTC fusion_btra_s34 = +7368.0（0.05pp）。
guard-d：fusion_btran_s34 逐位复现 v1（universal_combo_<SYM>.json，
   0.05pp）——v2 的 custody/nf 改动对 v1 臂必须零接触。

════════════════════════════════════════════════════════════

用法：PYTHONPATH=src .venv/bin/python analysis/universal_combination_v2_backtest.py [SYM ...]
输出：analysis/data_cache/universal_v2_<SYM>.json + universal_v2_summary.json
"""

from __future__ import annotations

import json
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as nr  # noqa: E402

from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from fugue_version_i import LADDER_SEG  # noqa: E402
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402
from universal_combination_backtest import analyze, bh_mdd  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
FLOOR = LADDER_SEG

BASE = "fusion_btra_s34"
MODES = ["hold26", "fusion_tr", BASE, "fusion_btran_s34", "fusion_btrau_s34",
         "fusion_btrauf_s34", "fusion_btraq_s34", "fusion_btranufq_s34"]
SYMBOLS = sys.argv[1:] or ["BTC", "OKLO", "BRN", "CL", "ES", "GC", "QQQ", "DX"]
BTRA_S34_IN_BOOK_BTC = 7368.0


def gates_v2(res: dict) -> dict:
    """v2 新增门观测面（v1 gates 由 analyze 内置）。"""
    return {
        "h1_freezes": res["n_route_h1_freezes"],
        "seq38_opens": res["n_seq38_opens_by_ladder"],
        "seq38_consbuy": res["n_seq38_consbuy_closes_by_ladder"],
        "seq38_nobreak": res["n_seq38_nobreak_closes_by_ladder"],
        "seq38_newdiv": res["n_seq38_newdiv_closes_by_ladder"],
        "osc_opens": res["n_osc_opens_by_ladder"],
        "osc_escalates": res["n_osc_escalates_by_ladder"],
        "route_amp_rejects": res["n_route_amp_rejects"],
        "route_phase_skips": [int(x) for x in res["n_route_phase_skips"]],
        "route_weak_rejects": res["n_route_weak_rejects"],
        "sub_escalates": res["n_sub_escalates_by_ladder"],
        "sub_net_cash": [round(x, 0) for x in res["sub_net_cash_by_ladder"]],
        "osc_net_cash": [round(x, 0) for x in res["osc_net_cash_by_ladder"]],
    }


def ablation(modes: dict) -> dict:
    """模块边际 + 交互残差（V1/V3/V4）。"""
    s = {m: modes[m]["strat_pct"] for m in modes}
    base = s[BASE]
    d_n = round(s["fusion_btran_s34"] - base, 1)
    d_u = round(s["fusion_btrau_s34"] - base, 1)
    d_f = round(s["fusion_btrauf_s34"] - s["fusion_btrau_s34"], 1)
    d_q = round(s["fusion_btraq_s34"] - base, 1)
    full = s["fusion_btranufq_s34"]
    resid = round(full - base - (d_n + d_u + d_f + d_q), 1)
    return {"base": base, "d_n": d_n, "d_u": d_u, "d_f": d_f, "d_q": d_q,
            "full": full, "interaction_residual": resid,
            "V4_full_ge_v1max": full >= max(base, s["fusion_btran_s34"])}


def run_symbol(sym: str) -> dict:
    path = SYMBOL_FILES[sym]
    opens, highs, lows, closes, years = load_ohlc(path)
    n = len(closes)
    bh_pct = (closes[-1] / closes[0] - 1) * 100
    bh_dd = bh_mdd(closes) * 100
    print(f"[{sym}] bars={n:,} BH={bh_pct:+.1f}% BH_MDD={bh_dd:.1f}%",
          flush=True)

    t0 = time.time()
    dir_flips: list = []
    trend_flips: list = []
    tape = compute_organic_signals(opens, highs, lows, closes,
                                   dir_flips=dir_flips,
                                   trend_flips=trend_flips)
    fp = {"bsp": sum(len(s.bsp_events[l]) for s in tape if s.bsp_events
                     for l in range(11)),
          "div": sum(len(s.div_events[l]) for s in tape if s.div_events
                     for l in range(11)),
          "flips": len(dir_flips), "tflips": len(trend_flips)}
    print(f"[{sym}] 信号层 {time.time() - t0:.1f}s fp={fp}", flush=True)

    ref = json.loads((DATA_DIR / f"p7_conj_{sym}.json").read_text())
    if ref["tape_fp"] != fp:
        return {"symbol": sym, "failed": "tape_fp_drift",
                "fp": fp, "ref_fp": ref["tape_fp"]}
    rtape = pack_tape(tape, dir_flips=dir_flips, trend_flips=trend_flips)

    out = {"symbol": sym, "n_bars": n, "tape_fp": fp,
           "bh_pct": round(bh_pct, 1), "bh_mdd_pct": round(bh_dd, 1),
           "design": "全量普适组合 v2 消融矩阵（判据见 docstring）",
           "modes": {}}
    for mode in MODES:
        t1 = time.time()
        res = nr.run_positional_rust(rtape, floor_ladder=FLOOR, mode=mode)
        a = analyze(res, closes, years)
        a["gates_v2"] = gates_v2(res)
        out["modes"][mode] = a
        g = a["gates"]
        g2 = a["gates_v2"]
        print(f"[{sym}][{mode}] {time.time() - t1:.1f}s "
              f"strat={a['strat_pct']:+.1f}% mdd={a['mdd_pct']}% "
              f"flips={sum(g['flips'])} "
              f"nest={sum(g['nest_fire_sell']) + sum(g['nest_fire_buy'])} "
              f"osc={sum(g2['osc_opens'])} h1={sum(g2['h1_freezes'])} "
              f"s38={sum(g2['seq38_opens'])} liq={sum(g['liquidations'])}",
              flush=True)

    guards = {}
    for m in ("hold26", "fusion_tr"):
        book = ref["modes"][m]["strat_pct"]
        now = out["modes"][m]["strat_pct"]
        guards[f"guard_b_{m}"] = [now, book, abs(now - book) < 0.05]
    if sym == "BTC":
        now = out["modes"][BASE]["strat_pct"]
        guards["guard_c_btra_s34"] = [now, BTRA_S34_IN_BOOK_BTC,
                                      abs(now - BTRA_S34_IN_BOOK_BTC) < 0.05]
    v1_file = DATA_DIR / f"universal_combo_{sym}.json"
    if v1_file.exists():
        v1 = json.loads(v1_file.read_text())
        if "modes" in v1:
            book = v1["modes"]["fusion_btran_s34"]["strat_pct"]
            now = out["modes"]["fusion_btran_s34"]["strat_pct"]
            guards["guard_d_btran_v1"] = [now, book, abs(now - book) < 0.05]
    out["guards"] = guards
    if not all(v[2] for v in guards.values()):
        out["failed"] = "guard_reproduction"
        return out

    out["ablation"] = ablation(out["modes"])
    print(f"[{sym}] ablation={json.dumps(out['ablation'])}", flush=True)
    return out


def main() -> None:
    summary = []
    for sym in SYMBOLS:
        try:
            out = run_symbol(sym)
        except Exception as e:  # 标的隔离，不吞错误细节
            out = {"symbol": sym, "failed": f"{type(e).__name__}: {e}"}
            print(f"[{sym}] FAILED {out['failed']}", flush=True)
        (DATA_DIR / f"universal_v2_{sym}.json").write_text(
            json.dumps(out, ensure_ascii=False, indent=1))
        if "failed" in out:
            summary.append({"symbol": sym, "failed": out["failed"]})
        else:
            row = {"symbol": sym, "bh_pct": out["bh_pct"],
                   "ablation": out["ablation"]}
            for mode in MODES:
                row[mode] = out["modes"][mode]["strat_pct"]
                row[f"{mode}_mdd"] = out["modes"][mode]["mdd_pct"]
            summary.append(row)
        (DATA_DIR / "universal_v2_summary.json").write_text(
            json.dumps(summary, ensure_ascii=False, indent=1))
    ok = [r for r in summary if "failed" not in r]
    for key, name in (("d_n", "nest"), ("d_u", "osc/H4"), ("d_f", "H1"),
                      ("d_q", "Seq38")):
        signs = {r["symbol"]: r["ablation"][key] for r in ok}
        pos = sum(v > 0 for v in signs.values())
        print(f"Δ{name}: 正 {pos}/{len(ok)} {signs}", flush=True)
    print(json.dumps(summary, ensure_ascii=False, indent=1), flush=True)


if __name__ == "__main__":
    main()
