"""构成性贯通回测——NRF v4 基座 + settle门 + A′合取选层 + R6′ regime门。

═══════════════════════════════════════════════════════════════════════
设计（编排者 2026-06-13：让构成性必然贯通）
═══════════════════════════════════════════════════════════════════════
概念运动链要求：次级别走势完成→线段确立→中枢边界确立→BSP有效→清仓有效。
v4 基座（`docs/nested_fugue_accounting.md` §1-§8 会计规则不变）的两个断点：
  1. **确认环节断**：settle 门 off ⟹ 生长期 type2 伪信号（29.6%）未过滤。
  2. **消费环节断**：清仓判据 = sell1[top]∧located[top]，top=θ 棘轮累积层
     ⟹ 棘轮单调攀高 + 同层合取把频率钉死（CL=OKLO=3，与波动率脱钩）。

三步修复（构成性贯通实验臂，mode="nrf_ct"）：
  - **Step 1 settle门**：require_settled=True（显式参数，**不**改全局默认——
    bit-exact 移植契约 + formalization-validity-domain：settle门 Piece 2C L3
    已证为 regime 函数 BTC −203pp，全局 True 是有效域膨胀。实验臂显式开门即
    达成构成性确认环节贯通，不破坏其他在册回测基线）。
  - **Step 2 A′合取选层**：top* = max{k≥floor : sell1[k] ∧ located[k]}
    （539号 §3.1；解耦 θ 棘轮、区别 v5 的 located 单独选层；located 作独立
    合取项 R5′）。
  - **Step 3 R6′ regime门**：trend_state[top*]==Trend（17课走势完全分类；
    趋势-type1=趋势衰竭清仓 / 盘整-type1=中枢震荡回中枢不清）。

对照基线（两臂同信号层 settle 门状态分离三效应）：
  - `nrf_v4_<SYM>.json`：v4，settle off，mode nrf（任务"原基线 3/8"）。
  - `nrf_v4sg_<SYM>.json`：v4，settle on，mode nrf（隔离 settle 门单独效应；
    vs nrf_ct 即隔离 A′+regime门）。

═══════════════════════════════════════════════════════════════════════
预注册判据（先于运行声明；L3）
═══════════════════════════════════════════════════════════════════════
任务 R1：CL 清仓频率增加（从 3 次 → 更多）。
任务 R2：OKLO 清仓不超过 v4（≤3 次）——防 v5 过度清仓（v5 OKLO 27 次踏空）。
任务 R3：八标的 P1 ≥ v4 的 3/8。
任务 R4：OKLO ≥ +695%（不恶化，v4 基线）。
任务 R5：MDD 8/8 仍全优于 BH。
539 R1′：清仓频率随波动率涌现分化（CL > OKLO）且**不**全标的同步过度
   （区别 v5 全标的暴涨 27-101）。
539 R2′：CL 三次主油崩（2014/2015/2020）≥2 次年度优于 BH（清仓落崩盘前顶）。
539 R3′：CL/OKLO/ES/GC 上涨年不显著劣于 v4（不踏空牛市，区别 v5 −642/−938）。
539 R6′：趋势标的踏空被 regime 门堵 ∧ 深崩域避险保留。
（539 R5′ = A′ 布尔分解两项各非恒真——由 A′ scan 代码构造保证，L0 非运行量。）

═══════════════════════════════════════════════════════════════════════
用法：PYTHONPATH=src .venv/bin/python analysis/constitutive_throughput_backtest.py [SYM ...]
输出：analysis/data_cache/nrf_ct_<SYM>.json + nrf_ct_summary.json
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
from nested_recursive_fugue_final_backtest import (  # noqa: E402
    FLOOR,
    analyze,
    bh_mdd,
)
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
SYMBOLS = sys.argv[1:] or ["OKLO", "QQQ", "BRN", "DX", "ES", "GC", "CL", "BTC"]

# R2′/R3′ 年度锚（cl_clearance_diagnosis.md §2）。
CL_CRASH_YEARS = ("2014", "2015", "2020")
CL_BULL_YEARS = ("2010", "2011", "2016", "2017", "2019")


def _load_baseline(prefix: str, sym: str, key: str) -> dict | None:
    """读对照基线（nrf_v4_<SYM> settle-off 或 nrf_v4sg_<SYM> settle-on）。"""
    p = DATA_DIR / f"{prefix}_{sym}.json"
    if not p.exists():
        return None
    d = json.loads(p.read_text())
    if "failed" in d:
        return None
    blk = d.get(key, {})
    pr = d.get("preregistered", {})
    sellpt = (pr.get("N1_liquidations_vs_spawns", {}).get("sellpt_closes")
              or pr.get("sellpt_on")
              or d.get(key, {}).get("exit_reasons", {}).get("sellpt", 0))
    return {
        "strat_pct": blk.get("strat_pct"),
        "mdd_pct": blk.get("mdd_pct"),
        "sellpt": sellpt,
        "yearly": blk.get("yearly"),
    }


def _crash_years_beat_bh(yearly: dict | None) -> dict:
    """R2′：CL 三次油崩年 strat 优于 BH 的次数。"""
    if not yearly:
        return {"checked": False}
    beat = {}
    for y in CL_CRASH_YEARS:
        row = yearly.get(y)
        if row:
            beat[y] = round(row["strat_log"] - row["bh_log"], 4)
    n_beat = sum(1 for v in beat.values() if v > 0)
    return {"checked": True, "per_year_strat_minus_bh": beat,
            "n_beat": n_beat, "R2p_pass": n_beat >= 2}


def prereg(a: dict, bh_pct: float, n_bars: int, sym: str,
           off: dict | None, sg: dict | None) -> dict:
    """nrf_ct 预注册判据裁决（任务 R1-R5 + 539 R1′-R6′）。"""
    c = a["nrf_counters"]
    sellpt_ct = a["exit_reasons"].get("sellpt", 0)
    strat_ct = a["strat_pct"]
    mdd_ct = a["mdd_pct"]
    out = {
        "P1_ge_bh": [strat_ct, round(bh_pct, 1), strat_ct >= bh_pct],
        "sellpt_ct": sellpt_ct,
        "spawns_ct": sum(c["spawns"]),
        "deep_fires_ct": sum(c["deep_fires"]),
        "liquidations_ct": sum(c["liquidations"]),
        "phys_long_frac_ct": round(c["phys_long_bars"] / n_bars, 3),
        "root_recL3_ct": c["root_entries"][5] if len(c["root_entries"]) > 5 else 0,
        # R5_mdd_beats_bh 由 run_symbol 用真实 BH_MDD 填充。
    }
    _ = mdd_ct
    if off is not None:
        out["vs_v4_off"] = {
            "strat_off": off["strat_pct"], "strat_ct": strat_ct,
            "delta_pp": round(strat_ct - (off["strat_pct"] or 0), 1),
            "sellpt_off": off["sellpt"], "sellpt_ct": sellpt_ct,
            "mdd_off": off["mdd_pct"], "mdd_ct": mdd_ct,
        }
    if sg is not None:
        out["vs_v4sg"] = {  # 隔离 A′+regime门（两臂同 settle on）
            "strat_sg": sg["strat_pct"], "strat_ct": strat_ct,
            "delta_pp": round(strat_ct - (sg["strat_pct"] or 0), 1),
            "sellpt_sg": sg["sellpt"], "sellpt_ct": sellpt_ct,
        }
    # 任务 R2 / 539 R3′ / R6′（趋势标的不踏空）：OKLO
    if sym == "OKLO":
        out["R2_oklo_not_over_clear"] = {
            "sellpt_ct": sellpt_ct, "le_3": sellpt_ct <= 3,
            "le_8_soft": sellpt_ct <= 8,
        }
        out["R4_oklo_not_degraded"] = {
            "strat_ct": strat_ct, "ge_695": strat_ct >= 695.0,
        }
    # 任务 R1 / 539 R1′ / R2′（CL 清仓解冻 + 油崩年避险）
    if sym == "CL":
        sellpt_off = (off or {}).get("sellpt", 0) or 0
        out["R1_cl_thaw"] = {
            "sellpt_off": sellpt_off, "sellpt_ct": sellpt_ct,
            "thawed": sellpt_ct > sellpt_off,
        }
        out["R2p_cl_crash_avoidance"] = _crash_years_beat_bh(a.get("yearly"))
    return out


def run_symbol(sym: str) -> dict:
    path = SYMBOL_FILES[sym]
    opens, highs, lows, closes, years = load_ohlc(path)
    n = len(closes)
    bh_pct = (closes[-1] / closes[0] - 1) * 100
    bh_dd = bh_mdd(closes) * 100
    print(f"[{sym}] bars={n:,} BH={bh_pct:+.1f}% BH_MDD={bh_dd:.1f}%", flush=True)

    t0 = time.time()
    dir_flips: list = []
    trend_flips: list = []
    # ★ settle 门显式开启（构成性确认环节）+ trend_flips（R6′ regime 门数据）
    tape = compute_organic_signals(opens, highs, lows, closes,
                                   dir_flips=dir_flips,
                                   trend_flips=trend_flips,
                                   require_settled=True)
    fp = {"bsp": sum(len(s.bsp_events[l]) for s in tape if s.bsp_events
                     for l in range(11)),
          "div": sum(len(s.div_events[l]) for s in tape if s.div_events
                     for l in range(11)),
          "flips": len(dir_flips), "tflips": len(trend_flips)}
    print(f"[{sym}] 信号层(settle on + trend) {time.time() - t0:.1f}s fp={fp}",
          flush=True)

    rtape = pack_tape(tape, dir_flips=dir_flips, trend_flips=trend_flips)

    off = _load_baseline("nrf_v4", sym, "nrf_v4")
    sg = _load_baseline("nrf_v4sg", sym, "nrf_v4_settle")

    def _run(mode: str) -> tuple[dict, dict]:
        res = nr.run_positional_rust(rtape, floor_ladder=FLOOR, mode=mode)
        a = analyze(res, closes, years)
        a["nrf_counters"]["deep_fires"] = res["n_nrf_deep_fires_by_ladder"]
        a["nrf_counters"]["earning_adds"] = res["n_nrf_earning_adds_by_ladder"]
        a["nrf_counters"]["earning_units"] = round(res["nrf_earning_units"], 3)
        a["nrf_counters"]["shrink_units"] = round(res["nrf_shrink_units"], 3)
        pr = prereg(a, bh_pct, n, sym, off, sg)
        pr["R5_mdd_beats_bh"] = [a["mdd_pct"], round(bh_dd, 1),
                                 a["mdd_pct"] >= bh_dd]  # MDD 负，越大越优
        return a, pr

    out: dict = {"symbol": sym, "n_bars": n, "tape_fp": fp,
                 "bh_pct": round(bh_pct, 1), "bh_mdd_pct": round(bh_dd, 1),
                 "design": "NRF v4 基座 + settle门(显式on) + A′合取选层 + R6′ "
                           "regime门两形态对照（539号开放轴实装）"}
    # 两 regime 门形态对照（同 tape，同 A′ 合取选层）：
    #   nrf_ct     = 走势类型门 trend_state[top*]（任务 body）
    #   nrf_ct_anc = anc 镜像门 ∃j>top* trend∧down（539 R6′ 原形式）
    for label, mode in (("ct", "nrf_ct"), ("ct_anc", "nrf_ct_anc")):
        t1 = time.time()
        a, pr = _run(mode)
        out[f"nrf_{label}"] = a
        out[f"preregistered_{label}"] = pr
        dpp = pr.get("vs_v4_off", {}).get("delta_pp", "NA")
        print(f"[{sym}][{mode}] {time.time() - t1:.1f}s "
              f"strat={a['strat_pct']:+.1f}% mdd={a['mdd_pct']}% "
              f"trades={a['n_trades']} sellpt={pr['sellpt_ct']} "
              f"deep={pr['deep_fires_ct']} P1={pr['P1_ge_bh'][2]} "
              f"Δv4off={dpp}pp", flush=True)
    return out


def main() -> None:
    summary = []
    for sym in SYMBOLS:
        try:
            out = run_symbol(sym)
        except Exception as e:  # 标的隔离
            import traceback
            traceback.print_exc()
            out = {"symbol": sym, "failed": f"{type(e).__name__}: {e}"}
            print(f"[{sym}] FAILED {out['failed']}", flush=True)
        (DATA_DIR / f"nrf_ct_{sym}.json").write_text(
            json.dumps(out, ensure_ascii=False, indent=1))
        if "failed" in out:
            summary.append({"symbol": sym, "failed": out["failed"]})
        else:
            summary.append({
                "symbol": sym, "bh_pct": out["bh_pct"],
                "ct_pct": out["nrf_ct"]["strat_pct"],
                "ct_mdd": out["nrf_ct"]["mdd_pct"],
                "ct_P1": out["preregistered_ct"]["P1_ge_bh"][2],
                "ct_sellpt": out["preregistered_ct"]["sellpt_ct"],
                "anc_pct": out["nrf_ct_anc"]["strat_pct"],
                "anc_mdd": out["nrf_ct_anc"]["mdd_pct"],
                "anc_P1": out["preregistered_ct_anc"]["P1_ge_bh"][2],
                "anc_sellpt": out["preregistered_ct_anc"]["sellpt_ct"],
            })
        (DATA_DIR / "nrf_ct_summary.json").write_text(
            json.dumps(summary, ensure_ascii=False, indent=1))
    print(json.dumps(summary, ensure_ascii=False, indent=1), flush=True)


if __name__ == "__main__":
    main()
