"""unn 流式引擎 bit-exact 验证：UnnStream 逐 bar push == run_positional_rust 批量。

═══════════════ 535号边界B（编排者裁决 2026-06-14）═══════════════

unn 引擎的定义域是 SignalTape（BarSig 序列），不是原始 OHLC。流式 `UnnStream`
接收逐 bar 的 BarSig 字段（push_bar 收 BarSig 而非 OHLC——上游缠论信号层
`compute_organic_signals` 本就逐 bar 流式）。

流式 `UnnStreamCore::step/finish` 与批量 `run_unified_necessity` **共享同一段循环体
代码** ⇒ bit-exact 是构造性保证（非两份代码对齐）。本脚本跨 **marshal 边界** 验证：
  - 批量路径：BarSignalI 列表 → pack_tape 列式 marshal → run_positional_rust(unn)
  - 流式路径：逐 BarSignalI 构造单 bar 字段 → UnnStream.push_bar → finish

两条独立 marshal 路径产出逐位等价的结果（trades / equity / final_nav / 全 counters）
⇒ 流式接口正确。认识论等级 L1（管线一致性，formalization-validity-domain.md——
验证两条 marshal 路径产同样的数，不是新假设的经验验证）。

8 条必然性 prove（N1-N8）在流式 step 内同样每 bar 检查（violation=panic）——
流式跑通无 panic ⇒ N1-N8 在流式驱动下同样成立（L2）。

用法（仓库根目录）：
    PYTHONPATH=src .venv/bin/python analysis/verify_unn_stream.py            # CL 全量
    PYTHONPATH=src .venv/bin/python analysis/verify_unn_stream.py CL 200000  # 子集烟测
    PYTHONPATH=src .venv/bin/python analysis/verify_unn_stream.py ES,CL      # 多标的
"""

from __future__ import annotations

import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as nr  # noqa: E402

from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from nested_recursive_fugue_final_backtest import FLOOR  # noqa: E402
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals, push_signal  # noqa: E402


def compare(sym: str, batch: dict, stream: dict, stream_new_trades: list, n_bars: int) -> bool:
    """逐键 bit-exact 比对 + push_bar 吐单正确性（前缀 + finish 清算全在收盘 bar）。"""
    fails: list = []
    keys = set(batch) | set(stream)
    for k in sorted(keys):
        bv = batch.get(k, "<MISSING>")
        rv = stream.get(k, "<MISSING>")
        if bv != rv:
            fails.append(k)
    # push_bar 逐 bar 吐出的 = step 内交易；完整 trades（含 finish 清算）已在 dict
    # "trades" 键比对中验证 == 批量。此处验证流式吐单是完整表的前缀，且差额笔
    # （finish 阶段 eod 关根 + cascade 子树）全部 exit_bar == 末根 bar——本质不变量：
    # finish 清算不属任何 bar、全在收盘；若流式漏吐某 step 内交易，该笔 exit_bar
    # < 末根会落入差额段被此断言抓住（reason 维度 eod/cascade 是噪声，exit_bar 才本质）。
    full = [tuple(t) for t in stream["trades"]]
    pushed = [tuple(t) for t in stream_new_trades]
    if pushed != full[:len(pushed)]:
        fails.append("push_bar吐单非trades前缀")
    last_bar = n_bars - 1
    tail = full[len(pushed):]  # exit_bar 在元组 index 3
    bad = [t[3] for t in tail if t[3] != last_bar]
    if bad:
        fails.append(f"push_bar漏吐bar内交易(exit_bar≠{last_bar}):{bad[:5]}")

    if fails:
        print(f"  ❌ {sym} MISMATCH（{len(fails)} 项）：{fails[:12]}", flush=True)
        for k in fails[:6]:
            if k in batch and k in stream:
                print(f"    {k}:\n      batch ={batch[k]}\n      stream={stream[k]}", flush=True)
        return False
    print(f"  ✅ {sym} bit-exact：{len(keys)} 键 + equity + final_nav + "
          f"push_bar 吐 {len(pushed)} 笔(前缀) + finish 清算 {len(tail)} 笔(全收盘) ==", flush=True)
    return True


def verify(sym: str, max_bars: int | None) -> bool:
    print(f"\n{'=' * 64}\n  {sym} — unn 流式 ≡ 批量 bit-exact 验证\n{'=' * 64}", flush=True)
    opens, highs, lows, closes, _years = load_ohlc(SYMBOL_FILES[sym])
    if max_bars is not None:
        opens, highs, lows, closes = (a[:max_bars] for a in (opens, highs, lows, closes))

    # 信号层（批量与流式共用同一磁带——隔离验证 unn 引擎层流式正确性，非信号层）。
    dir_flips: list = []
    t0 = time.time()
    tape = compute_organic_signals(opens, highs, lows, closes,
                                   dir_flips=dir_flips, require_settled=True)
    print(f"  信号层 {time.time() - t0:.1f}s（{len(tape):,} bars，{len(dir_flips):,} flips）", flush=True)

    # ── 批量路径 ──
    rtape = pack_tape(tape, dir_flips=dir_flips)
    t0 = time.time()
    batch = nr.run_positional_rust(rtape, floor_ladder=FLOOR, mode="unn")
    print(f"  批量引擎 {time.time() - t0:.2f}s（{len(batch['trades'])} 笔）", flush=True)

    # ── 流式路径（逐 BarSignalI push，跨 marshal 边界）──
    flips_by_bar: dict = {}
    for (bar, lad, d) in dir_flips:
        flips_by_bar.setdefault(bar, []).append((lad, d))
    t0 = time.time()
    stream = nr.UnnStream(floor_ladder=FLOOR)
    new_trades: list = []
    for i, s in enumerate(tape):
        new_trades.extend(push_signal(stream, s, flips_by_bar.get(i, [])))
    res = stream.finish()
    print(f"  流式引擎 {time.time() - t0:.2f}s（{len(new_trades)} 笔，"
          f"snapshot={stream.snapshot()}）", flush=True)

    return compare(sym, batch, res, new_trades, len(tape))


def main() -> int:
    args = sys.argv[1:]
    syms = args[0].split(",") if args else ["CL"]
    max_bars = int(args[1]) if len(args) > 1 and args[1] else None
    ok = True
    for sym in syms:
        sym = sym.strip().upper()
        if sym not in SYMBOL_FILES:
            print(f"标的 {sym} 无数据文件映射，跳过", file=sys.stderr)
            ok = False
            continue
        ok = verify(sym, max_bars) and ok
    print(f"\n{'=' * 64}\n  {'✅ 全部 bit-exact' if ok else '❌ 存在 MISMATCH'}\n{'=' * 64}", flush=True)
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
