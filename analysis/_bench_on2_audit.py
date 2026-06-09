"""O(N²) 审计基线/对比计时 + bit-exact 校验工具。

用法:
  BENCH_BARS=120000 .venv/bin/python analysis/_bench_on2_audit.py time   # 计时 E/I
  .venv/bin/python analysis/_bench_on2_audit.py dump <out.json>          # 落盘 E/I 信号磁带
  .venv/bin/python analysis/_bench_on2_audit.py diff <a.json> <b.json>   # 逐位比对两份磁带
"""
from __future__ import annotations

import json
import os
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

DATA = ROOT / "analysis" / "data_cache" / "oklo_1m_databento.json"


def load(n_max: int = 0):
    raw = json.loads(DATA.read_text())
    bars = raw["bars"]
    if n_max > 0:
        bars = bars[:n_max]
    o = [float(b["open"]) for b in bars]
    h = [float(b["high"]) for b in bars]
    l = [float(b["low"]) for b in bars]
    c = [float(b["close"]) for b in bars]
    return o, h, l, c


def _tape_e(sigs) -> list:
    """E 信号磁带 → 可比对的纯结构（仅取驱动交易的字段）。"""
    out = []
    for s in sigs:
        out.append((
            round(s.close, 10),
            bool(s.down_move_settled), bool(s.up_move_settled),
            bool(getattr(s, "l2_flip_short", False)),
            bool(getattr(s, "l2_flip_long", False)),
            bool(getattr(s, "entry_div_ok", False)),
            bool(getattr(s, "exit_div_ok", False)),
        ))
    return out


def _tape_i(sigs) -> list:
    out = []
    for s in sigs:
        out.append((
            round(s.close, 10),
            tuple(bool(x) for x in s.buy1),
            tuple(bool(x) for x in s.sell1),
            tuple(bool(x) for x in s.sell_any),
            tuple(bool(x) for x in s.buy_any),
            int(s.max_ladder),
            bool(s.type2_buy),
        ))
    return out


def cmd_time():
    n_max = int(os.environ.get("BENCH_BARS", "0"))
    o, h, l, c = load(n_max)
    n = len(c)
    import m1_e_rust_engine as RE
    from m1_i_rust_engine import compute_i_signals_rust

    t0 = time.time()
    e = RE.compute_e_signals_rust(o, h, l, c)
    te = time.time() - t0

    t0 = time.time()
    i = compute_i_signals_rust(o, h, l, c)
    ti = time.time() - t0

    print(f"bars={n:,}")
    print(f"E: {te:8.2f}s  ({n/te:,.0f} bar/s)")
    print(f"I: {ti:8.2f}s  ({n/ti:,.0f} bar/s)")
    print(f"len(e)={len(e)} len(i)={len(i)}")


def cmd_dump(outp: str):
    n_max = int(os.environ.get("BENCH_BARS", "0"))
    o, h, l, c = load(n_max)
    import m1_e_rust_engine as RE
    from m1_i_rust_engine import compute_i_signals_rust
    e = RE.compute_e_signals_rust(o, h, l, c)
    i = compute_i_signals_rust(o, h, l, c)
    Path(outp).write_text(json.dumps({"e": _tape_e(e), "i": _tape_i(i)}))
    print(f"dumped {len(e)} E + {len(i)} I → {outp}")


def cmd_diff(a: str, b: str):
    da = json.loads(Path(a).read_text())
    db = json.loads(Path(b).read_text())
    ok = True
    for key in ("e", "i"):
        ta = [tuple(_norm(x)) for x in da[key]]
        tb = [tuple(_norm(x)) for x in db[key]]
        if len(ta) != len(tb):
            print(f"[{key}] 长度发散 {len(ta)} vs {len(tb)}")
            ok = False
            continue
        diffs = [j for j in range(len(ta)) if ta[j] != tb[j]]
        if diffs:
            ok = False
            print(f"[{key}] {len(diffs)} bar 发散，首 5: {diffs[:5]}")
            j = diffs[0]
            print(f"   a[{j}]={ta[j]}")
            print(f"   b[{j}]={tb[j]}")
        else:
            print(f"[{key}] {len(ta):,} bar 逐位一致 ✓")
    print("BIT-EXACT PASS ✓" if ok else "DIVERGENCE ✗")
    sys.exit(0 if ok else 1)


def _norm(x):
    """JSON roundtrip 把 tuple 变 list，递归归一化为可比对结构。"""
    if isinstance(x, list):
        return tuple(_norm(y) for y in x)
    return x


if __name__ == "__main__":
    cmd = sys.argv[1] if len(sys.argv) > 1 else "time"
    if cmd == "time":
        cmd_time()
    elif cmd == "dump":
        cmd_dump(sys.argv[2])
    elif cmd == "diff":
        cmd_diff(sys.argv[2], sys.argv[3])
    else:
        print(f"unknown cmd: {cmd}")
        sys.exit(2)
