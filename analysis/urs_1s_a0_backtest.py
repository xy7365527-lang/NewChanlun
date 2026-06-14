"""URS 1s a0 回测——E*涌现层加深能否修正强牛清仓踏空(ES/BTC 1min失败标的)。

═══════════════════════════════════════════════════════════════════════
设计(编排者 2026-06-13 补充①:a0用1s,递归深度+1层lid4涌现,区间套更多精化层)
═══════════════════════════════════════════════════════════════════════
URS(mode=urs)清仓层=根voice涌现归属级别E*(从入场级别向上爬,父级别须①dir==Up
②anchor>=entry)。E*爬升空间 = 涌现层深度。假设:1min a0涌现层不够深 ⇒ E*爬升
受限 ⇒ 强牛(ES/BTC)清仓踏空。1s a0多一层(lid4) ⇒ E*更多精化层 ⇒ 清仓更准?

数据(只有这三个有1s):
  - ES1Y(1年,URS-1min失败 +560.8<BH+594.3) ← 核心:1s能否翻正?
  - CL1Y(1年,URS-1min成功 +126.4>BH+28.2) ← 对照:成功标的1s保持?
  - BTC2W(2周,URS-1min失败) ← 样本小,符号+MDD守恒检验

口径(历史教训project_cl_1s_a0_verdict:1s交易增益=零摩擦伪影,盈亏平衡0.58bps<taker1.45):
  - 零摩擦strat(批量回测器,与1min口径可比)
  - 含摩擦strat(maker单侧friction,DATASETS表)——守卫零摩擦伪影
  - floor双臂:1s floor=3(密度锚1s_l4≈1min_l3,对齐1min floor=2)主臂;
            1s floor=2(P4反例:操作床位过细交易爆炸)对照
  - 1min桶塔(同源墙钟分钟聚合)floor=2 = 同窗对照基线

═══════════════════════════════════════════════════════════════════════
预注册判据(先于运行;L3;Nautilus口径=批量回测器非流式,执行层接通为后续里程碑)
═══════════════════════════════════════════════════════════════════════
PR1(核心): ES 1s URS(f3,零摩擦) strat ≥ 同窗BH(1min URS失败ES)。1s翻正=E*加深修正踏空。
PR2: CL 1s URS(f3) 不劣于同窗1min URS(成功标的不被1s破坏)。
PR3(E*机制): 1s塔涌现更深(root_recL4>0 或 max涌现层 > 1min塔)——给E*更多爬升层。
PR4(零摩擦伪影守卫): 1s URS优势在含摩擦口径下存活(否则=秒级伪影,非清仓层改善)。
PR5(清仓分化): 1s sellpt时点 vs 1min同窗——清仓数变化方向(更准=踏空减少)。
否证价值:若1s不翻正ES/BTC,坐实"强牛清仓负期望"不是涌现层深度问题(E*已最优),
        是清仓动作本身结构边界(URS报告§4开放轴)。

用法: PYTHONPATH=src .venv/bin/python analysis/urs_1s_a0_backtest.py [KEY ...]
输出: analysis/data_cache/urs_1s_<KEY>.json + urs_1s_summary.json
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

DATA_DIR = ROOT / "analysis" / "data_cache"
# KEY → (文件, 格式, maker单侧摩擦率)  —— 同 nest_coverage_1s_a0.DATASETS
DATASETS = {
    "ES1Y": ("es_1s_databento_1y.json", "databento", 0.00005),
    "CL1Y": ("cl_1s_databento_1y.json", "databento", 0.0001),
    "BTC2W": ("btc_1s_2week.json", "binance", 0.0002),
    "ES2W": ("es_1s_2week.json", "databento", 0.00005),
}
KEYS = sys.argv[1:] or ["BTC2W", "ES1Y", "CL1Y"]


def load_1s(key):
    """加载 1s OHLCV。两种 dict 格式:databento(timestamps_ns纳秒) / binance(dates字符串)。
    返回 (ts_sec, o, h, l, c, friction)。"""
    fname, fmt, friction = DATASETS[key]
    raw = json.loads((DATA_DIR / fname).read_text())
    o = raw["opens"]; h = raw["highs"]; l = raw["lows"]; c = raw["closes"]
    if "timestamps_ns" in raw:
        ts = [t // 1_000_000_000 for t in raw["timestamps_ns"]]
    elif "dates" in raw:
        from datetime import datetime
        ts = [int(datetime.fromisoformat(d).timestamp()) for d in raw["dates"]]
    else:
        raise KeyError(f"{fname} 无 timestamps_ns/dates 时间字段")
    return ts, o, h, l, c, friction


def bucket_1min(ts, o, h, l, c):
    bo, bh, bl, bc = [], [], [], []
    cur = None
    for i in range(len(ts)):
        m = ts[i] // 60
        if m != cur:
            bo.append(o[i]); bh.append(h[i]); bl.append(l[i]); bc.append(c[i]); cur = m
        else:
            bh[-1] = max(bh[-1], h[i]); bl[-1] = min(bl[-1], l[i]); bc[-1] = c[i]
    return bo, bh, bl, bc


def _friction_adj(res, friction):
    """含摩擦 strat:每笔 trade 双侧 friction 扣减(notional×friction×2)。
    trades 元素 = 11元组(lad,eb,ep,xb,xp,sh,w,dfr,part,reason,pol)。"""
    cost = 0.0
    for t in res["trades"]:
        ep, xp, sh = t[2], t[4], t[5]
        cost += abs(sh) * (ep + xp) / 2 * friction * 2
    return (res["final_nav"] - cost) / 100000.0 * 100 - 100


def run_urs(rtape, floor, closes, friction):
    res = nr.run_positional_rust(rtape, floor_ladder=floor, mode="urs")
    a = analyze(res, closes, None)
    re_ = res["n_nrf_root_entries_by_ladder"]
    return {
        "strat_zero_fric": a["strat_pct"],
        "strat_with_fric": round(_friction_adj(res, friction), 1),
        "mdd": a["mdd_pct"],
        "n_trades": a["n_trades"],
        "sellpt": a["exit_reasons"].get("sellpt", 0),
        "spawns": sum(res["n_nrf_spawns_by_ladder"]),
        "deep_fires": sum(res["n_nrf_deep_fires_by_ladder"]),
        "root_recL4": re_[6] if len(re_) > 6 else 0,
        "root_recL3": re_[5] if len(re_) > 5 else 0,
        "max_root_layer": max((i for i, x in enumerate(re_) if x > 0), default=-1),
    }


def run_key(key):
    ts, o, h, l, c, friction = load_1s(key)
    n1s = len(c)
    bh = (c[-1] / c[0] - 1) * 100
    bh_dd = bh_mdd(c) * 100
    print(f"[{key}] 1s bars={n1s:,} BH={bh:+.1f}% BH_MDD={bh_dd:.1f}% friction={friction}", flush=True)

    t0 = time.time()
    df1s = []
    tape_1s = compute_organic_signals(o, h, l, c, dir_flips=df1s, require_settled=True)
    rt_1s = pack_tape(tape_1s, dir_flips=df1s)
    print(f"[{key}] 1s信号层 {time.time()-t0:.1f}s flips={len(df1s)}", flush=True)

    t0 = time.time()
    bo, bh_, bl, bc = bucket_1min(ts, o, h, l, c)
    df1m = []
    tape_1m = compute_organic_signals(bo, bh_, bl, bc, dir_flips=df1m, require_settled=True)
    rt_1m = pack_tape(tape_1m, dir_flips=df1m)
    bh_1m = (bc[-1] / bc[0] - 1) * 100  # 桶塔BH(同窗,应≈1s BH)
    print(f"[{key}] 1min桶 bars={len(bc):,} {time.time()-t0:.1f}s flips={len(df1m)}", flush=True)

    out = {"key": key, "n_1s": n1s, "n_1min": len(bc), "bh_pct": round(bh, 1),
           "bh_mdd_pct": round(bh_dd, 1), "friction": friction,
           "design": "URS 1s a0:E*涌现层加深修正强牛清仓踏空;同窗1min桶对照;含摩擦守卫零摩擦伪影"}
    arms = {}
    # 1s 塔:floor=3(密度锚主臂) + floor=2(P4反例)
    for f in (3, 2):
        t1 = time.time()
        arms[f"1s_f{f}"] = run_urs(rt_1s, f, c, friction)
        print(f"[{key}][1s f{f}] {time.time()-t1:.1f}s "
              f"strat0={arms[f'1s_f{f}']['strat_zero_fric']:+.1f}% "
              f"stratF={arms[f'1s_f{f}']['strat_with_fric']:+.1f}% "
              f"mdd={arms[f'1s_f{f}']['mdd']}% sellpt={arms[f'1s_f{f}']['sellpt']} "
              f"recL4={arms[f'1s_f{f}']['root_recL4']} maxlayer={arms[f'1s_f{f}']['max_root_layer']}", flush=True)
    # 1min 桶塔:floor=2(同窗基线)
    t1 = time.time()
    arms["1min_f2"] = run_urs(rt_1m, 2, bc, friction)
    arms["1min_f2"]["bh_pct"] = round(bh_1m, 1)
    print(f"[{key}][1min f2] {time.time()-t1:.1f}s "
          f"strat0={arms['1min_f2']['strat_zero_fric']:+.1f}% sellpt={arms['1min_f2']['sellpt']} "
          f"maxlayer={arms['1min_f2']['max_root_layer']}", flush=True)
    out["arms"] = arms
    # 预注册裁决
    s1s3 = arms["1s_f3"]; s1m = arms["1min_f2"]
    out["preregistered"] = {
        "PR1_1s_f3_zero_ge_bh": [s1s3["strat_zero_fric"], round(bh, 1), s1s3["strat_zero_fric"] >= bh],
        "PR3_deeper_emergence": {"1s_f3_maxlayer": s1s3["max_root_layer"],
                                 "1min_maxlayer": s1m["max_root_layer"],
                                 "1s_recL4": s1s3["root_recL4"],
                                 "deeper": s1s3["max_root_layer"] > s1m["max_root_layer"]},
        "PR4_fric_artifact_guard": {"zero": s1s3["strat_zero_fric"], "fric": s1s3["strat_with_fric"],
                                    "survives": s1s3["strat_with_fric"] >= bh},
        "PR5_clearance": {"1s_f3_sellpt": s1s3["sellpt"], "1min_sellpt": s1m["sellpt"],
                          "1s_vs_1min_strat0_pp": round(s1s3["strat_zero_fric"] - s1m["strat_zero_fric"], 1)},
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
            print(f"[{key}] FAILED {out['failed']}", flush=True)
        (DATA_DIR / f"urs_1s_{key}.json").write_text(json.dumps(out, ensure_ascii=False, indent=1))
        if "failed" not in out:
            pr = out["preregistered"]
            summary.append({"key": key, "bh": out["bh_pct"],
                            "1s_f3_zero": out["arms"]["1s_f3"]["strat_zero_fric"],
                            "1s_f3_fric": out["arms"]["1s_f3"]["strat_with_fric"],
                            "1min_f2": out["arms"]["1min_f2"]["strat_zero_fric"],
                            "PR1": pr["PR1_1s_f3_zero_ge_bh"][2],
                            "PR3_deeper": pr["PR3_deeper_emergence"]["deeper"],
                            "PR4_survives": pr["PR4_fric_artifact_guard"]["survives"]})
        else:
            summary.append({"key": key, "failed": out["failed"]})
        (DATA_DIR / "urs_1s_summary.json").write_text(json.dumps(summary, ensure_ascii=False, indent=1))
    print(json.dumps(summary, ensure_ascii=False, indent=1), flush=True)


if __name__ == "__main__":
    main()
