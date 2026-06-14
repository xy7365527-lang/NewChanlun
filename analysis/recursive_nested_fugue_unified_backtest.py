"""递归嵌套多重赋格(RNF)八标的回测——"平多≠开空"推到极限 vs URS 对照。

═══════════════════════════════════════════════════════════════════════
设计(编排者 2026-06-13:从递归嵌套多重赋格这一个概念重写交易层——吃到每一笔)
═══════════════════════════════════════════════════════════════════════
RNF(mode=rnf) 与 URS(mode=urs)/v4(mode=nrf) 唯一构成性差异 = C 清仓块:
  - v4:平多 = confirmed Sell1@top ∧ located@top(θ振幅棘轮层)         → 3/8
  - URS:平多 = sell1[E*] ∧ located[E*](E*=根涌现归属级别)             → 5/8
  - RNF:**根永不平多**(除EOD)。根层及以下一切卖点=开空(降成本spawn子空)。
        全部regime适应来自子空存活/死亡的净暴露呼吸,无离散清仓决策。
会计原语(§1-§8)三系统bit-exact;五条全局不变量每bar强制检查。零flag,唯一参数a0。

═══════════════════════════════════════════════════════════════════════
预注册判决(先于运行;L3 八标的真实数据交叉验证)
═══════════════════════════════════════════════════════════════════════
三结局(互斥,由八标的×{降成本净额short_net_cash, 暴露, 清仓}裁决):
  (a) 突破:RNF 8/8≥BH ⇒ 概念正确,清仓本身=牛市杀手(v4/URS的E*/棘轮误判平多时机)。
  (b) regime二难墙:RNF牛市域{BTC,ES,BRN}≈BH(去清仓踏空) ∧ 崩盘域{CL,GC,DX}失去
      保护(<URS,根满仓骑崩盘) ⇒ 与URS互补失败 ⇒ 证明清仓是regime函数不可约
      (牛要不清/崩要清,同一type1+located决策时不可区分,539号开放轴升格L3)。
  (c) 降成本普适代价:RNF处处差(Σshort_net_cash<0全标的) ⇒ 轧空税>降成本利润,
      降成本机制本身在背驰负预测域是负期望(非清仓问题)。

归因仪表(每标的):
  - Σshort_net_cash = 降成本净额(成功买回低 − 轧空税买回高)。符号=结局(c)判别。
  - nrf_shrink_units = 累计轧空税缩水(强牛中死掉的子空)。
  - phys_long_exposure = 物理多头bar占比(≈BH=1.0 ⇒ 无清仓踏空)。
  - sellpt = RNF应恒0(根永不平多);URS>0(对照)。

用法: PYTHONPATH=src .venv/bin/python analysis/recursive_nested_fugue_unified_backtest.py [SYM ...]
输出: analysis/data_cache/rnf_<SYM>.json + rnf_summary.json
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


def run_mode(rtape, floor, closes, n_bars, mode):
    res = nr.run_positional_rust(rtape, floor_ladder=floor, mode=mode)
    a = analyze(res, closes, None)
    short_net = sum(res["short_net_cash_by_ladder"])
    return {
        "strat_pct": a["strat_pct"],
        "mdd_pct": a["mdd_pct"],
        "n_trades": a["n_trades"],
        "sellpt": a["exit_reasons"].get("sellpt", 0),
        "eod": a["exit_reasons"].get("eod", 0),
        "recover": a["exit_reasons"].get("recover", 0),
        "negate": a["exit_reasons"].get("negate", 0),
        "spawns": sum(res["n_nrf_spawns_by_ladder"]),
        "deep_fires": sum(res["n_nrf_deep_fires_by_ladder"]),
        "liquidations": sum(res["n_short_liquidations_by_ladder"]),
        # 归因仪表:
        "short_net_cash": round(short_net, 0),               # 降成本净额(结局c判别)
        "shrink_units": round(res.get("nrf_shrink_units", 0.0), 1),  # 累计轧空税
        "earning_units": round(res.get("nrf_earning_units", 0.0), 1),
        "phys_long_exposure": round(res["nrf_phys_long_bars"] / n_bars, 3),
        "phys_short_exposure": round(res["nrf_phys_short_bars"] / n_bars, 3),
    }


def run_symbol(sym: str) -> dict:
    path = SYMBOL_FILES[sym]
    opens, highs, lows, closes, years = load_ohlc(path)
    n = len(closes)
    bh_pct = (closes[-1] / closes[0] - 1) * 100
    bh_dd = bh_mdd(closes) * 100
    print(f"[{sym}] bars={n:,} BH={bh_pct:+.1f}% BH_MDD={bh_dd:.1f}%", flush=True)

    t0 = time.time()
    dir_flips: list = []
    tape = compute_organic_signals(opens, highs, lows, closes,
                                   dir_flips=dir_flips, require_settled=True)
    rt = pack_tape(tape, dir_flips=dir_flips)
    print(f"[{sym}] 信号层(settle on) {time.time()-t0:.1f}s flips={len(dir_flips)}", flush=True)

    out = {"sym": sym, "n_bars": n, "bh_pct": round(bh_pct, 1),
           "bh_mdd_pct": round(bh_dd, 1),
           "design": "RNF 根永不平多(净暴露呼吸) vs URS(E*清仓) 同窗对照;零flag"}
    modes = {}
    for mode in ("rnf", "urs"):
        t1 = time.time()
        modes[mode] = run_mode(rt, FLOOR, closes, n, mode)
        m = modes[mode]
        print(f"[{sym}][{mode}] {time.time()-t1:.1f}s strat={m['strat_pct']:+.1f}% "
              f"(BH{bh_pct:+.1f}) mdd={m['mdd_pct']}% sellpt={m['sellpt']} "
              f"spawns={m['spawns']} short_net={m['short_net_cash']:+.0f} "
              f"shrink={m['shrink_units']} expL={m['phys_long_exposure']}", flush=True)
    out["modes"] = modes
    rnf, urs = modes["rnf"], modes["urs"]
    out["preregistered"] = {
        "RNF_P1_ge_bh": [rnf["strat_pct"], round(bh_pct, 1), rnf["strat_pct"] >= bh_pct],
        "URS_P1_ge_bh": [urs["strat_pct"], round(bh_pct, 1), urs["strat_pct"] >= bh_pct],
        "RNF_root_never_clears": [rnf["sellpt"], rnf["sellpt"] == 0],
        "RNF_cost_reduction_net": [rnf["short_net_cash"], rnf["short_net_cash"] >= 0],
        "RNF_full_exposure": [rnf["phys_long_exposure"], rnf["phys_long_exposure"] >= 0.95],
        "RNF_vs_URS_pp": round(rnf["strat_pct"] - urs["strat_pct"], 1),
    }
    return out


def main():
    summary = []
    for sym in SYMBOLS:
        try:
            out = run_symbol(sym)
        except Exception as e:
            import traceback; traceback.print_exc()
            out = {"sym": sym, "failed": f"{type(e).__name__}: {e}"}
            print(f"[{sym}] FAILED {out['failed']}", flush=True)
        (DATA_DIR / f"rnf_{sym}.json").write_text(json.dumps(out, ensure_ascii=False, indent=1))
        if "failed" not in out:
            pr = out["preregistered"]
            summary.append({
                "sym": sym, "bh": out["bh_pct"],
                "rnf": out["modes"]["rnf"]["strat_pct"],
                "urs": out["modes"]["urs"]["strat_pct"],
                "RNF_P1": pr["RNF_P1_ge_bh"][2], "URS_P1": pr["URS_P1_ge_bh"][2],
                "rnf_sellpt": out["modes"]["rnf"]["sellpt"],
                "rnf_short_net": out["modes"]["rnf"]["short_net_cash"],
                "rnf_expL": out["modes"]["rnf"]["phys_long_exposure"],
                "rnf_mdd": out["modes"]["rnf"]["mdd_pct"],
            })
        else:
            summary.append({"sym": sym, "failed": out["failed"]})
        (DATA_DIR / "rnf_summary.json").write_text(json.dumps(summary, ensure_ascii=False, indent=1))
    rnf_p1 = sum(1 for s in summary if s.get("RNF_P1"))
    urs_p1 = sum(1 for s in summary if s.get("URS_P1"))
    print(f"\n=== RNF P1 = {rnf_p1}/8 ≥BH | URS P1 = {urs_p1}/8 ≥BH ===", flush=True)
    print(json.dumps(summary, ensure_ascii=False, indent=1), flush=True)


if __name__ == "__main__":
    main()
