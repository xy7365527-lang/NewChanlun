"""磁带二进制 dump — 供纯 Rust cargo test（trade_behavior）加载。

职责边界（任务 2026-06-11 交易行为分解）：信号事件全部来自 Rust 增量接口
（organic_signals 是 driver），本脚本只做列式打包落盘——与 pack_tape 同口径，
目标从 PyO3 OrganicTape 换成文件。交易层回测在 cargo test 内纯 Rust 执行，
不走 Python→PyO3 链路。

输出（analysis/data_cache/）：
  _tape_v2r_{SYM}.bin  — 磁带二进制（格式见下，小端）
  _tape_v2r_{SYM}_ts.i64 — 清洗后逐 bar epoch 秒（naive，数据文件原生时区）

二进制格式（小端，与 rust/src/trade_behavior.rs load_tape 逐字段对齐）：
  header: magic u64=0x4E43545056325200 ("NCTPV2R\0")
          n_bars u64, n_bsp u64, n_div u64, n_flip u64
  closes: n*f64
  buy1, sell1, sell_any, buy_any, up_settled: 各 n*u16
  max_ladder: n*u8
  type2: n*u8
  bsp 行: bar i64, lad u8, kind u8(1/2/3), side u8(0=buy,1=sell),
          confirmed u8, has_cs u8, seg_idx i64, cs i64(无效=0), zd f64, zg f64,
          price f64（has_cs=0 ⇒ cs/zd/zg 写 0/NaN/NaN，Rust 侧还原 None）
  div 行: bar i64, lad u8, kind u8(0=trend,1=consolidation), dir u8(0=up,1=down),
          seg_idx i64, fa f64, fc f64, price f64
  flip 行: bar i64, lad u8, dir u8(0=up,1=down)

用法：PYTHONPATH=src .venv/bin/python analysis/_dump_tape_rust.py OKLO
"""

from __future__ import annotations

import json
import math
import struct
import sys
import time
from datetime import datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

from fugue_v2_full_backtest import SYMBOL_FILES  # noqa: E402
from fugue_version_i import MAX_LADDER  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
MAGIC = 0x4E43545056325200

KIND_CODE = {"type1": 1, "type2": 2, "type3": 3}
SIDE_CODE = {"buy": 0, "sell": 1}
DIV_KIND_CODE = {"trend": 0, "consolidation": 1}
DIR_CODE = {"up": 0, "down": 1}


def load_ohlc_with_ts(path: Path):
    """load_ohlc 的清洗逻辑逐字复刻 + 保留逐 bar 时间戳（epoch 秒，naive）。"""
    raw = json.loads(path.read_text())
    if "bars" in raw:
        bars = raw["bars"]
        o_in = [b["open"] for b in bars]
        h_in = [b["high"] for b in bars]
        l_in = [b["low"] for b in bars]
        c_in = [b["close"] for b in bars]
        d_in = [b["ts"] for b in bars]
    else:
        o_in, h_in, l_in, c_in = raw["opens"], raw["highs"], raw["lows"], raw["closes"]
        d_in = raw.get("dates")
    opens, highs, lows, closes, ts = [], [], [], [], []
    for idx in range(len(c_in)):
        o, h, l, c = float(o_in[idx]), float(h_in[idx]), float(l_in[idx]), float(c_in[idx])
        if math.isnan(o) or math.isnan(h) or math.isnan(l) or math.isnan(c):
            continue
        if o <= 0 or h <= 0 or l <= 0 or c <= 0:
            continue
        opens.append(o)
        highs.append(h)
        lows.append(l)
        closes.append(c)
        if d_in is not None:
            s = str(d_in[idx])
            dt = datetime.fromisoformat(s)
            if dt.tzinfo is not None:  # BRN dates 带 +00:00 → 取 UTC naive
                dt = dt.replace(tzinfo=None)
            ts.append(int(dt.timestamp()))
    return opens, highs, lows, closes, ts


def dump(sym: str) -> None:
    opens, highs, lows, closes, ts = load_ohlc_with_ts(SYMBOL_FILES[sym])
    n = len(closes)
    assert len(ts) == n, f"ts 与 closes 长度不一致：{len(ts)} vs {n}"
    print(f"[{sym}] bars={n:,}，信号层开始", flush=True)
    t0 = time.time()
    dir_flips: list = []
    tape = compute_organic_signals(opens, highs, lows, closes, dir_flips=dir_flips)
    print(f"[{sym}] 信号层 {time.time() - t0:.1f}s", flush=True)

    # 磁带指纹（与 rev_v2_paired_backtest 同口径——Rust 侧硬校验用）
    fp_bsp = sum(len(s.bsp_events[l]) for s in tape if s.bsp_events
                 for l in range(MAX_LADDER))
    fp_div = sum(len(s.div_events[l]) for s in tape if s.div_events
                 for l in range(MAX_LADDER))
    print(f"[{sym}] tape_fp: bsp={fp_bsp} div={fp_div} flips={len(dir_flips)}",
          flush=True)

    def mask(rows) -> int:
        m = 0
        for k, v in enumerate(rows):
            if v:
                m |= 1 << k
        return m

    bsp_rows, div_rows = [], []
    for i, s in enumerate(tape):
        if s.bsp_events:
            for lad in range(MAX_LADDER):
                for e in s.bsp_events[lad]:
                    # (kind, side, seg_idx, confirmed, cs, zd, zg, price)
                    cs = e[4]
                    bsp_rows.append((i, lad, KIND_CODE[e[0]], SIDE_CODE[e[1]],
                                     1 if e[3] else 0, 1 if cs is not None else 0,
                                     e[2], cs if cs is not None else 0,
                                     e[5] if e[5] is not None else float("nan"),
                                     e[6] if e[6] is not None else float("nan"),
                                     e[7]))
        if s.div_events:
            for lad in range(MAX_LADDER):
                for d in s.div_events[lad]:
                    # (kind, direction, side, seg_idx, fa, fc, price)
                    div_rows.append((i, lad, DIV_KIND_CODE[d[0]], DIR_CODE[d[1]],
                                     d[3], d[4], d[5], d[6]))

    out = DATA_DIR / f"_tape_v2r_{sym}.bin"
    with out.open("wb") as f:
        f.write(struct.pack("<QQQQQ", MAGIC, n, len(bsp_rows), len(div_rows),
                            len(dir_flips)))
        f.write(struct.pack(f"<{n}d", *(s.close for s in tape)))
        for attr in ("buy1", "sell1", "sell_any", "buy_any"):
            f.write(struct.pack(f"<{n}H", *(mask(getattr(s, attr)) for s in tape)))
        f.write(struct.pack(
            f"<{n}H", *((mask(s.up_move_settled) if s.up_move_settled else 0)
                        for s in tape)))
        f.write(struct.pack(f"<{n}B", *(s.max_ladder for s in tape)))
        f.write(struct.pack(f"<{n}B", *(1 if s.type2_buy else 0 for s in tape)))
        for r in bsp_rows:
            f.write(struct.pack("<qBBBBBqqddd", r[0], r[1], r[2], r[3], r[4],
                                r[5], r[6], r[7], r[8], r[9], r[10]))
        for r in div_rows:
            f.write(struct.pack("<qBBBqddd", *r))
        for bar, lad, d in dir_flips:
            f.write(struct.pack("<qBB", bar, lad, DIR_CODE[d]))
    (DATA_DIR / f"_tape_v2r_{sym}_ts.i64").write_bytes(
        struct.pack(f"<{n}q", *ts))
    print(f"[{sym}] 落盘 {out.name}（{out.stat().st_size / 1e6:.1f}MB）"
          f" + ts.i64", flush=True)


if __name__ == "__main__":
    for sym in sys.argv[1:]:
        dump(sym.upper())
