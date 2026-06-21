"""赋格引擎 v3（三轴分离 D∞ word 处理器）× NautilusTrader **真流式** 回测集成。

═══════════════ 存在论位置 ═══════════════

fugue_v3 = 从头写的**层结构操作引擎**（reinterp §3.2 / 定理 RF；三轴分离 D∞ word）：
  - 核心仓 H⁰（2/3 恒持，σ-不变 Casimir，只在顶背驰 C 整体清）
  - 机动仓 H¹（1/3=f=1/λ，四步 1-cycle 穿 ε=−1：平多→开空→平空→做多）
信号层**复用** spiral::signal（向心 confirm + nf_sell/nf_buy；enable_macd_divergence=True
已开 ⟹ type1 背驰按 MACD 面积判定 ⟹ 单边上扬无背驰 ⟹ 机动仓自然休眠 = 结构性 regime 门）。

═══════════════ 与纯 Python 流式版的范畴差（任务字面：NT 流式）═══════════════

旧 `backtest_fugue_v3.py` = 纯 Python `for i in range(n)` 循环（数据源 `fugue_v2_full_backtest
.load_ohlc`，可截断 60K bar）——与 `unn_stream` 数据源/数据量都不可比。

本模块 = **真流式 + 与 unn 同源**：NautilusTrader BacktestEngine 逐 bar 回放，on_bar 每收到
一根新 bar →
  ① `StreamingSignalReader.process_bar`（缠论增量 RecursiveOrchestrator + 信号读出，逐 bar 产
     BarSignalI + 本 bar dir_flips）——信号层逐 bar 流式；
  ② `FugueV3Stream.push_bar`（fugue_v3 引擎逐 bar 消费 BarSig，prove 每 bar panic）——引擎层
     逐 bar 流式。
on_stop → `FugueV3Stream.finish()`（eod 清仓 + 拷信号层计数器）→ 完整结果。

数据加载走 `trading_system.backtest_unn.load_bars`（INSTRUMENTS Databento 全量）——与
`backtest_unn_stream.py` **逐字同源同量** ⟹ v3 vs unn 可严格对比（同数据集，唯一变量=引擎）。

═══════════════ 认识论等级（formalization-validity-domain）═══════════════

  - 四步 1-cycle 结构 / 守恒 / σ-配额：L0（prove 守卫每 bar panic，零 panic=验收）。
  - 机动仓激活（哪些声部发声）：L2 regime 依赖。
  - 本回测（机动仓 alpha 是否跑赢 BH）：**L3**（真实数据，可否证；正/负域诚实报告）。
  - bit-exact（流式 finish() == 批量 run_fugue_v3）：L1，信息增量为零，**由构造保证**（ffi.rs：
    批量+流式共享 `FugueEngineCore::step/finish`）。运行时对账仅作 spot-check（--verify-batch），
    默认关闭——核心验收是逐 bar prove 零 panic，非 L1 冗余对账。

用法（仓库根目录）：
    PYTHONPATH=src .venv/bin/python trading_system/backtest_fugue_v3.py --symbol OKLO
    PYTHONPATH=src .venv/bin/python trading_system/backtest_fugue_v3.py --symbol OKLO --verify-batch
输出：trading_system/data_cache/fugue_v3_<SYM>.json
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

from nested_recursive_fugue_final_backtest import FLOOR, LADDER_NAMES, bh_mdd  # noqa: E402
from organic_signals import StreamingSignalReader, push_signal  # noqa: E402

from trading_system.backtest_unn import load_bars  # noqa: E402（复用 Databento→Bar 加载，与 unn 同源）
from trading_system.config.instruments import INSTRUMENTS, make_instrument  # noqa: E402

OUT_DIR = REPO_ROOT / "trading_system" / "data_cache"


# ════════════════ fugue_v3 结果分析（层结构，非 voice forest）════════════════

def analyze_fugue(res: dict, closes) -> dict:
    """fugue_v3 核心指标（strat_pct/mdd/n_trades/exit_reasons/by_ladder）。

    复刻 `nested_recursive_fugue_final_backtest.analyze` 的 NAV/MDD/层分解逻辑，**去掉 nrf
    森林 counters 块**——fugue_v3 是层结构（H⁰核心⊕H¹机动，非 voice forest），不声明它不具备
    的 nrf 字段（no-patch-mentality：声明=能力）。物理 NAV 唯一真值 = equity 序列末值。
    """
    trades = res["trades"]
    strat_pct = (res["final_nav"] / 100_000.0 - 1) * 100
    equity = res["equity"]
    assert abs(equity[-1][1] - res["final_nav"]) < 1e-3, \
        f"equity 末值漂移：{equity[-1][1]} ≠ {res['final_nav']}"
    peak = mdd = 0.0
    for (_b, nav_i) in equity:
        peak = max(peak, nav_i)
        if peak > 0:
            mdd = min(mdd, nav_i / peak - 1.0)
    by_ladder: dict = {}
    reason_counts: dict[str, int] = {}
    n_long = n_short = 0
    for (lad, eb, ep, xb, xp, sh, w, dfr, part, reason, pol) in trades:
        reason_counts[reason] = reason_counts.get(reason, 0) + 1
        if pol == "long":
            n_long += 1
            pnl = sh * (xp - ep)
        else:
            n_short += 1
            pnl = sh * (ep - xp)
        d = by_ladder.setdefault((lad, pol), {"n": 0, "pnl_cash": 0.0, "wins": 0, "held": 0})
        d["n"] += 1
        d["pnl_cash"] += pnl
        d["wins"] += pnl > 0
        d["held"] += xb - eb
    for d in by_ladder.values():
        d["pnl_cash"] = round(d["pnl_cash"], 0)
        d["avg_held_bars"] = round(d["held"] / d["n"], 0) if d["n"] else 0
        del d["held"]
    return {
        "strat_pct": round(strat_pct, 1),
        "mdd_pct": round(mdd * 100, 1),
        "n_trades": len(trades),
        "n_long": n_long,
        "n_short": n_short,
        "exit_reasons": reason_counts,
        "by_ladder": {f"{LADDER_NAMES.get(k, str(k))}/{pol}": v
                      for (k, pol), v in sorted(by_ladder.items())},
    }


def _sig_to_bartuple(sig, flip_rows: list) -> tuple:
    """BarSignalI → run_fugue_v3 的 11-tuple（镜像 push_signal 的拆解，bit-exact 前提）。"""
    from organic_signals import MAX_LADDER, _ladder_mask
    bsp_rows: list = []
    if sig.bsp_events:
        for lad in range(MAX_LADDER):
            for e in sig.bsp_events[lad]:
                bsp_rows.append((lad, e[0], e[1], e[2], bool(e[3]), e[4], e[5], e[6], e[7]))
    div_rows: list = []
    if sig.div_events:
        for lad in range(MAX_LADDER):
            for d in sig.div_events[lad]:
                div_rows.append((lad, d[0], d[1], d[3], d[4], d[5], d[6]))
    up = _ladder_mask(sig.up_move_settled) if sig.up_move_settled else 0
    return (
        sig.close, _ladder_mask(sig.buy1), _ladder_mask(sig.sell1),
        _ladder_mask(sig.sell_any), _ladder_mask(sig.buy_any),
        up, sig.max_ladder, bool(sig.type2_buy), bsp_rows, div_rows, flip_rows,
    )


# ════════════════ NautilusTrader Strategy（真流式：on_bar → process_bar → push_bar）════════════════

class FugueV3StreamStrategyConfig(StrategyConfig, frozen=True):
    instrument_id: InstrumentId
    bar_type: BarType
    verify_batch: bool = False


class FugueV3StreamStrategy(Strategy):
    """fugue_v3 真流式策略壳（结构同 UnnStreamStrategy，引擎换 FugueV3Stream）。

    on_bar 逐 bar：缠论增量信号读出（StreamingSignalReader）→ fugue_v3 引擎推进（FugueV3Stream）。
    结果挂 self.result，P&L 来自引擎自身账本（equity 序列末值 = final_nav）。
    LMT-only / 不下单：fugue_v3 的几何限价撮合固化在 Rust 引擎内，本壳不向 NT 下单。
    """

    def __init__(self, config: FugueV3StreamStrategyConfig) -> None:
        super().__init__(config)
        self.reader: StreamingSignalReader | None = None
        self.stream: "nr.FugueV3Stream | None" = None
        self.dir_flips: list = []          # StreamingSignalReader 收集器（逐 bar append）
        self._bar_i = 0
        self._new_trades: list = []        # push_bar 逐 bar 吐单累积（诊断用）
        self.result: dict | None = None
        self.stream_res: dict | None = None  # finish() 完整 dict（fugue counters 取用）
        self.trades: list = []             # finish() 完整 trade 行（11 元组逐字落盘）
        self.equity: list = []             # finish() equity 序列（NAV 采样，曲线拐点用）
        self._bartuples: list = []         # 仅 verify_batch：收集 11-tuple 供 run_fugue_v3 对账

    def on_start(self) -> None:
        # require_settled=True：与 unn_stream/批量基线逐字一致。
        self.reader = StreamingSignalReader(dir_flips=self.dir_flips, require_settled=True)
        self.stream = nr.FugueV3Stream(floor_ladder=FLOOR)
        self.subscribe_bars(self.config.bar_type)
        self.log.info(f"FugueV3StreamStrategy 启动: {self.config.bar_type}（逐 bar process_bar→push_bar）")

    def on_bar(self, bar) -> None:
        assert self.reader is not None and self.stream is not None
        o = bar.open.as_double(); h = bar.high.as_double()
        low = bar.low.as_double(); c = bar.close.as_double()
        i = self._bar_i
        # ① 信号层逐 bar：缠论增量 + 信号读出（本 bar BarSignalI；新 dir_flips 已 append）。
        n_before = len(self.dir_flips)
        sig = self.reader.process_bar(i, o, h, low, c)
        # 本 bar 新增 dir_flips（process_bar append 的全部 bar==i）→ (ladder, dir) 行。
        flip_rows = [(lad, d) for (_b, lad, d) in self.dir_flips[n_before:]]
        if self.config.verify_batch:
            self._bartuples.append(_sig_to_bartuple(sig, flip_rows))
        # ② 引擎层逐 bar：fugue_v3 消费 BarSig（prove 每 bar 检查，violation=panic）。
        new = push_signal(self.stream, sig, flip_rows)
        self._new_trades.extend(new)
        self._bar_i += 1

    def on_stop(self) -> None:
        if self.stream is None or self._bar_i == 0:
            self.log.error("无 bar 流式推进，跳过 finish")
            return
        res = self.stream.finish()
        self.stream_res = res
        a = analyze_fugue(res, None)
        a["_n_pushed_trades"] = len(self._new_trades)
        self.result = a
        self.trades = res["trades"]
        self.equity = res["equity"]
        self.log.info(f"on_stop: 流式 finish（{self._bar_i:,} bar，{len(res['trades'])} 笔）")


# ════════════════ 批量基线（同 sig 序列，bit-exact spot-check）════════════════

def verify_bit_exact(stream_res: dict, bartuples: list, closes) -> dict:
    """流式 finish() 结果 vs 批量 run_fugue_v3(同 sig 序列) 逐键 ==（analyze_fugue 口径）。

    L1 spot-check：bartuples 由流式 reader 产出的同一 sig 序列拆解 ⟹ 信号层完全一致，
    本对账仅校验引擎层 step/finish 流式累积 == 批量。bit-exact 由构造保证（共享 step/finish），
    本函数仅作实证确认。
    """
    batch_res = nr.run_fugue_v3(bartuples, floor_ladder=FLOOR)
    sa = analyze_fugue(stream_res, closes)
    ba = analyze_fugue(batch_res, closes)
    keys = set(sa) | set(ba)
    mism = {k: {"stream": sa.get(k), "batch": ba.get(k)} for k in keys if sa.get(k) != ba.get(k)}
    return {"status": "PASS" if not mism else "MISMATCH", "n_keys": len(keys), "mismatches": mism}


# ════════════════ runner ════════════════

def build_engine(spec, log_level: str, verify_batch: bool):
    instrument = make_instrument(spec)
    engine = BacktestEngine(
        config=BacktestEngineConfig(
            trader_id=TraderId("FUGUEV3-STREAM-001"),
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
    engine.add_strategy(FugueV3StreamStrategy(
        config=FugueV3StreamStrategyConfig(
            instrument_id=instrument.id, bar_type=bar_type, verify_batch=verify_batch),
    ))
    return engine, instrument, bar_type


def main() -> int:
    parser = argparse.ArgumentParser(description="赋格引擎 v3 × NautilusTrader 真流式回测")
    parser.add_argument("--symbol", default="OKLO")
    parser.add_argument("--bars", type=int, default=0, help="最大 bar 数（0=全量）")
    parser.add_argument("--verify-batch", action="store_true",
                        help="同时跑批量 run_fugue_v3 校验 bit-exact（收集全部 BarTuple，耗内存，小标的用）")
    parser.add_argument("--log-level", default="ERROR")
    args = parser.parse_args()

    sym = args.symbol
    if sym not in INSTRUMENTS:
        print(f"标的 {sym} 未注册（可用: {list(INSTRUMENTS)}）", file=sys.stderr)
        return 1
    spec = INSTRUMENTS[sym]
    max_bars = args.bars or None
    OUT_DIR.mkdir(parents=True, exist_ok=True)

    engine, instrument, bar_type = build_engine(spec, args.log_level, args.verify_batch)

    print(f"加载 {sym}（max_bars={args.bars or '全量'}）...")
    t0 = time.time()
    bars, opens, highs, lows, closes = load_bars(
        sym, bar_type, spec.price_precision, max_bars, instrument.size_precision)
    bh_pct = (closes[-1] / closes[0] - 1) * 100
    bh_dd = bh_mdd(closes) * 100
    print(f"  {len(bars):,} Bar  {time.time() - t0:.1f}s  BH={bh_pct:+.1f}% BH_MDD={bh_dd:.1f}%")

    # ── NT 真流式回放（on_bar 逐 bar process_bar→push_bar）──
    engine.add_data(bars)
    print("NautilusTrader 真流式回放（逐 bar process_bar → push_bar → finish；prove 每 bar panic）...")
    t0 = time.time()
    engine.run()
    print(f"  流式回放完成 {time.time() - t0:.1f}s（零 panic ⟹ 守恒/四步闭合/穿 ε=−1 全程成立 L0/L2）")
    strat = engine.trader.strategies()[0]
    a = strat.result
    stream_res = strat.stream_res
    if a is None or stream_res is None:
        print("流式未产出结果", file=sys.stderr)
        engine.dispose()
        return 1

    p1 = a["strat_pct"] >= bh_pct

    # ── bit-exact spot-check（仅 --verify-batch）──
    bit_exact = None
    if args.verify_batch:
        print("批量基线（run_fugue_v3 同 sig 序列）bit-exact 对账...")
        t0 = time.time()
        bit_exact = verify_bit_exact(stream_res, strat._bartuples, closes)
        print(f"  对账完成 {time.time() - t0:.1f}s: {bit_exact['status']}")

    cyc_opens = sum(stream_res["n_cycle_opens_by_ladder"])
    cyc_closes = sum(stream_res["n_cycle_closes_by_ladder"])
    liqs = sum(stream_res["n_liquidations_by_ladder"])
    mob_pnl = sum(stream_res["mobile_realized_pnl_by_ladder"])

    print("\n========== 赋格引擎 v3（三轴分离 D∞ word）× NautilusTrader 真流式回测 ==========")
    print(f"标的          : {sym}（{instrument.id}）")
    print(f"bars          : {len(closes):,}")
    print(f"strat_pct     : {a['strat_pct']:+.1f}%   (BH {bh_pct:+.1f}%, "
          f"P1={'PASS ✓' if p1 else 'fail ✗'})   [L3 有效域读数]")
    print(f"max drawdown  : {a['mdd_pct']:.1f}%   (BH_MDD {bh_dd:.1f}%)")
    print(f"n_trades      : {a['n_trades']}  (多头 {a['n_long']} / 空头 {a['n_short']}; "
          f"push_bar 吐 {a.get('_n_pushed_trades', '?')} 笔)")
    print(f"exit_reasons  : {a.get('exit_reasons', {})}")
    print(f"四步循环      : opens={cyc_opens} closes={cyc_closes} 强平={liqs}  "
          f"机动仓已实现={mob_pnl:+.0f}")
    print(f"多声部并发    : max_concurrent_voices={stream_res['max_concurrent_voices']}  "
          f"Δr=−1 闭合={stream_res['cross_level_closures']}")
    print(f"核心仓        : 清仓={sum(stream_res['n_core_clears_by_ladder'])} "
          f"建仓={sum(stream_res['n_entries_by_ladder'])}")
    print(f"成本门拒绝    : cost={sum(stream_res['n_cost_rejects_by_ladder'])} "
          f"noref={sum(stream_res['n_noref_rejects_by_ladder'])}")
    if bit_exact:
        print(f"bit-exact     : {bit_exact['status']}  "
              f"{bit_exact['mismatches'] if bit_exact['mismatches'] else ''}")

    out = {
        "symbol": sym, "instrument_id": str(instrument.id), "n_bars": len(closes),
        "bh_pct": round(bh_pct, 1), "bh_mdd_pct": round(bh_dd, 1),
        "max_bars": args.bars or None,
        "epistemic_level": "L3（真实数据，可否证；正/负域诚实报告）",
        # ── 关键指标（平铺，汇总脚本直读）──
        "strat_pct": a["strat_pct"], "mdd_pct": a["mdd_pct"], "n_trades": a["n_trades"],
        "n_long": a["n_long"], "n_short": a["n_short"],
        "P1_ge_bh": p1,
        # ── prove 验收：能产出 JSON ⟹ 流式全程零 panic ⟹ 守恒/四步闭合/穿 ε=−1 成立（L0/L2）──
        "prove_panic": 0,
        "exit_reasons": a.get("exit_reasons", {}),
        "by_ladder": a.get("by_ladder", {}),
        "cycle_opens": cyc_opens, "cycle_closes": cyc_closes, "liquidations": liqs,
        "mobile_realized_pnl": round(mob_pnl, 0),
        # max_concurrent_voices = fugue_v3 层结构的 max_kids 对应量（非 voice-forest nrf_max_children）。
        "max_concurrent_voices": stream_res["max_concurrent_voices"],
        "cross_level_closures": stream_res["cross_level_closures"],
        "core_clears": sum(stream_res["n_core_clears_by_ladder"]),
        "entries": sum(stream_res["n_entries_by_ladder"]),
        "fire_sell_by_ladder": stream_res["n_fire_sell_by_ladder"],
        "fire_buy_by_ladder": stream_res["n_fire_buy_by_ladder"],
        "bit_exact_vs_batch": bit_exact,
        "architecture": "NautilusTrader BacktestEngine 逐bar回放 → on_bar(StreamingSignalReader"
                        ".process_bar → FugueV3Stream.push_bar) → on_stop(finish); 信号层+引擎层两侧"
                        "流式; 层结构四步循环 H⁰(2/3核心)⊕H¹(1/3机动,穿ε=−1); 复用 spiral 信号层; "
                        "数据与 unn_stream 同源(load_bars)",
        # ── 逐笔明细（11 元组逐字落盘，无虚构）──
        # operational 角色由 (polarity, exit_reason) 推导（recover/cascade=子 voice；core_clear/eod=核心）。
        "trade_schema": ["ladder", "entry_bar", "entry_price", "exit_bar",
                         "exit_price", "shares", "weight_at_entry", "deferred_bars",
                         "partial", "exit_reason", "polarity"],
        "trades": strat.trades,
        "equity": strat.equity,
        "closes_first": closes[0], "closes_last": closes[-1],
    }
    (OUT_DIR / f"fugue_v3_{sym}.json").write_text(json.dumps(out, ensure_ascii=False, indent=1))
    print(f"\n结果写入 {OUT_DIR / f'fugue_v3_{sym}.json'}")

    engine.reset()
    engine.dispose()

    if bit_exact and bit_exact["status"] == "MISMATCH":
        print("\n⚠️ bit-exact 失败：NT 流式与批量不一致——须诊断（no-workaround）", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
