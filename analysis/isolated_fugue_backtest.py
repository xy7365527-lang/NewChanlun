"""逐仓独立声部森林(iso)回测——栈→森林升级,四改动一次性实装。

═══════════════════════════════════════════════════════════════════════
设计(编排者 2026-06-14:把已逐仓的 voice 栈升级为逐仓 voice 森林)
═══════════════════════════════════════════════════════════════════════
src/trading/isolated_fugue.rs。与 v4(nested_fugue)/URS(unified_recursive) 的
构成性差异 = 数据结构从 voice **栈**(每级别至多一 voice、链尾操作、全局
acted 互斥)升级为 voice **森林**:
  1. per-voice acted(去全局互斥)——同 bar 多 level/多 voice 独立操作。
  2. Type2 接入区间套窗口(中枢回测确认词汇,概念链第12环)。
  3. 净额→逐仓森林:VoiceLedger 树(root 可多 child),守恒律 §8.1/§8.3 每 bar 守卫。
  4. 逐 level 独立 BSP 消费(改动1 推论)。
清仓层 = URS E* 涌现归属(cascade 全树回现金,§6)。零 flag,唯一参数 a0。

═══════════════════════════════════════════════════════════════════════
预注册判据(先于运行;L2 单标的 / L3 多标的)
═══════════════════════════════════════════════════════════════════════
R_capture(核心): iso 捕获信号量 > urs/v4(spawns/n_trades 严格更多)——直接检验
  森林是否吃到了栈丢失的信号("没吃到"的根因诊断)。这是机制存在性验证(L1→L2)。
P1: iso strat ≥ BH(任务终极目标:吃到买卖点 → 跑赢 BH)。
R_mdd: MDD vs BH。
vs: iso vs v4 / iso vs urs delta_pp(森林是否把多吃的信号转化为 alpha)。
否证价值:若 R_capture 成立但 P1/vs 不成立,坐实"多吃信号≠多赚"(信号捕获与
  alpha 正交;栈纪律可能是特征非 bug——参 共享仓位 L2 否证先例)。

用法: PYTHONPATH=src .venv/bin/python analysis/isolated_fugue_backtest.py [SYM ...]
输出: analysis/data_cache/iso_<SYM>.json + iso_summary.json
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
from nested_recursive_fugue_final_backtest import FLOOR, analyze, bh_mdd  # noqa: E402
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
SYMBOLS = sys.argv[1:] or ["OKLO", "QQQ", "BRN", "DX", "ES", "GC", "CL", "BTC"]


def _load_cached(prefix: str, blk_key: str, sym: str) -> dict | None:
    """读已缓存的 v4/urs 结果(对比基线)。"""
    p = DATA_DIR / f"{prefix}_{sym}.json"
    if not p.exists():
        return None
    d = json.loads(p.read_text())
    if "failed" in d:
        return None
    blk = d.get(blk_key, {})
    pr = d.get("preregistered", {})
    sellpt = pr.get("sellpt") or blk.get("exit_reasons", {}).get("sellpt", 0)
    return {
        "strat_pct": blk.get("strat_pct"),
        "mdd_pct": blk.get("mdd_pct"),
        "n_trades": blk.get("n_trades"),
        "spawns": sum(blk.get("nrf_counters", {}).get("spawns", []) or [0]),
        "sellpt": sellpt,
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
    # settle 门 on(构成性确认贯通);iso 只需 dir_flips(E* 读 dir/anchor)。
    tape = compute_organic_signals(
        opens, highs, lows, closes, dir_flips=dir_flips, require_settled=True
    )
    print(f"[{sym}] 信号层(settle on) {time.time()-t0:.1f}s flips={len(dir_flips)}", flush=True)
    rtape = pack_tape(tape, dir_flips=dir_flips)

    v4 = _load_cached("nrf_v4", "nrf_v4", sym)
    urs = _load_cached("urs", "urs", sym)

    t1 = time.time()
    res = nr.run_positional_rust(rtape, floor_ladder=FLOOR, mode="iso")
    a = analyze(res, closes, years)
    a["nrf_counters"]["deep_fires"] = res["n_nrf_deep_fires_by_ladder"]
    a["nrf_counters"]["earning_units"] = round(res["nrf_earning_units"], 3)
    sellpt = a["exit_reasons"].get("sellpt", 0)
    c = a["nrf_counters"]
    spawns = sum(c["spawns"])
    pr: dict = {
        "P1_ge_bh": [a["strat_pct"], round(bh_pct, 1), a["strat_pct"] >= bh_pct],
        "sellpt": sellpt,
        "spawns": spawns,
        "n_trades": a["n_trades"],
        "deep_fires": sum(c["deep_fires"]),
        "liquidations": sum(c["liquidations"]),
        "phys_long_frac": round(c["phys_long_bars"] / n, 3),
        "R_mdd_ge_bh": [a["mdd_pct"], round(bh_dd, 1), a["mdd_pct"] >= bh_dd],
    }
    # R_capture:森林是否多吃信号(vs urs/v4 的 spawns/trades)。
    base = urs or v4
    if base is not None:
        pr["R_capture"] = {
            "iso_spawns": spawns,
            "base_spawns": base["spawns"],
            "iso_trades": a["n_trades"],
            "base_trades": base["n_trades"],
            "more_signals": (spawns > (base["spawns"] or 0))
            or (a["n_trades"] > (base["n_trades"] or 0)),
        }
    if v4 is not None:
        pr["vs_v4"] = {
            "strat_v4": v4["strat_pct"],
            "strat_iso": a["strat_pct"],
            "delta_pp": round(a["strat_pct"] - (v4["strat_pct"] or 0), 1),
        }
    if urs is not None:
        pr["vs_urs"] = {
            "strat_urs": urs["strat_pct"],
            "strat_iso": a["strat_pct"],
            "delta_pp": round(a["strat_pct"] - (urs["strat_pct"] or 0), 1),
        }
    print(
        f"[{sym}][iso] {time.time()-t1:.1f}s strat={a['strat_pct']:+.1f}% "
        f"mdd={a['mdd_pct']}% trades={a['n_trades']} spawns={spawns} sellpt={sellpt} "
        f"deep={pr['deep_fires']} P1={pr['P1_ge_bh'][2]}",
        flush=True,
    )
    return {
        "symbol": sym,
        "n_bars": n,
        "bh_pct": round(bh_pct, 1),
        "bh_mdd_pct": round(bh_dd, 1),
        "iso": a,
        "preregistered": pr,
        "design": "iso:逐仓独立声部森林(per-voice acted+Type2+树形隔离会计),E*清仓,零flag",
    }


def main() -> None:
    summary = []
    for sym in SYMBOLS:
        try:
            out = run_symbol(sym)
        except Exception as e:  # noqa: BLE001
            import traceback

            traceback.print_exc()
            out = {"symbol": sym, "failed": f"{type(e).__name__}: {e}"}
            print(f"[{sym}] FAILED {out['failed']}", flush=True)
        (DATA_DIR / f"iso_{sym}.json").write_text(
            json.dumps(out, ensure_ascii=False, indent=1)
        )
        if "failed" in out:
            summary.append({"symbol": sym, "failed": out["failed"]})
        else:
            p = out["preregistered"]
            summary.append(
                {
                    "symbol": sym,
                    "bh_pct": out["bh_pct"],
                    "iso_pct": out["iso"]["strat_pct"],
                    "iso_mdd": out["iso"]["mdd_pct"],
                    "P1": p["P1_ge_bh"][2],
                    "spawns": p["spawns"],
                    "trades": p["n_trades"],
                    "sellpt": p["sellpt"],
                    "R_capture": p.get("R_capture", {}).get("more_signals"),
                    "vs_v4": p.get("vs_v4", {}).get("delta_pp"),
                    "vs_urs": p.get("vs_urs", {}).get("delta_pp"),
                }
            )
        (DATA_DIR / "iso_summary.json").write_text(
            json.dumps(summary, ensure_ascii=False, indent=1)
        )
    print(json.dumps(summary, ensure_ascii=False, indent=1), flush=True)


if __name__ == "__main__":
    main()
