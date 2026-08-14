# FROZEN(#951)：绑 recursive_t.TFugueStream（push_bar -> Vec<Trade11> 逐笔 trade 流）；口径变更不同步；重跑用 git checkout impl/951-theta-pyo3-bridge
"""统一递归算子 T 流式赋格引擎 × NautilusTrader **真流式** 回测（8 标的 × 3 模式）。

═══════════════ 存在论位置 ═══════════════

T 算子 = standalone 递归引擎（第65课 aₙ=f(aₙ₋₁) 形式化）：从线段序列 a₀ 迭代到涌现上界 r*，
产全塔买卖点。本回测把 T 接入完整**递归嵌套多重赋格**仓位逻辑（`recursive_t::t_engine`，**不简化**
——对照 `recursive_t::backtest.rs` 的 apply_bsp 简化版只做多/减仓不回补/向下空仓）：

  四步跨级别循环 + 会计双重性 + 多空双向（复用 fugue_v3 验证过的会计/守恒原语）：
  1. 方向由涌现最高级别走势方向决定（root_direction）；
  2. 本级别买点 → 建仓（F，方向跟随涌现）；
  3. 次级别卖点 → 减仓做短差（E sink，H¹ 1/3 配额 σ⁻¹∘τ）；
  4. 次级别买点 → 回补（D recover，σ∘τ，σ-不变 1/3）；
  5. 本级别卖点 → 清仓（C 顶背驰整体清）；
  6. 多空双向（涌现向下 → 做空，全部 ε 镜像）。

═══════════════ 真流式（方案 B 精确重跑，全 Rust 内聚）═══════════════

与 unn/spiral/fugue_v3 的范畴差：那三个信号层在 Python（StreamingSignalReader→push_signal）。
T standalone ⇒ 信号层 + 仓位层**全 Rust**：`TFugueStream.push_bar(o,h,l,c)` 只传 OHLC。内部
`RecursiveOrchestrator` 逐 bar 产段 → 段确认时重跑 `iterate(a0, mode)` 全塔 → diff 新增 BSP →
当前 bar（确认时点）投放 → 喂完整仓位引擎。**比 batch backtest.rs 更严格**（后者 BSP 成交 @ 端点
raw_end = 残留 look-ahead；本流式 BSP 直到段确认 bar 才可见/投放 ⇒ 因果无 look-ahead）。

NautilusTrader BacktestEngine 逐 bar 回放 → on_bar(stream.push_bar) → on_stop(finish)。数据走
`load_bars`（与 unn/v3 同源 Databento JSON + load_ohlc 清洗）。

═══════════════ 三模式（步骤c 走势完美判定，受控实验）═══════════════

Structural（纯结构 5 条件）/ And（结构∧MACD 收紧）/ Or（结构∨MACD 放宽）——三路**同 a₀ 同操作层**，
唯一变量 = 步骤c 力度判据。收益差异**全部**归因于 MACD 收紧/放宽（formalization-validity-domain）。

═══════════════ 认识论等级 ═══════════════

  - 四步循环结构 / 会计双重性 / Σ|units| 守恒 / NAV 中性：L0（每 bar prove 守卫 panic，零 panic=验收）。
  - 哪条 word 此刻发声（BSP 是否 fire）：L2 regime 依赖。
  - 回测 alpha（跑赢 BH？）：**L3**（真实数据，可否证；正/负域诚实报告）。
  - bit-exact（流式 finish() == 批量 run_t_fugue）：L1，由构造保证（共享 TFugueStreamCore）。

用法（仓库根目录）：
    PYTHONPATH=src .venv/bin/python trading_system/backtest_t_fugue.py                    # 8 标的 × 3 模式
    PYTHONPATH=src .venv/bin/python trading_system/backtest_t_fugue.py --symbols OKLO,CL  # 指定标的
    PYTHONPATH=src .venv/bin/python trading_system/backtest_t_fugue.py --modes structural # 指定模式
输出：trading_system/data_cache/t_fugue_<SYM>_<MODE>.json + 汇总矩阵打印。
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

from trading_system.backtest_unn import load_bars  # noqa: E402（复用 Databento→Bar 加载，与 unn/v3 同源）
from trading_system.config.instruments import INSTRUMENTS, make_instrument  # noqa: E402

OUT_DIR = REPO_ROOT / "trading_system" / "data_cache"

INITIAL_CAPITAL = 100_000.0  # = recursive_t/fugue_v3 引擎 INITIAL_CAPITAL（NAV 基准）
MODES = ("structural", "and", "or")
DEFAULT_SYMBOLS = ("CL", "BRN", "DX", "GC", "ES", "QQQ", "BTC", "OKLO")


def bh_mdd(closes: list[float]) -> float:
    """buy-and-hold 最大回撤（正数比例）。"""
    peak = closes[0]
    mdd = 0.0
    for c in closes:
        peak = max(peak, c)
        if peak > 0:
            mdd = min(mdd, c / peak - 1.0)
    return -mdd


def analyze_t(res: dict) -> dict:
    """T 流式赋格结果 → 核心指标（strat_pct/mdd/by_ladder/exit_reasons）。

    物理 NAV 唯一真值 = equity 序列末值（= final_nav）。trade11 逐字消费，无虚构字段。
    operational 角色由 (polarity, exit_reason) 推导（trim/recover=子声部短差；core_clear/eod=核心仓）。
    """
    trades = res["trades"]
    strat_pct = (res["final_nav"] / INITIAL_CAPITAL - 1) * 100
    equity = res["equity"]
    peak = mdd = 0.0
    for (_b, nav_i) in equity:
        peak = max(peak, nav_i)
        if peak > 0:
            mdd = min(mdd, nav_i / peak - 1.0)
    by_ladder: dict = {}
    reason_counts: dict[str, int] = {}
    n_long = n_short = 0
    for (lad, eb, ep, xb, xp, sh, _w, _dfr, _part, reason, pol) in trades:
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
        "by_ladder": {f"L{lad}/{pol}": v for (lad, pol), v in sorted(by_ladder.items())},
    }


# ════════════════ NautilusTrader Strategy（真流式：on_bar → push_bar）════════════════


class TFugueStreamStrategyConfig(StrategyConfig, frozen=True):
    instrument_id: InstrumentId
    bar_type: BarType
    mode: str = "structural"


class TFugueStreamStrategy(Strategy):
    """T 流式赋格策略壳（结构同 FugueV3StreamStrategy，信号层全 Rust 内聚）。

    on_bar 逐 bar：`TFugueStream.push_bar(o,h,l,c)`（内部 orchestrator→重跑 T→完整仓位引擎，
    prove 守卫每 bar panic）。on_stop → `finish()`。P&L 来自引擎自身账本（equity 末值 = final_nav）。
    LMT-only / 不向 NT 下单：撮合固化在 Rust 引擎内（成交 = 确认时点 close）。
    """

    def __init__(self, config: TFugueStreamStrategyConfig) -> None:
        super().__init__(config)
        self.stream: "nr.TFugueStream | None" = None
        self._bar_i = 0
        self._new_trades: list = []
        self.result: dict | None = None
        self.stream_res: dict | None = None
        self.trades: list = []
        self.equity: list = []

    def on_start(self) -> None:
        self.stream = nr.TFugueStream(self.config.mode)
        self.subscribe_bars(self.config.bar_type)
        self.log.info(f"TFugueStreamStrategy 启动: {self.config.bar_type} mode={self.config.mode}（逐 bar push_bar）")

    def on_bar(self, bar) -> None:
        assert self.stream is not None
        o = bar.open.as_double()
        h = bar.high.as_double()
        low = bar.low.as_double()
        c = bar.close.as_double()
        new = self.stream.push_bar(o, h, low, c)
        self._new_trades.extend(new)
        self._bar_i += 1

    def on_stop(self) -> None:
        if self.stream is None or self._bar_i == 0:
            self.log.error("无 bar 流式推进，跳过 finish")
            return
        res = self.stream.finish()
        self.stream_res = res
        a = analyze_t(res)
        a["_n_pushed_trades"] = len(self._new_trades)
        self.result = a
        self.trades = res["trades"]
        self.equity = res["equity"]
        self.log.info(f"on_stop: 流式 finish（{self._bar_i:,} bar，{len(res['trades'])} 笔）")


# ════════════════ runner ════════════════


def build_engine(spec, mode: str, log_level: str):
    instrument = make_instrument(spec)
    engine = BacktestEngine(
        config=BacktestEngineConfig(
            trader_id=TraderId("TFUGUE-STREAM-001"),
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
    engine.add_strategy(TFugueStreamStrategy(
        config=TFugueStreamStrategyConfig(
            instrument_id=instrument.id, bar_type=bar_type, mode=mode),
    ))
    return engine, instrument, bar_type


def run_symbol(sym: str, modes: tuple[str, ...], max_bars: int | None, log_level: str) -> list[dict]:
    """单标的全模式 NT 真流式回测。bars 加载一次，3 模式各跑一个 BacktestEngine。"""
    spec = INSTRUMENTS[sym]
    rows: list[dict] = []

    # 用首个模式的 instrument 加载 bars（一次），三模式复用（同 bar 序列）。
    _e0, instrument0, bar_type0 = build_engine(spec, modes[0], log_level)
    _e0.dispose()
    print(f"加载 {sym}（max_bars={max_bars or '全量'}）...", flush=True)
    t0 = time.time()
    bars, opens, highs, lows, closes = load_bars(
        sym, bar_type0, spec.price_precision, max_bars, instrument0.size_precision)
    bh_pct = (closes[-1] / closes[0] - 1) * 100
    bh_dd = bh_mdd(closes) * 100
    print(f"  {len(bars):,} Bar  {time.time() - t0:.1f}s  BH={bh_pct:+.1f}% BH_MDD={bh_dd:.1f}%", flush=True)

    for mode in modes:
        engine, instrument, bar_type = build_engine(spec, mode, log_level)
        engine.add_data(bars)
        t0 = time.time()
        engine.run()
        strat = engine.trader.strategies()[0]
        a = strat.result
        stream_res = strat.stream_res
        if a is None or stream_res is None:
            print(f"  [{sym}/{mode}] 未产出结果", file=sys.stderr)
            engine.dispose()
            continue
        p1 = a["strat_pct"] >= bh_pct
        cyc_opens = sum(stream_res["n_cycle_opens_by_ladder"])
        cyc_closes = sum(stream_res["n_cycle_closes_by_ladder"])
        liqs = sum(stream_res["n_liquidations_by_ladder"])
        mob_pnl = sum(stream_res["mobile_realized_pnl_by_ladder"])
        clears = sum(stream_res["n_core_clears_by_ladder"])
        entries = sum(stream_res["n_entries_by_ladder"])
        print(
            f"  [{sym}/{mode:11s}] strat={a['strat_pct']:+8.1f}% (BH {bh_pct:+.1f}%, "
            f"P1={'PASS' if p1 else 'fail'}) mdd={a['mdd_pct']:.1f}% trades={a['n_trades']}"
            f"(L{a['n_long']}/S{a['n_short']}) entry={entries} sink={cyc_opens} rec={cyc_closes} "
            f"clear={clears} liq={liqs} mob={mob_pnl:+.0f} voices={stream_res['max_concurrent_voices']} "
            f"({time.time() - t0:.1f}s)",
            flush=True,
        )
        out = {
            "symbol": sym, "instrument_id": str(instrument.id), "mode": mode,
            "n_bars": len(closes), "bh_pct": round(bh_pct, 1), "bh_mdd_pct": round(bh_dd, 1),
            "max_bars": max_bars,
            "epistemic_level": "L3（真实数据，可否证；正/负域诚实报告）",
            "strat_pct": a["strat_pct"], "mdd_pct": a["mdd_pct"], "n_trades": a["n_trades"],
            "n_long": a["n_long"], "n_short": a["n_short"], "P1_ge_bh": p1,
            "prove_panic": 0,  # 能产 JSON ⟹ 流式全程零 panic ⟹ 守恒/四步闭合/穿 ε=−1 成立（L0/L2）
            "exit_reasons": a.get("exit_reasons", {}), "by_ladder": a.get("by_ladder", {}),
            "entries": entries, "sink_opens": cyc_opens, "recover_closes": cyc_closes,
            "core_clears": clears, "liquidations": liqs, "mobile_realized_pnl": round(mob_pnl, 0),
            "max_concurrent_voices": stream_res["max_concurrent_voices"],
            "cross_level_closures": stream_res["cross_level_closures"],
            "fire_phys_long_bars": stream_res["phys_long_bars"],
            "fire_phys_short_bars": stream_res["phys_short_bars"],
            "architecture": "NautilusTrader BacktestEngine 逐bar回放 → on_bar(TFugueStream.push_bar"
                            "(o,h,l,c)) → on_stop(finish); 信号层+仓位层全 Rust 内聚(orchestrator→重跑"
                            "T→完整赋格引擎); 四步循环 H⁰核心⊕H¹机动(穿ε=−1,1/3配额); 复用 fugue_v3 会计"
                            "原语; 数据与 unn/v3 同源(load_bars)",
            "trade_schema": ["ladder", "entry_bar", "entry_price", "exit_bar", "exit_price",
                             "shares", "weight_at_entry", "deferred_bars", "partial",
                             "exit_reason", "polarity"],
            "trades": strat.trades, "equity": strat.equity,
            "closes_first": closes[0], "closes_last": closes[-1],
        }
        OUT_DIR.mkdir(parents=True, exist_ok=True)
        (OUT_DIR / f"t_fugue_{sym}_{mode}.json").write_text(json.dumps(out, ensure_ascii=False, indent=1))
        rows.append({
            "sym": sym, "mode": mode, "bh": bh_pct, "strat": a["strat_pct"],
            "mdd": a["mdd_pct"], "trades": a["n_trades"], "p1": p1,
        })
        engine.reset()
        engine.dispose()
    return rows


def print_matrix(rows: list[dict]) -> None:
    """8 标的 × 3 模式 strat_pct 矩阵（对照 backtest_run.rs 打印格式）。"""
    by_sym: dict = {}
    for r in rows:
        by_sym.setdefault(r["sym"], {})[r["mode"]] = r
    print("\n========== T 流式赋格 × NautilusTrader 8 标的 × 3 模式 strat_pct 矩阵 [L3] ==========")
    print(f"{'标的':<6} {'BH':>10} | {'Structural':>12} {'AND':>12} {'OR':>12} | {'P1(S/A/O)':>10}")
    print("-" * 78)
    for sym, m in by_sym.items():
        s = m.get("structural", {})
        a = m.get("and", {})
        o = m.get("or", {})
        bh = s.get("bh", a.get("bh", o.get("bh", 0.0)))
        p1 = "".join("✓" if m.get(md, {}).get("p1") else "✗" for md in MODES)
        print(f"{sym:<6} {bh:>+9.1f}% | {s.get('strat', 0):>+11.1f}% {a.get('strat', 0):>+11.1f}% "
              f"{o.get('strat', 0):>+11.1f}% | {p1:>10}")
    print("-" * 78)
    print(f"{'trades':<6} {'':>10} | "
          + " ".join(f"{md[:4]:>11}" for md in MODES))
    for sym, m in by_sym.items():
        print(f"{sym:<6} {'':>10} | "
              + " ".join(f"{m.get(md, {}).get('trades', 0):>11}" for md in MODES))
    print("=" * 78)


def main() -> int:
    parser = argparse.ArgumentParser(description="T 流式赋格 × NautilusTrader 真流式回测（8×3）")
    parser.add_argument("--symbols", default=",".join(DEFAULT_SYMBOLS),
                        help="逗号分隔标的（默认全 8）")
    parser.add_argument("--modes", default=",".join(MODES),
                        help="逗号分隔模式 structural/and/or（默认全 3）")
    parser.add_argument("--bars", type=int, default=0, help="最大 bar 数（0=全量）")
    parser.add_argument("--log-level", default="ERROR")
    args = parser.parse_args()

    symbols = [s.strip().upper() for s in args.symbols.split(",") if s.strip()]
    modes = tuple(m.strip().lower() for m in args.modes.split(",") if m.strip())
    for m in modes:
        if m not in MODES:
            print(f"非法模式 {m}（可用: {MODES}）", file=sys.stderr)
            return 1
    for s in symbols:
        if s not in INSTRUMENTS:
            print(f"标的 {s} 未注册（可用: {list(INSTRUMENTS)}）", file=sys.stderr)
            return 1
    max_bars = args.bars or None

    all_rows: list[dict] = []
    for sym in symbols:
        try:
            all_rows.extend(run_symbol(sym, modes, max_bars, args.log_level))
        except FileNotFoundError as e:
            print(f"[{sym}] 数据缺失，跳过: {e}", file=sys.stderr)
        except Exception as e:  # noqa: BLE001 — runner 容错：单标的失败不中断全矩阵
            print(f"[{sym}] 回测失败，跳过: {type(e).__name__}: {e}", file=sys.stderr)

    if all_rows:
        print_matrix(all_rows)
    else:
        print("无任何标的产出结果", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
