"""P6×P7 合取臂八标的预注册回测（535 号裁决实验）。

上游：535 号生成态矛盾（.chanlun/genealogy/pending/535）+ p6_phase_machine_results.md
（P6 单边否证 + 窗口×词汇耦合发现）+ unified_config_regime_research.md §2.3 R2 /
§6 P7（预注册母条款）。

臂位：fusion_tr = kind 时钟 × R2 削减位置门（P7 proper，research §6 逐字
"hold26 削减词汇加位置门（c≥ZG 才减）vs 无位置门"）；fusion_pr = 相位机 ×
位置门（P6×P7 合取基座——535 裁决臂）；fusion_pur = 合取 + 统一 osc 层
（research §5.1 完整层 0 的已实装子集）。
位置门语义（引擎 commit）：震荡相（phase 臂 Φ=OSC 精确；kind 臂 dir≠Down，
熊市削减零接触）卖点削减要求 c ≥ ZG(k)；无存活中枢不拦截（049:54 出场语义
回退）；回复侧位置门不在本轴（P7 预注册为削减侧最小差分）。

══════════════ 预注册判据（先于回测数据声明）══════════════

口径：零摩擦 close 主口径 + 摩擦面（0.05%/侧）并报。

C1（535 号裁决判据，主判据）：BTC nav 比值 nav(fusion_pr)/nav(fusion_t)
    ≥ 0.95（P6 同口径守卫，fusion_p 曾 0.300 击穿）。
    ≥ 0.95 ⇒ 535 矛盾消解为词汇残缺伪影（"中枢震荡的方法"= 位置词汇读法
    获 L3 支持），535 移 settled；
    < 0.95 ⇒ 矛盾为真定义冲突，535 保持生成态升编排者（届时概念分离候选 =
    「中枢震荡短差」有效域按 regime 二分）。
C2（双正域保持）：DX 与 BRN 上 fusion_pur ≥ fusion_t（P6 轮两个正域标的
    在合取下不丢失）。
C3（GC 均值回归解释检验）：预测 GC 仍 fusion_pr < hold26（p6 结果 §3.2：
    GC 拒一切停削窗口）。若 GC 转正（fusion_pr ≥ hold26）⇒ §3.2 解释被
    否证（其负贡献其实来自词汇而非窗口本身）——双向均有信息。
C4（P7 proper 差分报告）：fusion_tr − fusion_t 逐标的 8/8 全表（research
    §6 P7 未预注册通过线，纯配对报告供词汇轴独立读数）。
P3 同型：n_r2_pos_blocks 逐标的报告；八标的合计 == 0 ⇒ 门空转（H2 先例
    单独否证）；全拦（削减全无）⇒ 退化检查。

守卫：hold26/fusion_t 逐位复现在册（0.05pp）+ tape_fp。

════════════════════════════════════════════════════════════

用法：PYTHONPATH=src .venv/bin/python analysis/p7_conjunction_backtest.py [SYM ...]
输出：analysis/data_cache/p7_conj_<SYM>.json + p7_conj_summary.json
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
from p6_phase_machine_backtest import analyze, best_in_book, bh_mdd  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
FLOOR = LADDER_SEG
MODES = ["hold26", "fusion_t", "fusion_tr", "fusion_p", "fusion_pr", "fusion_pur"]
SYMBOLS = sys.argv[1:] or ["BTC", "OKLO", "BRN", "CL", "ES", "GC", "QQQ", "DX"]
FRICTION_SIDE = 0.0005
GUARD_TOL = 0.05


def prereg(sym: str, modes: dict) -> dict:
    h = modes["hold26"]["strat_pct"]
    ft = modes["fusion_t"]["strat_pct"]
    tr = modes["fusion_tr"]["strat_pct"]
    fp = modes["fusion_p"]["strat_pct"]
    pr = modes["fusion_pr"]["strat_pct"]
    pur = modes["fusion_pur"]["strat_pct"]
    out: dict = {}
    out["C1_nav_ratio_pr_ft"] = round((1 + pr / 100) / (1 + ft / 100), 4)
    out["C2_pur_ge_ft"] = [pur, ft, pur >= ft]
    out["C3_pr_vs_hold26"] = [pr, h, pr >= h]
    out["C4_delta_tr_ft"] = round(tr - ft, 1)
    out["delta_pr_p"] = round(pr - fp, 1)  # 词汇增量（窗口固定为相位机）
    out["delta_osc_conj"] = round(pur - pr, 1)  # osc 层增量（合取基座上）
    out["r2_blocks"] = {
        m: sum(modes[m]["r2_obs"]) for m in ("fusion_tr", "fusion_pr", "fusion_pur")
    }
    return out


def settle_summary(rows: list) -> dict:
    ok = [r for r in rows if "failed" not in r]
    by = {r["symbol"]: r["preregistered"] for r in ok}
    verdict: dict = {}
    if "BTC" in by:
        c1 = by["BTC"]["C1_nav_ratio_pr_ft"]
        verdict["C1_535"] = {
            "btc_nav_ratio": c1, "pass": c1 >= 0.95,
            "resolution": ("矛盾消解为词汇残缺伪影——535 移 settled"
                           if c1 >= 0.95 else
                           "真定义冲突——535 保持生成态升编排者")}
    c2 = {s: by[s]["C2_pur_ge_ft"][2] for s in ("DX", "BRN") if s in by}
    verdict["C2"] = {"per_symbol": c2,
                     "pass": all(c2.values()) and len(c2) == 2}
    if "GC" in by:
        gc = by["GC"]["C3_pr_vs_hold26"]
        verdict["C3_GC"] = {"pr": gc[0], "hold26": gc[1],
                            "mean_reversion_explanation_holds": not gc[2]}
    verdict["C4_table"] = {r["symbol"]: r["preregistered"]["C4_delta_tr_ft"]
                           for r in ok}
    verdict["r2_gate_active"] = {
        r["symbol"]: r["preregistered"]["r2_blocks"]["fusion_pr"] for r in ok}
    return verdict


def run_symbol(sym: str) -> dict:
    book = best_in_book(sym)
    path = SYMBOL_FILES[sym]
    opens, highs, lows, closes, years = load_ohlc(path)
    n = len(closes)
    bh_pct = (closes[-1] / closes[0] - 1) * 100
    print(f"[{sym}] bars={n:,} BH={bh_pct:+.1f}%", flush=True)

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
    print(f"[{sym}] 信号层 {time.time() - t0:.1f}s", flush=True)

    ref = json.loads((DATA_DIR / f"osc_scco_{sym}.json").read_text())
    if ref["tape_fp"] != fp:
        return {"symbol": sym, "failed": "tape_fp_drift",
                "fp": fp, "ref_fp": ref["tape_fp"]}
    ft_book = json.loads(
        (DATA_DIR / f"fusion_t_eight_{sym}.json").read_text())["modes"]
    rtape = pack_tape(tape, dir_flips=dir_flips, trend_flips=trend_flips)

    out = {"symbol": sym, "n_bars": n, "tape_fp": fp,
           "bh_pct": round(bh_pct, 1), "bh_mdd_pct": round(bh_mdd(closes) * 100, 1),
           "book": book, "friction_side": FRICTION_SIDE,
           "design": "P6×P7 合取臂（535 号裁决实验，判据见探针 docstring）",
           "modes": {}}
    for mode in MODES:
        t1 = time.time()
        res = nr.run_positional_rust(rtape, floor_ladder=FLOOR, mode=mode)
        if "n_r2_pos_blocks_by_ladder" not in res:
            raise KeyError("n_r2_pos_blocks_by_ladder marshal 缺键（引擎能力不符）")
        a = analyze(res, closes, years)
        a["r2_obs"] = res["n_r2_pos_blocks_by_ladder"]
        out["modes"][mode] = a
        print(f"[{sym}][{mode}] {time.time() - t1:.1f}s "
              f"strat={a['strat_pct']:+.1f}% fa={a['fa_strat_pct']:+.1f}% "
              f"r2_blocks={sum(a['r2_obs'])} reasons={a['exit_reasons']}",
              flush=True)
    for mode, key in (("hold26", "hold26"), ("fusion_t", "fusion_t")):
        got = out["modes"][mode]["strat_pct"]
        want = ft_book[key]["strat_pct"]
        if abs(got - want) >= GUARD_TOL:
            return {"symbol": sym,
                    "failed": f"guard_{mode}_drift: {got} ≠ {want}"}
    out["preregistered"] = prereg(sym, out["modes"])
    print(f"[{sym}] prereg={json.dumps(out['preregistered'])}", flush=True)
    return out


def main() -> None:
    summary = []
    payload: dict = {}
    for sym in SYMBOLS:
        try:
            out = run_symbol(sym)
        except Exception as e:
            out = {"symbol": sym, "failed": f"{type(e).__name__}: {e}"}
            print(f"[{sym}] FAILED {out['failed']}", flush=True)
        (DATA_DIR / f"p7_conj_{sym}.json").write_text(
            json.dumps(out, ensure_ascii=False, indent=1))
        if "failed" in out:
            summary.append({"symbol": sym, "failed": out["failed"]})
        else:
            row = {"symbol": sym, "bh_pct": out["bh_pct"],
                   "best_in_book": out["book"]["best"]}
            for mode in MODES:
                m = out["modes"][mode]
                row[mode] = m["strat_pct"]
                row[f"{mode}_fa"] = m["fa_strat_pct"]
            row["preregistered"] = out["preregistered"]
            summary.append(row)
        payload = {"rows": summary}
        if any("failed" not in r for r in summary):
            payload["verdict"] = settle_summary(summary)
        (DATA_DIR / "p7_conj_summary.json").write_text(
            json.dumps(payload, ensure_ascii=False, indent=1))
    print(json.dumps(payload.get("verdict", {}), ensure_ascii=False, indent=1),
          flush=True)


if __name__ == "__main__":
    main()
