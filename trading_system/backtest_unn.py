"""统一必然性引擎（unn，mode="unn"）× NautilusTrader 回测集成。

把 `analysis/unified_necessity_backtest.py` 的批量 unn 引擎接入 trading_system 的
NautilusTrader 骨架（ChanlunStrategy / Databento loader / GLBX venue 装配复用）。

══════════════ 架构本质（诚实声明，no-patch.md 规则）══════════════

unn 引擎 = `newchan_rust.run_positional_rust(rtape, floor_ladder, mode="unn")`
**是批量 tape 消费器**，不是逐 bar 流式引擎：
  - 输入 rtape = `compute_organic_signals(...)` 预计算的整条磁带（递归结构跨**全序列**
    计算——segment/move/recL2-6 的级别涌现需要完整序列，无法逐 bar 增量暴露）。
  - 当前 PyO3 build 只导出 `run_positional_rust`（批量入口），**未导出**逐 bar 的
    `PositionalStream` 流式类；约束"不改 Rust"⟹ 不能凭空造流式接口。

因此本集成的诚实形态（事件驱动框架托管批量引擎的适配器模式）：
  NautilusTrader BacktestEngine = **数据回放 + 生命周期框架**
    （Databento JSON → Bar → msgbus → UnnTapeStrategy.on_bar 缓冲 OHLC）；
  unn 引擎在 **on_stop 批量运行**（缓冲完整 ⟹ 此时才有完整磁带）。
  P&L / equity 曲线来自 unn **自身账本**（res["equity"] / res["final_nav"]），
    **不来自 NT portfolio**——批量引擎在回放结束后无法回溯下 NT 订单。

LMT-only：批量路径不向 NT 下任何单（unn 的几何限价撮合模型已固化在 Rust 引擎内，
  res 的 trades/equity 即该模型产出）；约束"LMT单only / 不执行实盘"平凡满足。

══════════════ 验证判据 ══════════════

P0（管线正确性，L1）：NT 托管路径的 unn 结果与独立的 analysis 基线
  （analysis/data_cache/unn_<SYM>.json）**逐字一致**——证明把 unn 塞进 NT 的
  数据总线不改变任何数字（同引擎 + 同磁带 ⟹ 必须 bit-exact）。
  L1 标注：这是管线一致性验证（两条管线产同样的数），不是新假设的经验验证
  （formalization-validity-domain.md）。

N1-N8（必然性，L2）：引擎内 8 个 prove 函数运行时证明，违反即 panic——
  此调用跑通即 N1-N8 在该标的真实数据上成立（见 analysis/unified_necessity_backtest.py）。

用法（仓库根目录）：
    PYTHONPATH=src .venv/bin/python trading_system/backtest_unn.py            # CL 全量 + 验证
    PYTHONPATH=src .venv/bin/python trading_system/backtest_unn.py --symbol CL
    PYTHONPATH=src .venv/bin/python trading_system/backtest_unn.py --bars 200000  # 子集烟测(跳过验证)
输出：trading_system/data_cache/unn_nt_<SYM>.json
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

import pandas as pd  # noqa: E402

from nautilus_trader.backtest.engine import BacktestEngine  # noqa: E402
from nautilus_trader.backtest.config import BacktestEngineConfig  # noqa: E402
from nautilus_trader.backtest.models import FillModel  # noqa: E402
from nautilus_trader.config import LoggingConfig, StrategyConfig  # noqa: E402
from nautilus_trader.model.currencies import USD  # noqa: E402
from nautilus_trader.model.data import Bar, BarType  # noqa: E402
from nautilus_trader.model.enums import AccountType, OmsType  # noqa: E402
from nautilus_trader.model.identifiers import InstrumentId, TraderId, Venue  # noqa: E402
from nautilus_trader.model.objects import Money  # noqa: E402
from nautilus_trader.trading.strategy import Strategy  # noqa: E402

import newchan_rust as nr  # noqa: E402

from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from nested_recursive_fugue_final_backtest import FLOOR, analyze, bh_mdd  # noqa: E402
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

from trading_system.config.instruments import INSTRUMENTS, make_instrument  # noqa: E402
from trading_system.data.databento_loader import bars_from_dataframe  # noqa: E402

OUT_DIR = REPO_ROOT / "trading_system" / "data_cache"
BASELINE_DIR = REPO_ROOT / "analysis" / "data_cache"


# ════════════════════════ 数据加载（Databento JSON → Bar）════════════════════════

def load_bars(sym: str, bar_type: BarType, price_precision: int,
              max_bars: int | None) -> tuple[list[Bar], list, list, list, list]:
    """analysis Databento JSON → 清洗后 OHLC 数组 + list[Bar]（路 B 构造器）。

    返回 (bars, opens, highs, lows, closes)。bars 与裸数组**结构等价**：
    NT 的 on_bar 缓冲将逐字重建这些数组，是 P0 bit-exact 的前提。

    时间戳设计（诚实声明）：Bar 用**合成单调 1-min 时间戳**（非原始 dates）。
    理由——unn 引擎时间戳盲（signal_bridge.py：「引擎 process_bar 无时间戳，
    bar index = 隐式时间轴」），NT 时间戳仅作回放排序装置。用合成戳而非截断原始
    dates，规避 load_ohlc 中间删行（nan/≤0/spike-revert，如 BRN）导致的日期错位——
    错位会破坏 bit-exact 却不报错（声明膨胀）。volume=0 同理（unn 不消费 volume）。
    """
    path = SYMBOL_FILES[sym]
    opens, highs, lows, closes, _years = load_ohlc(path)  # 唯一清洗真相源
    if max_bars is not None:
        opens, highs, lows, closes = (a[:max_bars] for a in (opens, highs, lows, closes))
    n = len(closes)
    minute_ns = 60_000_000_000
    idx = pd.to_datetime(
        [i * minute_ns for i in range(n)], utc=True,
    )  # 合成单调戳：0, 60s, 120s, ...（unn 无视；NT 仅需单调）
    df = pd.DataFrame(
        {"open": opens, "high": highs, "low": lows, "close": closes,
         "volume": [0.0] * n},
        index=idx,
    )
    bars = bars_from_dataframe(df, bar_type, price_precision)
    return bars, opens, highs, lows, closes


# ════════════════════════ NautilusTrader Strategy（批量托管壳）════════════════════════

class UnnTapeStrategyConfig(StrategyConfig, frozen=True):
    instrument_id: InstrumentId
    bar_type: BarType


class UnnTapeStrategy(Strategy):
    """unn 批量引擎的 NautilusTrader 托管壳。

    NT 角色：数据回放 + 生命周期（on_bar 缓冲 OHLC）。
    unn 角色：on_stop 批量运行（缓冲完整 ⟹ 完整磁带 ⟹ run_positional_rust）。
    结果挂在 self.result（runner 读取），不经 NT portfolio。
    """

    def __init__(self, config: UnnTapeStrategyConfig) -> None:
        super().__init__(config)
        self._o: list[float] = []
        self._h: list[float] = []
        self._l: list[float] = []
        self._c: list[float] = []
        self.result: dict | None = None

    def on_start(self) -> None:
        self.subscribe_bars(self.config.bar_type)
        self.log.info(f"UnnTapeStrategy 启动: {self.config.bar_type}（缓冲 OHLC，unn 在 on_stop 批量运行）")

    def on_bar(self, bar: Bar) -> None:
        # 批量引擎：on_bar 仅缓冲，不做逐 bar 结构计算（unn 是 tape 消费器）。
        self._o.append(bar.open.as_double())
        self._h.append(bar.high.as_double())
        self._l.append(bar.low.as_double())
        self._c.append(bar.close.as_double())

    def on_stop(self) -> None:
        n = len(self._c)
        if n == 0:
            self.log.error("无 bar 缓冲，跳过 unn")
            return
        self.log.info(f"on_stop: 缓冲 {n:,} bar → 运行 unn 批量引擎")
        self.result = run_unn(self._o, self._h, self._l, self._c)


# ════════════════════════ unn 批量引擎调用 ════════════════════════

def run_unn(opens: list, highs: list, lows: list, closes: list) -> dict:
    """compute_organic_signals → pack_tape → run_positional_rust(mode="unn") → analyze。

    与 analysis/unified_necessity_backtest.py 逐字一致（settle on + 仅 dir_flips）——
    P0 bit-exact 的前提。引擎内 8 个 prove 违反即 panic ⟹ 跑通即 N1-N8 成立（L2）。
    """
    dir_flips: list = []
    t0 = time.time()
    tape = compute_organic_signals(opens, highs, lows, closes,
                                   dir_flips=dir_flips, require_settled=True)
    t_sig = time.time() - t0
    rtape = pack_tape(tape, dir_flips=dir_flips)
    t1 = time.time()
    res = nr.run_positional_rust(rtape, floor_ladder=FLOOR, mode="unn")  # prove panic = 必然性失败
    t_eng = time.time() - t1
    # years=None：NT 路径不做分年（分年是诊断维度，不影响 strat_pct/mdd/n_trades 的 P0 比对）。
    a = analyze(res, closes, years=None)
    a["nrf_max_children"] = res.get("nrf_max_children", 0)  # N1 森林实证（analyze 不透传）
    a["_timing"] = {"signals_s": round(t_sig, 1), "engine_s": round(t_eng, 1)}
    return a


# ════════════════════════ P0 验证（管线正确性，L1）════════════════════════

def verify_against_baseline(sym: str, nt_result: dict) -> dict:
    """NT 托管路径 vs analysis 基线（unn_<SYM>.json）逐字一致检查。

    同引擎 + 同磁带 + 同清洗数据 ⟹ strat_pct/mdd/n_trades 必须 bit-exact。
    不一致 = 管线在 NT 托管时引入了差异（数据错位 / 长度不等 / 浮点路径分叉），
    是必须停下来诊断的真实矛盾（no-workaround.md），不静默放过。
    """
    bp = BASELINE_DIR / f"unn_{sym}.json"
    if not bp.exists():
        return {"status": "no_baseline", "note": f"{bp} 不存在，跳过 P0（仅全量有基线）"}
    base = json.loads(bp.read_text())
    if "failed" in base:
        return {"status": "baseline_failed", "note": base["failed"]}
    b = base["unn"]
    checks = {
        "strat_pct": (nt_result["strat_pct"], b["strat_pct"]),
        "mdd_pct": (nt_result["mdd_pct"], b["mdd_pct"]),
        "n_trades": (nt_result["n_trades"], b["n_trades"]),
    }
    mismatches = {k: {"nt": v[0], "baseline": v[1]} for k, v in checks.items() if v[0] != v[1]}
    return {
        "status": "PASS" if not mismatches else "MISMATCH",
        "checks": {k: v[0] for k, v in checks.items()},
        "mismatches": mismatches,
    }


# ════════════════════════ runner ════════════════════════

def build_engine(spec, log_level: str) -> tuple[BacktestEngine, object, BarType]:
    instrument = make_instrument(spec)
    venue = instrument.id.venue
    engine = BacktestEngine(
        config=BacktestEngineConfig(
            trader_id=TraderId("UNN-NT-001"),
            logging=LoggingConfig(log_level=log_level),
        ),
    )
    engine.add_venue(
        venue=venue,
        oms_type=OmsType.NETTING,
        account_type=AccountType.MARGIN,
        base_currency=USD,
        starting_balances=[Money(1_000_000.0, USD)],
        # 判决一：保守撮合口径（队列末位，穿越才成交）——批量路径不下单，此处仅为装配完整。
        fill_model=FillModel(prob_fill_on_limit=0.0, prob_slippage=0.0),
    )
    engine.add_instrument(instrument)
    bar_type = BarType.from_str(f"{instrument.id}-1-MINUTE-LAST-EXTERNAL")
    strategy = UnnTapeStrategy(
        config=UnnTapeStrategyConfig(instrument_id=instrument.id, bar_type=bar_type),
    )
    engine.add_strategy(strategy)
    return engine, instrument, bar_type


def main() -> int:
    parser = argparse.ArgumentParser(description="unn × NautilusTrader 回测集成")
    parser.add_argument("--symbol", default="CL", help="标的（默认 CL；有基线的 ES/CL 可验证）")
    parser.add_argument("--bars", type=int, default=0, help="最大 bar 数（0=全量；子集跳过 P0 验证）")
    parser.add_argument("--log-level", default="ERROR", help="Nautilus 日志级别")
    args = parser.parse_args()

    sym = args.symbol
    if sym not in INSTRUMENTS:
        print(f"标的 {sym} 未在 INSTRUMENTS 注册（可用: {list(INSTRUMENTS)}）", file=sys.stderr)
        return 1
    if sym not in SYMBOL_FILES:
        print(f"标的 {sym} 无数据文件映射", file=sys.stderr)
        return 1

    spec = INSTRUMENTS[sym]
    max_bars = args.bars or None
    OUT_DIR.mkdir(parents=True, exist_ok=True)

    engine, instrument, bar_type = build_engine(spec, args.log_level)

    print(f"加载 {sym} 数据（max_bars={args.bars or '全量'}）...")
    t0 = time.time()
    bars, opens, highs, lows, closes = load_bars(
        sym, bar_type, spec.price_precision, max_bars,
    )
    bh_pct = (closes[-1] / closes[0] - 1) * 100
    bh_dd = bh_mdd(closes) * 100
    print(f"  {len(bars):,} 根 Bar 构造完成 {time.time() - t0:.1f}s  "
          f"BH={bh_pct:+.1f}% BH_MDD={bh_dd:.1f}%")

    engine.add_data(bars)
    print("NautilusTrader 回放（on_bar 缓冲 → on_stop 批量 unn）...")
    t1 = time.time()
    engine.run()
    print(f"  回放+unn 完成 {time.time() - t1:.1f}s")

    strategy = engine.trader.strategies()[0]
    result = strategy.result
    if result is None:
        print("unn 未产出结果（缓冲为空？）", file=sys.stderr)
        engine.dispose()
        return 1

    # P0 验证（仅全量；子集与全量基线不可比）
    verify = None
    if max_bars is None:
        verify = verify_against_baseline(sym, result)

    print("\n========== unn × NautilusTrader 回测完成 ==========")
    print(f"标的          : {sym}（{instrument.id}）")
    print(f"bars 回放     : {len(closes):,}")
    print(f"strat_pct     : {result['strat_pct']:+.1f}%   (BH {bh_pct:+.1f}%, "
          f"P1={'PASS' if result['strat_pct'] >= bh_pct else 'fail'})")
    print(f"max drawdown  : {result['mdd_pct']:.1f}%   (BH_MDD {bh_dd:.1f}%)")
    print(f"n_trades      : {result['n_trades']}")
    nrf = result["nrf_counters"]
    print(f"N1 max_kids   : {result.get('nrf_max_children', '?')}  "
          f"roots={sum(nrf['root_entries'])} spawns={sum(nrf['spawns'])}")
    print(f"耗时          : 信号层 {result['_timing']['signals_s']}s + "
          f"引擎 {result['_timing']['engine_s']}s")
    if verify is not None:
        print(f"P0 (vs 基线)  : {verify['status']}  {verify.get('mismatches') or verify.get('note', '')}")

    out = {
        "symbol": sym, "instrument_id": str(instrument.id), "n_bars": len(closes),
        "bh_pct": round(bh_pct, 1), "bh_mdd_pct": round(bh_dd, 1),
        "max_bars": args.bars or None,
        "unn": result,
        "P1_ge_bh": result["strat_pct"] >= bh_pct,
        "P0_pipeline_verify": verify,
        "architecture": "NautilusTrader=数据回放+生命周期；unn=on_stop批量(tape消费);"
                        "P&L来自unn账本非NT portfolio;LMT-only(批量不下单)",
    }
    (OUT_DIR / f"unn_nt_{sym}.json").write_text(json.dumps(out, ensure_ascii=False, indent=1))
    print(f"\n结果写入 {OUT_DIR / f'unn_nt_{sym}.json'}")

    engine.reset()
    engine.dispose()

    if verify is not None and verify["status"] == "MISMATCH":
        print("\n⚠️ P0 失败：NT 托管路径与基线不一致——管线引入了差异，须诊断", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
