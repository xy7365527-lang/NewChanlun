"""统一必然性引擎（unn）× NautilusTrader **真流式** 回测集成。

═══════════════ 与 backtest_unn.py 的范畴差（535号边界B）═══════════════

`backtest_unn.py` = 适配器模式（NT 当数据总线，on_bar 缓冲 OHLC，unn 在 on_stop
**批量** 跑）——诚实声明它「不是真正的流式」。

本模块 = **真流式**：NautilusTrader BacktestEngine 逐 bar 回放，on_bar 每收到一根
新 bar →
  ① `StreamingSignalReader.process_bar`（缠论增量 RecursiveOrchestrator + 信号读出，
     逐 bar 产出 BarSignalI + 本 bar dir_flips）——信号层逐 bar 流式；
  ② `UnnStream.push_bar`（unn 引擎逐 bar 消费 BarSig，返回本 bar 交易信号）——
     引擎层逐 bar 流式。
on_stop → `UnnStream.finish()`（eod cascade 关根 + N4 反证）→ 完整结果。

两层都是逐 bar 步进，与各自的批量路径 **共享同一段循环体代码**（信号层
StreamingSignalReader.process_bar；引擎层 UnnStreamCore::step）⇒ bit-exact 由构造保证。

为何不改 production ChanlunStrategy（任务字面）：它走独立的 ChanlunBridge 信号路径
（阶段1 信号贯通，PositionalStream 接入尚是 TODO），与 organic_signals/unn 路径是
两套信号架构。把 unn 塞进去 = 混淆两套架构（no-patch.md）。unn 的定义域是
organic_signals 磁带 ⇒ 专用流式壳是范畴正确的形态。

═══════════════ 验证判据 ═══════════════

bit-exact（L1 管线一致性，formalization-validity-domain.md）：NT 流式路径
`finish()` 结果与同数据批量 `run_positional_rust(mode="unn")` 逐键 == ——证明
NautilusTrader 逐 bar 驱动的流式引擎与批量引擎产出逐位相同的结果。

N1-N8（必然性，L2）：流式 step 内 8 个 prove 每 bar 检查（violation=panic）——
NT 流式跑通无 panic ⇒ N1-N8 在逐 bar 驱动下同样成立。

用法（仓库根目录）：
    PYTHONPATH=src .venv/bin/python trading_system/backtest_unn_stream.py            # CL 全量
    PYTHONPATH=src .venv/bin/python trading_system/backtest_unn_stream.py --symbol CL --bars 200000
输出：trading_system/data_cache/unn_stream_<SYM>.json
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO_ROOT))
sys.path.insert(0, str(REPO_ROOT / "src"))
sys.path.insert(0, str(REPO_ROOT / "analysis"))

from nautilus_trader.backtest.engine import BacktestEngine  # noqa: E402
from nautilus_trader.backtest.config import BacktestEngineConfig  # noqa: E402
from nautilus_trader.backtest.models import FillModel  # noqa: E402
from nautilus_trader.config import LoggingConfig, StrategyConfig  # noqa: E402
from nautilus_trader.model.currencies import USD  # noqa: E402
from nautilus_trader.model.data import BarType  # noqa: E402
from nautilus_trader.model.enums import AccountType, OmsType  # noqa: E402
from nautilus_trader.model.identifiers import InstrumentId, TraderId  # noqa: E402
from nautilus_trader.model.objects import Money  # noqa: E402
from nautilus_trader.trading.strategy import Strategy  # noqa: E402

import newchan_rust as nr  # noqa: E402

from nested_recursive_fugue_final_backtest import FLOOR, analyze, bh_mdd  # noqa: E402
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import StreamingSignalReader, compute_organic_signals, push_signal  # noqa: E402

from trading_system.backtest_unn import load_bars  # noqa: E402（复用 Databento→Bar 加载）
from trading_system.config.instruments import INSTRUMENTS, make_instrument  # noqa: E402

OUT_DIR = REPO_ROOT / "trading_system" / "data_cache"


# ════════════════ NautilusTrader Strategy（真流式：on_bar → process_bar → push_bar）════════════════

class UnnStreamStrategyConfig(StrategyConfig, frozen=True):
    instrument_id: InstrumentId
    bar_type: BarType


class UnnStreamStrategy(Strategy):
    """unn 真流式策略壳。

    on_bar 逐 bar：缠论增量信号读出（StreamingSignalReader）→ unn 引擎推进（UnnStream）。
    结果挂 self.result（runner 读取），P&L 来自 unn 自身账本（与批量同账本模型）。
    LMT-only / 不下单：unn 的几何限价撮合已固化在 Rust 引擎内，本壳不向 NT 下单。
    """

    def __init__(self, config: UnnStreamStrategyConfig) -> None:
        super().__init__(config)
        self.reader: StreamingSignalReader | None = None
        self.stream: "nr.UnnStream | None" = None
        self.dir_flips: list = []          # StreamingSignalReader 收集器（逐 bar append）
        self._bar_i = 0
        self._new_trades: list = []        # push_bar 逐 bar 吐单累积
        self.result: dict | None = None
        self.trades: list = []             # finish() 完整 trade 行（11 元组逐字落盘）
        self.equity: list = []             # finish() equity 序列（采样 NAV，曲线拐点用）
        # 缓冲 OHLC：仅供 on_stop 后的批量 bit-exact 对账（不参与流式计算）。
        self._o: list[float] = []
        self._h: list[float] = []
        self._l: list[float] = []
        self._c: list[float] = []

    def on_start(self) -> None:
        # require_settled=True：与 analysis/批量基线逐字一致（backtest_unn.run_unn 同口径）。
        self.reader = StreamingSignalReader(dir_flips=self.dir_flips, require_settled=True)
        self.stream = nr.UnnStream(floor_ladder=FLOOR)
        self.subscribe_bars(self.config.bar_type)
        self.log.info(f"UnnStreamStrategy 启动: {self.config.bar_type}（逐 bar 流式 process_bar→push_bar）")

    def on_bar(self, bar) -> None:
        assert self.reader is not None and self.stream is not None
        o = bar.open.as_double(); h = bar.high.as_double()
        low = bar.low.as_double(); c = bar.close.as_double()
        self._o.append(o); self._h.append(h); self._l.append(low); self._c.append(c)
        i = self._bar_i
        # ① 信号层逐 bar：缠论增量 + 信号读出（本 bar BarSignalI；新 dir_flips 已 append）。
        n_before = len(self.dir_flips)
        sig = self.reader.process_bar(i, o, h, low, c)
        # 本 bar 新增 dir_flips（process_bar append 的全部 bar==i）→ (ladder, dir) 行。
        flip_rows = [(lad, d) for (_b, lad, d) in self.dir_flips[n_before:]]
        # ② 引擎层逐 bar：unn 消费 BarSig（prove N1-N8 每 bar 检查，violation=panic）。
        new = push_signal(self.stream, sig, flip_rows)
        self._new_trades.extend(new)
        self._bar_i += 1

    def on_stop(self) -> None:
        if self.stream is None or self._bar_i == 0:
            self.log.error("无 bar 流式推进，跳过 finish")
            return
        res = self.stream.finish()
        a = analyze(res, self._c, years=None)
        a["nrf_max_children"] = res.get("nrf_max_children", 0)
        a["_n_pushed_trades"] = len(self._new_trades)
        self.result = a
        # 逐笔明细落盘（11 元组逐字，无虚构）+ equity 序列（曲线拐点分析用）。
        # root/child 不由 trade 行携带——operational 角色由 (polarity, exit_reason)
        # 刻画（recover/cascade=子 voice；eod/flip_*=根；liq=两者皆可，市场强平）。
        self.trades = res["trades"]
        self.equity = res["equity"]
        self.log.info(f"on_stop: 流式 finish（{self._bar_i:,} bar，{len(res['trades'])} 笔）")


# ════════════════ 批量基线（同数据，bit-exact 对账）════════════════

def run_unn_batch(opens: list, highs: list, lows: list, closes: list) -> dict:
    """compute_organic_signals → pack_tape → run_positional_rust(unn)（批量基线）。"""
    dir_flips: list = []
    tape = compute_organic_signals(opens, highs, lows, closes,
                                   dir_flips=dir_flips, require_settled=True)
    rtape = pack_tape(tape, dir_flips=dir_flips)
    res = nr.run_positional_rust(rtape, floor_ladder=FLOOR, mode="unn")
    a = analyze(res, closes, years=None)
    a["nrf_max_children"] = res.get("nrf_max_children", 0)
    return a


def verify_bit_exact(stream_result: dict, batch_result: dict) -> dict:
    """NT 流式 analyze 结果 vs 批量 analyze 结果逐键 ==（_ 前缀诊断键除外）。"""
    keys = (set(stream_result) | set(batch_result)) - {"_timing", "_n_pushed_trades"}
    mism = {}
    for k in sorted(keys):
        sv = stream_result.get(k, "<MISSING>")
        bv = batch_result.get(k, "<MISSING>")
        if sv != bv:
            mism[k] = {"stream": sv, "batch": bv}
    return {"status": "PASS" if not mism else "MISMATCH", "n_keys": len(keys), "mismatches": mism}


# ════════════════ runner ════════════════

def build_engine(spec, log_level: str):
    instrument = make_instrument(spec)
    engine = BacktestEngine(
        config=BacktestEngineConfig(
            trader_id=TraderId("UNN-STREAM-001"),
            logging=LoggingConfig(log_level=log_level),
        ),
    )
    engine.add_venue(
        venue=instrument.id.venue,
        oms_type=OmsType.NETTING,
        account_type=AccountType.MARGIN,
        base_currency=USD,
        starting_balances=[Money(1_000_000.0, USD)],
        fill_model=FillModel(prob_fill_on_limit=0.0, prob_slippage=0.0),
    )
    engine.add_instrument(instrument)
    bar_type = BarType.from_str(f"{instrument.id}-1-MINUTE-LAST-EXTERNAL")
    engine.add_strategy(UnnStreamStrategy(
        config=UnnStreamStrategyConfig(instrument_id=instrument.id, bar_type=bar_type),
    ))
    return engine, instrument, bar_type


def main() -> int:
    parser = argparse.ArgumentParser(description="unn × NautilusTrader 真流式回测")
    parser.add_argument("--symbol", default="CL")
    parser.add_argument("--bars", type=int, default=0, help="最大 bar 数（0=全量）")
    parser.add_argument("--log-level", default="ERROR")
    args = parser.parse_args()

    sym = args.symbol
    if sym not in INSTRUMENTS:
        print(f"标的 {sym} 未注册（可用: {list(INSTRUMENTS)}）", file=sys.stderr)
        return 1
    spec = INSTRUMENTS[sym]
    max_bars = args.bars or None
    OUT_DIR.mkdir(parents=True, exist_ok=True)

    engine, instrument, bar_type = build_engine(spec, args.log_level)

    print(f"加载 {sym}（max_bars={args.bars or '全量'}）...")
    t0 = time.time()
    bars, opens, highs, lows, closes = load_bars(
        sym, bar_type, spec.price_precision, max_bars, instrument.size_precision)
    bh_pct = (closes[-1] / closes[0] - 1) * 100
    bh_dd = bh_mdd(closes) * 100
    print(f"  {len(bars):,} Bar  {time.time() - t0:.1f}s  BH={bh_pct:+.1f}% BH_MDD={bh_dd:.1f}%")

    # ── NT 真流式回放（on_bar 逐 bar process_bar→push_bar）──
    engine.add_data(bars)
    print("NautilusTrader 真流式回放（逐 bar process_bar → push_bar → finish）...")
    t0 = time.time()
    engine.run()
    print(f"  流式回放完成 {time.time() - t0:.1f}s")
    strat = engine.trader.strategies()[0]
    stream_result = strat.result
    if stream_result is None:
        print("流式未产出结果", file=sys.stderr)
        engine.dispose()
        return 1

    # ── 批量基线（同数据）──
    print("批量基线（compute_organic_signals → run_positional_rust(unn)）...")
    t0 = time.time()
    batch_result = run_unn_batch(opens, highs, lows, closes)
    print(f"  批量完成 {time.time() - t0:.1f}s")

    verify = verify_bit_exact(stream_result, batch_result)

    print("\n========== unn × NautilusTrader 真流式回测 ==========")
    print(f"标的          : {sym}（{instrument.id}）")
    print(f"bars          : {len(closes):,}")
    print(f"strat_pct     : {stream_result['strat_pct']:+.1f}%   (BH {bh_pct:+.1f}%, "
          f"P1={'PASS' if stream_result['strat_pct'] >= bh_pct else 'fail'})")
    print(f"max drawdown  : {stream_result['mdd_pct']:.1f}%   (BH_MDD {bh_dd:.1f}%)")
    print(f"n_trades      : {stream_result['n_trades']}  (push_bar 吐 {stream_result.get('_n_pushed_trades', '?')} 笔)")
    print(f"N1 max_kids   : {stream_result.get('nrf_max_children', '?')}")
    print(f"bit-exact     : {verify['status']}  ({verify['n_keys']} 键)  "
          f"{verify['mismatches'] if verify['mismatches'] else ''}")

    out = {
        "symbol": sym, "instrument_id": str(instrument.id), "n_bars": len(closes),
        "bh_pct": round(bh_pct, 1), "bh_mdd_pct": round(bh_dd, 1),
        "max_bars": args.bars or None,
        "unn_stream": stream_result,
        "P1_ge_bh": stream_result["strat_pct"] >= bh_pct,
        "bit_exact_vs_batch": verify,
        "architecture": "NautilusTrader BacktestEngine 逐bar回放 → on_bar(StreamingSignalReader.process_bar"
                        " → UnnStream.push_bar) → on_stop(finish); 信号层+引擎层两侧流式; bit-exact vs 批量",
        # ── 逐笔明细（11 元组逐字落盘，无虚构）──
        # trade_schema 声明每个 trades 行的字段顺序与语义。root/child 不在行内：
        # operational 角色由 (polarity, exit_reason) 推导——recover/cascade=子 voice；
        # eod/flip_short/flip_long=根；liq=root|child 皆可（市场强平遍历所有活跃空头）。
        # N1 守卫保证任一时刻至多一个 active root。长腿 root-vs-long-child 在行内不可分。
        "trade_schema": ["ladder", "entry_bar", "entry_price", "exit_bar",
                         "exit_price", "shares", "weight_at_entry", "deferred_bars",
                         "partial", "exit_reason", "polarity"],
        "trades": strat.trades,
        "equity": strat.equity,   # [(bar, nav)] 采样序列（EQUITY_SAMPLE_BARS≈日采样）
        "closes_first": closes[0], "closes_last": closes[-1],
    }
    (OUT_DIR / f"unn_stream_{sym}.json").write_text(json.dumps(out, ensure_ascii=False, indent=1))
    print(f"\n结果写入 {OUT_DIR / f'unn_stream_{sym}.json'}")

    engine.reset()
    engine.dispose()

    if verify["status"] == "MISMATCH":
        print("\n⚠️ bit-exact 失败：NT 流式与批量不一致——须诊断（no-workaround）", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
