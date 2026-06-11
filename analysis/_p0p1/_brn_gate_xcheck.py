"""一次性交叉复现守卫：重建后的共享库在 BRN 上复现 P1 门变体在册值。
P1 在册（analysis/_p0p1/p1_dual_gate.md 四格表）：
  V2ofF1c +48.94 / V2ofF1g +58.67 / V2ofF1cg +48.94
"""
import sys, time
from pathlib import Path
ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "src")); sys.path.insert(0, str(ROOT / "analysis"))
import newchan_rust as nr
from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc
from fugue_version_i import extended_metrics, LADDER_SEG
from organic_fugue_rust_backtest import _trades_from_rust
from organic_fugue_rust_check import pack_tape
from organic_signals import compute_organic_signals

REG = {"V2ofF1c": 48.94, "V2ofF1g": 58.67, "V2ofF1cg": 48.94}
opens, highs, lows, closes, years = load_ohlc(SYMBOL_FILES["BRN"])
dir_flips = []
tape = compute_organic_signals(opens, highs, lows, closes, dir_flips=dir_flips)
rtape = pack_tape(tape, dir_flips=dir_flips)
ok = True
for name, reg in REG.items():
    res = nr.run_organic_rust(rtape, name, floor_ladder=LADDER_SEG,
                              stop_mode="none", diag=False)
    m = extended_metrics(_trades_from_rust(res["trades"]), years)
    comp = round(m["total_compound"], 2)
    status = "PASS" if abs(comp - reg) < 0.005 else "FAIL"
    ok = ok and status == "PASS"
    print(f"[{name:9s}] 本次 {comp:+.2f} vs P1在册 {reg:+.2f} → {status}", flush=True)
sys.exit(0 if ok else 1)
