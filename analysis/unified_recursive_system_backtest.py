"""统一递归系统(URS)八标的回测——从概念链23环直接翻译,零flag。

═══════════════════════════════════════════════════════════════════════
设计(编排者 2026-06-13:不再patch,从构成性必然一次性实装)
═══════════════════════════════════════════════════════════════════════
与v4(nested_fugue)唯一差异 = C清仓层定义:
  - v4:振幅棘轮top(否证3/8) / CT:A′事件选层(L3否证踏空)
  - URS:清仓层 = 根voice涌现归属级别E*(走势结构驱动)
E* = 从根入场级别向上爬,父级别lad+1须同时①dir==Up ②anchor>=root.entry_bar
(持仓期内走势自身生长)。清仓 = sell1[E*] ∧ located[E*]。
runner.rs ExitMode::Emergent已L3验证(emht五标的全正BTC+170.8pp)。
零flag:无ClearanceMode/regime门/白名单。唯一参数a0。settle门on(构成性确认贯通)。

═══════════════════════════════════════════════════════════════════════
预注册判据(先于运行;L3)
═══════════════════════════════════════════════════════════════════════
P1: 八标的strat≥BH(任务目标8/8)。
R_freq: 清仓频率随波动率自然分化(CL sellpt > OKLO sellpt),非门控。
R_noteff: OKLO不过度清仓踏空(sellpt远小于v5的27,不恶化v4的+695%)。
R_mdd: MDD 8/8优于BH。
否证价值:若<8/8,坐实"清仓频率regime不可约"第五形态(E*走势结构驱动仍不普适)。

用法: PYTHONPATH=src .venv/bin/python analysis/unified_recursive_system_backtest.py [SYM ...]
输出: analysis/data_cache/urs_<SYM>.json + urs_summary.json
"""
from __future__ import annotations
import json, sys, time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))
import newchan_rust as nr  # noqa: E402
from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from nested_recursive_fugue_final_backtest import FLOOR, analyze, bh_mdd  # noqa: E402
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
SYMBOLS = sys.argv[1:] or ["OKLO", "QQQ", "BRN", "DX", "ES", "GC", "CL", "BTC"]


def _load_v4(sym: str) -> dict | None:
    p = DATA_DIR / f"nrf_v4_{sym}.json"
    if not p.exists():
        return None
    d = json.loads(p.read_text())
    if "failed" in d:
        return None
    blk = d.get("nrf_v4", {})
    pr = d.get("preregistered", {})
    sellpt = (pr.get("N1_liquidations_vs_spawns", {}).get("sellpt_closes")
              or pr.get("sellpt_on") or blk.get("exit_reasons", {}).get("sellpt", 0))
    return {"strat_pct": blk.get("strat_pct"), "mdd_pct": blk.get("mdd_pct"), "sellpt": sellpt}


def run_symbol(sym: str) -> dict:
    path = SYMBOL_FILES[sym]
    opens, highs, lows, closes, years = load_ohlc(path)
    n = len(closes)
    bh_pct = (closes[-1] / closes[0] - 1) * 100
    bh_dd = bh_mdd(closes) * 100
    print(f"[{sym}] bars={n:,} BH={bh_pct:+.1f}% BH_MDD={bh_dd:.1f}%", flush=True)

    t0 = time.time()
    dir_flips: list = []
    # settle门on(构成性确认贯通);URS只需dir_flips(E*读dir/anchor),不需trend_flips。
    tape = compute_organic_signals(opens, highs, lows, closes,
                                   dir_flips=dir_flips, require_settled=True)
    print(f"[{sym}] 信号层(settle on) {time.time()-t0:.1f}s flips={len(dir_flips)}", flush=True)
    rtape = pack_tape(tape, dir_flips=dir_flips)

    v4 = _load_v4(sym)
    t1 = time.time()
    res = nr.run_positional_rust(rtape, floor_ladder=FLOOR, mode="urs")
    a = analyze(res, closes, years)
    a["nrf_counters"]["deep_fires"] = res["n_nrf_deep_fires_by_ladder"]
    a["nrf_counters"]["earning_units"] = round(res["nrf_earning_units"], 3)
    sellpt = a["exit_reasons"].get("sellpt", 0)
    c = a["nrf_counters"]
    pr = {
        "P1_ge_bh": [a["strat_pct"], round(bh_pct, 1), a["strat_pct"] >= bh_pct],
        "sellpt": sellpt, "spawns": sum(c["spawns"]), "deep_fires": sum(c["deep_fires"]),
        "liquidations": sum(c["liquidations"]),
        "phys_long_frac": round(c["phys_long_bars"] / n, 3),
        "root_recL3": c["root_entries"][5] if len(c["root_entries"]) > 5 else 0,
        "R_mdd_ge_bh": [a["mdd_pct"], round(bh_dd, 1), a["mdd_pct"] >= bh_dd],
    }
    if v4 is not None:
        pr["vs_v4"] = {"strat_v4": v4["strat_pct"], "strat_urs": a["strat_pct"],
                       "delta_pp": round(a["strat_pct"] - (v4["strat_pct"] or 0), 1),
                       "sellpt_v4": v4["sellpt"], "sellpt_urs": sellpt}
    print(f"[{sym}][urs] {time.time()-t1:.1f}s strat={a['strat_pct']:+.1f}% "
          f"mdd={a['mdd_pct']}% trades={a['n_trades']} sellpt={sellpt} "
          f"deep={pr['deep_fires']} P1={pr['P1_ge_bh'][2]}", flush=True)
    return {"symbol": sym, "n_bars": n, "bh_pct": round(bh_pct, 1),
            "bh_mdd_pct": round(bh_dd, 1), "urs": a, "preregistered": pr,
            "design": "URS:E*涌现归属清仓层(走势结构驱动),零flag,settle on"}


def main() -> None:
    summary = []
    for sym in SYMBOLS:
        try:
            out = run_symbol(sym)
        except Exception as e:
            import traceback; traceback.print_exc()
            out = {"symbol": sym, "failed": f"{type(e).__name__}: {e}"}
            print(f"[{sym}] FAILED {out['failed']}", flush=True)
        (DATA_DIR / f"urs_{sym}.json").write_text(json.dumps(out, ensure_ascii=False, indent=1))
        if "failed" in out:
            summary.append({"symbol": sym, "failed": out["failed"]})
        else:
            summary.append({"symbol": sym, "bh_pct": out["bh_pct"],
                            "urs_pct": out["urs"]["strat_pct"], "urs_mdd": out["urs"]["mdd_pct"],
                            "P1": out["preregistered"]["P1_ge_bh"][2],
                            "sellpt": out["preregistered"]["sellpt"]})
        (DATA_DIR / "urs_summary.json").write_text(json.dumps(summary, ensure_ascii=False, indent=1))
    print(json.dumps(summary, ensure_ascii=False, indent=1), flush=True)


if __name__ == "__main__":
    main()
