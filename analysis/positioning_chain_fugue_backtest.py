"""区间套定位链驱动赋格（pcf，mode="pcf"）八标的必然性检验 + 回测。

上游：编排者 2026-06-14"那你要实现啊"。`nested_interval_fugue` 用**固定
min_trade_ladder** 模拟区间套（结果是 regime 函数）。固定参数不是区间套——
区间套（第14环）是操作层级由**走势结构动态决定**（定位链顶层 source_ladder），
非外部 floor。

实装：`rust/src/trading/positioning_chain_fugue.rs`。2026-06-14 编排者"他某种意义上
是必然递归的，你来实装，严格实装"——把 located 武装从**自下而上**（要求全层同时
对齐 ⇒ source 坍缩到 segment 92-95%）改为**自上而下级联**（nf@k 触发 ⇒ 级联武装
[segment..=k] 全层，source=k = 最高有 located 的层）。与 URS 唯一构成性差异：操作层
= source = 级联链顶。双侧定位链（卖侧出场 + 买侧入场）。零参数——无 min_trade_ladder。

══════════════ 必然性检验（验收标准——先于回测，L0/L2）══════════════

引擎内 `prove_chain` 在 F/C/D/E 每个操作点运行时证明（违反即 panic）：
  N1（完整链）：每个操作的 source 对应一条 [segment..=S] 全 located 的连续前缀
     （级联构造保证）——"没有定位链的交易 = bug"的逐操作硬断言。
  N2（因果）：located.arm_bar ≤ 操作 bar（链在操作前级联，无未来定位）。
  N3（链顶一致）：located[s].source_ladder == s（级联统一 source 正确）。
⇒ 8 标的真实数据跑通**无 panic** = N1∧N2∧N3 在 ~25M bar 上成立（L2 必然性验证）。
守恒律（§8.1）每 bar assert，违反即 panic = 会计正确性 L2 验证。

  N_dist（源分布——级联是否起作用）：entry/spawn 的 source 分布。自下而上坍缩到
     segment 92-95%；自上而下应上移（src_levels 覆盖多层 ∧ segment 占比下降）。
     注：高级别 candidate 本就稀少 ⇒ segment 仍可能占多数（级联不凭空造高 source）。

脚本侧逻辑验证（消费 res，证明 source 驱动操作）：
  N4（source 决定操作量）：spawn 量 = parent.units × θ_source（构造保证；脚本
     报告 spawns by ladder 分布，验证降成本量随 source 层变化）。
  N5（出场对齐 source）：flip/clear 仅在卖链 source ≥ root.ladder（构造保证；
     报告 sellpt/flip 计数，验证非任意级别反向信号触发出场）。

══════════════ 回测判据（必然性通过后；L2/L3——非验收标准）══════════════

P1（对比）：pcf ≥ BH 逐标的 + 对比 URS 基线（缓存 urs_<SYM>.json）。
   **回测结果不是验收标准**（编排者：必然性检验才是）——P1 是有效域读数。
ΔURS：strat_pcf − strat_urs；定位 source 链驱动 vs URS 涌现层爬升的差异。

谱系（强制——与 539号已否证 A′ 区分）：A′ 单层 located 选清仓层 ⇒ 强牛踏空
   （L3 否证，project_constitutive_throughput_falsified）。pcf 要求整条链 +
   入场-出场 source 绑定 ⇒ 强牛高层卖链罕见 ⇒ 不踏空（待检验假设，非结论）。
   若 pcf 回测复现踏空（强牛标的 sellpt 暴涨、跑输 BH）⇒ source 绑定不足以堵
   踏空，A′ 同族死因延伸；若不复现 ⇒ source 绑定是新机制轴。

用法：PYTHONPATH=src .venv/bin/python analysis/positioning_chain_fugue_backtest.py [SYM ...]
输出：analysis/data_cache/pcf_<SYM>.json + pcf_summary.json
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
    LADDER_NAMES,
    analyze,
    bh_mdd,
)
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
SYMBOLS = sys.argv[1:] or ["OKLO", "QQQ", "BRN", "DX", "ES", "GC", "CL", "BTC"]


def _load_urs(sym: str) -> dict | None:
    p = DATA_DIR / f"urs_{sym}.json"
    if not p.exists():
        return None
    d = json.loads(p.read_text())
    if "failed" in d:
        return None
    blk = d.get("urs", {})
    return {
        "strat_pct": blk.get("strat_pct"),
        "mdd_pct": blk.get("mdd_pct"),
        "sellpt": blk.get("exit_reasons", {}).get("sellpt", 0),
        "p1": d.get("preregistered", {}).get("P1_ge_bh", [None, None, None])[2],
    }


def necessity(a: dict, res: dict, n: int) -> dict:
    """脚本侧必然性逻辑读数（N4/N5）+ 操作层分布（source 链动态选层的证据）。

    引擎内 prove_chain（N1/N2/N3）已在运行无 panic 时成立——本函数读 res 验证
    source 确实驱动操作层级与量（非固定 floor）。
    """
    nrfc = a["nrf_counters"]
    ent = res["n_entries_by_ladder"]
    root_ent = res["n_nrf_root_entries_by_ladder"]
    spawns = res["n_nrf_spawns_by_ladder"]
    flips = res["n_nrf_root_flips_by_ladder"]
    # 操作层分布：哪些 source 层产生了入场/降成本（≥2 个不同层 = source 动态选层，
    # 非固定 floor 钉死单层）。
    entry_levels = [k for k in range(len(root_ent)) if root_ent[k] > 0]
    spawn_levels = [k for k in range(len(spawns)) if spawns[k] > 0]
    return {
        "N1_N2_N3_runtime_proof": "PASS（引擎跑通无 panic ⇒ 每操作有完整链+因果+链顶一致）",
        "entry_source_levels": [(k, LADDER_NAMES.get(k, str(k)), root_ent[k]) for k in entry_levels],
        "spawn_source_levels": [(k, LADDER_NAMES.get(k, str(k)), spawns[k]) for k in spawn_levels],
        "N_source_dynamic": len(set(entry_levels) | set(spawn_levels)),
        "flips_by_ladder": [(k, flips[k]) for k in range(len(flips)) if flips[k] > 0],
        "sellpt_clear": a["exit_reasons"].get("sellpt", 0),
        "total_root_entries": sum(root_ent),
        "total_spawns": sum(spawns),
        "deep_fires": sum(res["n_nrf_deep_fires_by_ladder"]),
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
    # ★ 与 URS 基线逐字一致：settle on + 仅 dir_flips（唯一变量 = 引擎层选择机制）。
    tape = compute_organic_signals(opens, highs, lows, closes,
                                   dir_flips=dir_flips, require_settled=True)
    print(f"[{sym}] 信号层(settle on) {time.time() - t0:.1f}s flips={len(dir_flips)}",
          flush=True)
    rtape = pack_tape(tape, dir_flips=dir_flips)
    urs = _load_urs(sym)

    t1 = time.time()
    # 必然性检验：引擎内 prove_chain 违反即 panic——此调用跑通即 N1/N2/N3 成立。
    res = nr.run_positional_rust(rtape, floor_ladder=FLOOR, mode="pcf")
    a = analyze(res, closes, years)
    a["nrf_counters"]["deep_fires"] = res["n_nrf_deep_fires_by_ladder"]
    nec = necessity(a, res, n)

    p1 = a["strat_pct"] >= bh_pct
    d_urs = round(a["strat_pct"] - (urs["strat_pct"] or 0), 1) if urs else None
    print(f"[{sym}][pcf] {time.time() - t1:.1f}s strat={a['strat_pct']:+.1f}% "
          f"mdd={a['mdd_pct']}% trades={a['n_trades']} "
          f"roots={nec['total_root_entries']} spawns={nec['total_spawns']} "
          f"sellpt={nec['sellpt_clear']} src_levels={nec['N_source_dynamic']} "
          f"P1={p1} ΔURS={d_urs}pp", flush=True)

    return {
        "symbol": sym, "n_bars": n, "bh_pct": round(bh_pct, 1),
        "bh_mdd_pct": round(bh_dd, 1),
        "design": "区间套定位链驱动赋格 pcf：操作层=source=完整定位链顶层（零参数）",
        "pcf": a,
        "necessity": nec,
        "P1_ge_bh": [a["strat_pct"], round(bh_pct, 1), p1],
        "vs_urs": ({"strat_urs": urs["strat_pct"], "strat_pcf": a["strat_pct"],
                    "delta_pp": d_urs, "mdd_urs": urs["mdd_pct"], "mdd_pcf": a["mdd_pct"],
                    "sellpt_urs": urs["sellpt"], "sellpt_pcf": nec["sellpt_clear"]}
                   if urs else None),
    }


def main() -> None:
    summary = []
    for sym in SYMBOLS:
        try:
            out = run_symbol(sym)
        except Exception as e:  # 标的隔离；prove_chain panic 在此显形为必然性失败
            import traceback
            traceback.print_exc()
            out = {"symbol": sym, "failed": f"{type(e).__name__}: {e}"}
            print(f"[{sym}] FAILED（必然性检验失败 or 数据缺失）{out['failed']}", flush=True)
        (DATA_DIR / f"pcf_{sym}.json").write_text(
            json.dumps(out, ensure_ascii=False, indent=1))
        if "failed" in out:
            summary.append({"symbol": sym, "failed": out["failed"]})
        else:
            summary.append({
                "symbol": sym, "bh_pct": out["bh_pct"],
                "pcf_pct": out["pcf"]["strat_pct"], "pcf_mdd": out["pcf"]["mdd_pct"],
                "P1": out["P1_ge_bh"][2],
                "dURS": (out["vs_urs"] or {}).get("delta_pp"),
                "src_levels": out["necessity"]["N_source_dynamic"],
                "roots": out["necessity"]["total_root_entries"],
                "spawns": out["necessity"]["total_spawns"],
                "sellpt": out["necessity"]["sellpt_clear"],
            })
        (DATA_DIR / "pcf_summary.json").write_text(
            json.dumps(summary, ensure_ascii=False, indent=1))
    print(json.dumps(summary, ensure_ascii=False, indent=1), flush=True)


if __name__ == "__main__":
    main()
