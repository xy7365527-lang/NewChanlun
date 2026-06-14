"""RNF 1s a0 回测——递归深度+1层(lid4)能否缩小轧空税/加深净暴露呼吸。

═══════════════════════════════════════════════════════════════════════
设计(编排者 2026-06-13:a0=1s。更多递归深度→区间套更多精化→确认更快)
═══════════════════════════════════════════════════════════════════════
RNF(mode=rnf)根永不平多,净暴露=N−Σ子空units随子空存活/死亡呼吸。
1s假设:确认更快 ⇒ 子空回补(降成本兑现)更快 ⇒ 每次失败的轧空税m(c−P)更小
       (更早认错买回,c−P更小) + 更多递归层精化时点。
对照URS(E*清仓):1s能否让RNF在强牛(ES/BTC)≈BH(去清仓踏空)且降成本净正。

数据(只有这三个有1s,同urs_1s_a0_backtest.DATASETS):
  - ES1Y / CL1Y(databento) / BTC2W(binance)
口径(项目教训:1s交易增益=零摩擦伪影,盈亏平衡0.58bps<taker1.45):
  - 零摩擦strat(可比1min) + 含摩擦strat(maker单侧friction守卫零摩擦伪影)
  - floor=3(密度锚1s_l4≈1min_l3) 主臂

用法: PYTHONPATH=src .venv/bin/python analysis/rnf_1s_a0_backtest.py [KEY ...]
输出: analysis/data_cache/rnf_1s_<KEY>.json + rnf_1s_summary.json
"""
from __future__ import annotations
import json, sys, time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))
import newchan_rust as nr  # noqa: E402
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402
from nested_recursive_fugue_final_backtest import analyze, bh_mdd  # noqa: E402
from urs_1s_a0_backtest import DATASETS, load_1s, bucket_1min  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
KEYS = sys.argv[1:] or ["BTC2W", "ES1Y", "CL1Y"]


def _friction_adj(res, friction):
    cost = 0.0
    for t in res["trades"]:
        ep, xp, sh = t[2], t[4], t[5]
        cost += abs(sh) * (ep + xp) / 2 * friction * 2
    return (res["final_nav"] - cost) / 100000.0 * 100 - 100


def run_mode(rtape, floor, closes, n_bars, friction, mode):
    res = nr.run_positional_rust(rtape, floor_ladder=floor, mode=mode)
    a = analyze(res, closes, None)
    return {
        "strat_zero_fric": a["strat_pct"],
        "strat_with_fric": round(_friction_adj(res, friction), 1),
        "mdd": a["mdd_pct"], "n_trades": a["n_trades"],
        "sellpt": a["exit_reasons"].get("sellpt", 0),
        "spawns": sum(res["n_nrf_spawns_by_ladder"]),
        "deep_fires": sum(res["n_nrf_deep_fires_by_ladder"]),
        "short_net_cash": round(sum(res["short_net_cash_by_ladder"]), 0),
        "shrink_units": round(res.get("nrf_shrink_units", 0.0), 1),
        "phys_long_exposure": round(res["nrf_phys_long_bars"] / n_bars, 3),
    }


def run_key(key):
    ts, o, h, l, c, friction = load_1s(key)
    n1s = len(c)
    bh = (c[-1] / c[0] - 1) * 100
    bh_dd = bh_mdd(c) * 100
    print(f"[{key}] 1s bars={n1s:,} BH={bh:+.1f}% BH_MDD={bh_dd:.1f}% friction={friction}", flush=True)

    t0 = time.time()
    df1s: list = []
    tape_1s = compute_organic_signals(o, h, l, c, dir_flips=df1s, require_settled=True)
    rt_1s = pack_tape(tape_1s, dir_flips=df1s)
    print(f"[{key}] 1s信号层 {time.time()-t0:.1f}s flips={len(df1s)}", flush=True)

    bo, bh_, bl, bc = bucket_1min(ts, o, h, l, c)
    df1m: list = []
    tape_1m = compute_organic_signals(bo, bh_, bl, bc, dir_flips=df1m, require_settled=True)
    rt_1m = pack_tape(tape_1m, dir_flips=df1m)
    bh_1m = (bc[-1] / bc[0] - 1) * 100

    out = {"key": key, "n_1s": n1s, "n_1min": len(bc), "bh_pct": round(bh, 1),
           "bh_mdd_pct": round(bh_dd, 1), "friction": friction,
           "design": "RNF 1s a0:根永不平多净暴露呼吸;1s缩轧空税;vs URS(E*清仓)+1min桶对照"}
    arms = {}
    for mode in ("rnf", "urs"):
        t1 = time.time()
        arms[f"1s_f3_{mode}"] = run_mode(rt_1s, 3, c, n1s, friction, mode)
        m = arms[f"1s_f3_{mode}"]
        print(f"[{key}][1s f3 {mode}] {time.time()-t1:.1f}s strat0={m['strat_zero_fric']:+.1f}% "
              f"stratF={m['strat_with_fric']:+.1f}% mdd={m['mdd']}% sellpt={m['sellpt']} "
              f"short_net={m['short_net_cash']:+.0f} shrink={m['shrink_units']} "
              f"expL={m['phys_long_exposure']}", flush=True)
    # 1min 桶对照(RNF)
    t1 = time.time()
    arms["1min_f2_rnf"] = run_mode(rt_1m, 2, bc, len(bc), friction, "rnf")
    arms["1min_f2_rnf"]["bh_pct"] = round(bh_1m, 1)
    print(f"[{key}][1min f2 rnf] {time.time()-t1:.1f}s strat0={arms['1min_f2_rnf']['strat_zero_fric']:+.1f}% "
          f"sellpt={arms['1min_f2_rnf']['sellpt']}", flush=True)
    out["arms"] = arms
    r1s, u1s = arms["1s_f3_rnf"], arms["1s_f3_urs"]
    out["preregistered"] = {
        "RNF_1s_P1_ge_bh": [r1s["strat_zero_fric"], round(bh, 1), r1s["strat_zero_fric"] >= bh],
        "RNF_1s_vs_URS_1s_pp": round(r1s["strat_zero_fric"] - u1s["strat_zero_fric"], 1),
        "RNF_1s_vs_1min_pp": round(r1s["strat_zero_fric"] - arms["1min_f2_rnf"]["strat_zero_fric"], 1),
        "RNF_cost_net_sign": [r1s["short_net_cash"], r1s["short_net_cash"] >= 0],
        "fric_artifact_guard": {"zero": r1s["strat_zero_fric"], "fric": r1s["strat_with_fric"],
                                "survives": r1s["strat_with_fric"] >= bh},
    }
    return out


def main():
    summary = []
    for key in KEYS:
        try:
            out = run_key(key)
        except Exception as e:
            import traceback; traceback.print_exc()
            out = {"key": key, "failed": f"{type(e).__name__}: {e}"}
        (DATA_DIR / f"rnf_1s_{key}.json").write_text(json.dumps(out, ensure_ascii=False, indent=1))
        if "failed" not in out:
            pr = out["preregistered"]
            summary.append({"key": key, "bh": out["bh_pct"],
                            "rnf_1s": out["arms"]["1s_f3_rnf"]["strat_zero_fric"],
                            "urs_1s": out["arms"]["1s_f3_urs"]["strat_zero_fric"],
                            "rnf_1min": out["arms"]["1min_f2_rnf"]["strat_zero_fric"],
                            "RNF_1s_P1": pr["RNF_1s_P1_ge_bh"][2],
                            "rnf_short_net": out["arms"]["1s_f3_rnf"]["short_net_cash"],
                            "fric_survives": pr["fric_artifact_guard"]["survives"]})
        else:
            summary.append({"key": key, "failed": out["failed"]})
        (DATA_DIR / "rnf_1s_summary.json").write_text(json.dumps(summary, ensure_ascii=False, indent=1))
    print(json.dumps(summary, ensure_ascii=False, indent=1), flush=True)


if __name__ == "__main__":
    main()
