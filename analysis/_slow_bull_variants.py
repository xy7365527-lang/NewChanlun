"""离线判据变体评估：消费 _slow_bull_probe_<SYM>.json 的 trim_rows。

对每标的 × 模式，把逐笔 trim 按豁免判据变体分桶，报告被豁免桶的
加权 trim_alpha（= 该判据下停削可直接回收/损失的 nats，链式效应除外）：

  E0 任意祖先：∃ j>k, trend∧up(j)
  E1 距离≥2：∃ j≥k+2
  E2 祖先≥recL2：∃ j≥max(k+1,4)
  E3 祖先≥recL3：∃ j≥max(k+1,5)

另输出：E0 豁免桶的 per-ladder 与 per-regime（年度 bull/bear/range，
bh_log ±0.10 nats 口径，QQQ 无时间戳跳过）分解——验证熊市削减不被豁免。

用法：.venv/bin/python analysis/_slow_bull_variants.py
"""

from __future__ import annotations

import json
from pathlib import Path

DATA_DIR = Path(__file__).resolve().parent / "data_cache"
SYMBOLS = ["ES", "QQQ", "BTC", "GC", "DX"]
MODES = ["hold26", "fusion_t"]
LADDER_NAMES = {2: "segment", 3: "move(L1)", 4: "recL2", 5: "recL3",
                6: "recL4", 7: "recL5"}
REGIME_NATS = 0.10

VARIANTS = {
    "E0_any_ancestor": lambda lad, mask: bool(mask >> (lad + 1)),
    "E1_dist_ge2": lambda lad, mask: bool(mask >> (lad + 2)),
    "E2_anc_ge_recL2": lambda lad, mask: bool(mask >> max(lad + 1, 4)),
    "E3_anc_ge_recL3": lambda lad, mask: bool(mask >> max(lad + 1, 5)),
}


def year_regimes(sym: str) -> dict | None:
    """从 fusion_t_eight_<SYM>.json 的 yearly bh_log 取 regime 标签。"""
    p = DATA_DIR / f"fusion_t_eight_{sym}.json"
    if not p.exists():
        return None
    d = json.loads(p.read_text())
    yearly = d["modes"]["hold26"].get("yearly")
    if yearly is None:
        return None
    out = {}
    for y, r in yearly.items():
        b = r["bh_log"]
        out[int(y)] = ("bull" if b >= REGIME_NATS
                       else "bear" if b <= -REGIME_NATS else "range")
    return out


def main() -> None:
    for sym in SYMBOLS:
        p = DATA_DIR / f"_slow_bull_probe_{sym}.json"
        d = json.loads(p.read_text())
        regimes = year_regimes(sym)
        print(f"\n════ {sym} (BH {d['bh_pct']}%) ════")
        for mode in MODES:
            m = d["modes"][mode]
            rows = m.get("trim_rows")
            if rows is None:
                print(f" [{mode}] 无 trim_rows（旧版探针输出），跳过")
                continue
            print(f" [{mode}] strat={m['strat_pct']:+.1f}% "
                  f"n_pairs={len(rows)}")
            for vname, fn in VARIANTS.items():
                ex_n = ex_wa = rest_wa = 0.0
                for lad, yr, xb, a, w, mask in rows:
                    if fn(lad, mask):
                        ex_n += 1
                        ex_wa += w * a
                    else:
                        rest_wa += w * a
                print(f"   {vname:18s} 豁免n={int(ex_n):6d} "
                      f"豁免wα={ex_wa:+.4f}（压制=回收 {-ex_wa:+.4f}） "
                      f"剩余wα={rest_wa:+.4f}")
            # E0 豁免桶 per-ladder
            per_lad: dict = {}
            for lad, yr, xb, a, w, mask in rows:
                if bool(mask >> (lad + 1)):
                    e = per_lad.setdefault(lad, [0, 0.0])
                    e[0] += 1
                    e[1] += w * a
            print("   E0豁免桶 per-ladder: "
                  + " ".join(f"{LADDER_NAMES.get(k, k)}:n={v[0]},"
                             f"wα={v[1]:+.3f}"
                             for k, v in sorted(per_lad.items())))
            # E0 豁免桶 per-regime + 熊市豁免泄漏检查
            if regimes:
                per_reg: dict = {}
                bear_total = bear_exempt = 0
                for lad, yr, xb, a, w, mask in rows:
                    if yr is None:
                        continue
                    reg = regimes.get(yr, "?")
                    if reg == "bear":
                        bear_total += 1
                    if bool(mask >> (lad + 1)):
                        e = per_reg.setdefault(reg, [0, 0.0])
                        e[0] += 1
                        e[1] += w * a
                        if reg == "bear":
                            bear_exempt += 1
                print("   E0豁免桶 per-regime: "
                      + " ".join(f"{r}:n={v[0]},wα={v[1]:+.3f}"
                                 for r, v in sorted(per_reg.items()))
                      + f" | 熊市trim豁免泄漏 {bear_exempt}/{bear_total}")


if __name__ == "__main__":
    main()
