"""NewChan 命令行入口"""

from __future__ import annotations

import argparse
import sys

from newchan import __version__


# ------------------------------------------------------------------
# argparse 构建
# ------------------------------------------------------------------


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="newchan",
        description="缠论量化分析工具",
    )
    parser.add_argument(
        "-V", "--version",
        action="version",
        version=f"%(prog)s {__version__}",
    )

    sub = parser.add_subparsers(dest="command")

    # ---- fetch 子命令 ------------------------------------------------
    fetch_p = sub.add_parser("fetch", help="拉取行情数据")
    fetch_p.add_argument(
        "--source",
        choices=["ibkr", "av"],
        default="ibkr",
        help="数据源: ibkr (Interactive Brokers) 或 av (Alpha Vantage)，默认 ibkr",
    )
    fetch_p.add_argument(
        "--symbol",
        required=True,
        help="品种代码（ibkr: CL/GC/ES/NQ/… | av: BRENT）",
    )
    fetch_p.add_argument(
        "--interval",
        default="1min",
        help="K 线周期，如 1min/5min/15min/1hour/1day，默认 1min",
    )
    fetch_p.add_argument(
        "--duration",
        default="2 D",
        help='拉取时长（仅 ibkr），IB 格式，如 "2 D"、"1 W"，默认 "2 D"',
    )
    fetch_p.add_argument(
        "--refresh",
        action="store_true",
        default=False,
        help="强制刷新（忽略本地缓存）",
    )

    # ---- plot 子命令 ------------------------------------------------
    plot_p = sub.add_parser("plot", help="交互式蜡烛图（TradingView 风格）")
    plot_p.add_argument(
        "--symbol",
        required=True,
        help="品种代码（需先 fetch 过）",
    )
    plot_p.add_argument(
        "--interval",
        default="1min",
        help="原始数据的 K 线周期（对应 fetch 时的 --interval），默认 1min",
    )
    plot_p.add_argument(
        "--display-tf",
        default="1m",
        help="初始显示周期: 1m/5m/15m/30m/1h/4h/1d/1w，默认 1m",
    )
    plot_p.add_argument(
        "--compare",
        default=None,
        help="合成标的缓存名（如 CL_GC_spread），启用上下分屏对比模式",
    )

    # ---- chart 子命令 ------------------------------------------------
    chart_p = sub.add_parser("chart", help="启动交互式图表服务（TradingView 风格）")
    chart_p.add_argument(
        "--port",
        type=int,
        default=8765,
        help="HTTP 服务端口，默认 8765",
    )

    # ---- synthetic 子命令 --------------------------------------------
    synth_p = sub.add_parser("synthetic", help="生成合成标的（价差/比值）")
    synth_p.add_argument(
        "--a",
        required=True,
        dest="sym_a",
        help="品种 A（如 CL）",
    )
    synth_p.add_argument(
        "--b",
        required=True,
        dest="sym_b",
        help="品种 B（如 GC）",
    )
    synth_p.add_argument(
        "--op",
        choices=["spread", "ratio"],
        default="spread",
        help="运算方式: spread (A-B) 或 ratio (A/B)，默认 spread",
    )
    synth_p.add_argument(
        "--interval",
        default="1min",
        help="原始数据的 K 线周期，默认 1min",
    )

    # ---- fetch-db 子命令（Databento 批量拉取）----------------------------
    fetchdb_p = sub.add_parser("fetch-db", help="从 Databento 拉取完整历史数据")
    fetchdb_p.add_argument(
        "--symbol",
        default=None,
        help="品种代码（如 CL, AMD）；不指定时用 --all",
    )
    fetchdb_p.add_argument(
        "--all",
        action="store_true",
        default=False,
        dest="fetch_all",
        help="拉取所有默认标的（CL,GC,ES,NQ,SI,AMD,NVDA）",
    )
    fetchdb_p.add_argument(
        "--intervals",
        default="1min,1day",
        help="逗号分隔的周期列表，默认 1min,1day",
    )
    fetchdb_p.add_argument(
        "--start",
        default="2010-01-01",
        help="起始日期 YYYY-MM-DD，默认 2010-01-01",
    )

    return parser


# ------------------------------------------------------------------
# 环境检查
# ------------------------------------------------------------------


def _check_av_env() -> None:
    """检查 Alpha Vantage 环境变量。"""
    from newchan.config import ALPHAVANTAGE_API_KEY

    if not ALPHAVANTAGE_API_KEY:
        print(
            "错误: 未设置 ALPHAVANTAGE_API_KEY。\n"
            "请在项目根目录创建 .env 文件并填写该值，可参考 .env.example。",
            file=sys.stderr,
        )
        sys.exit(1)


# ------------------------------------------------------------------
# fetch 子命令
# ------------------------------------------------------------------

_AV_SYMBOLS = {"BRENT"}


def _cmd_fetch(args: argparse.Namespace) -> None:
    """处理 fetch 子命令。"""
    from newchan.cache import load_df, save_df
    from newchan.convert import bars_to_df

    symbol = args.symbol.upper()
    source = args.source
    interval = args.interval
    cache_name = f"{symbol}_{interval}_raw"

    # 缓存命中
    if not args.refresh:
        cached = load_df(cache_name)
        if cached is not None:
            print(f"已从缓存加载 {cache_name}，共 {len(cached)} 条。")
            print(cached.tail())
            return

    # ---------- IBKR ----------
    if source == "ibkr":
        from newchan.data_ibkr import IBKRProvider

        print(f"正在通过 IBKR 拉取 {symbol} {interval} 数据（duration={args.duration}）…")
        with IBKRProvider() as provider:
            bars = provider.fetch_historical(
                symbol=symbol,
                interval=interval,
                duration=args.duration,
            )

    # ---------- Alpha Vantage ----------
    elif source == "av":
        _check_av_env()
        from newchan.data_av import AlphaVantageProvider

        if symbol not in _AV_SYMBOLS:
            print(
                f"错误: Alpha Vantage 暂不支持 {symbol}，"
                f"当前支持: {', '.join(sorted(_AV_SYMBOLS))}",
                file=sys.stderr,
            )
            sys.exit(1)

        print(f"正在从 Alpha Vantage 拉取 {symbol} daily 数据 …")
        provider_av = AlphaVantageProvider()
        if symbol == "BRENT":
            bars = provider_av.fetch_brent_daily()
    else:
        print(f"错误: 未知数据源 {source}", file=sys.stderr)
        sys.exit(1)

    if not bars:
        print("警告: 未获取到任何数据。")
        return

    df = bars_to_df(bars)
    path = save_df(cache_name, df)
    print(f"已保存 {len(df)} 条到 {path}")
    print(df.tail())


# ------------------------------------------------------------------
# plot 子命令
# ------------------------------------------------------------------


def _load_cache_or_exit(cache_name: str, symbol: str, interval: str) -> "pd.DataFrame":
    """从缓存加载数据，不存在则报错退出。"""
    from newchan.cache import load_df

    df = load_df(cache_name)
    if df is None:
        print(
            f"错误: 缓存 {cache_name} 不存在，请先运行:\n"
            f"  python -m newchan.cli fetch --symbol {symbol} --interval {interval}",
            file=sys.stderr,
        )
        sys.exit(1)
    return df


def _cmd_plot(args: argparse.Namespace) -> None:
    """处理 plot 子命令 — 交互式蜡烛图。"""
    symbol = args.symbol.upper()
    interval = args.interval
    cache_name = f"{symbol}_{interval}_raw"

    df = _load_cache_or_exit(cache_name, symbol, interval)
    print(f"已加载 {cache_name}，共 {len(df)} 条。")

    compare_name = args.compare

    if compare_name is None:
        # 单品种模式
        from newchan.b_chart import show_chart

        show_chart(df, symbol=symbol, default_tf=args.display_tf)
    else:
        # 对比模式
        from newchan.b_chart import show_compare
        from newchan.cache import load_df

        synth_cache = f"{compare_name}_{interval}_raw"
        df_synth = load_df(synth_cache)
        if df_synth is None:
            print(
                f"错误: 合成标的缓存 {synth_cache} 不存在，请先运行:\n"
                f"  python -m newchan.cli synthetic --a ... --b ... --interval {interval}",
                file=sys.stderr,
            )
            sys.exit(1)

        print(f"已加载合成标的 {synth_cache}，共 {len(df_synth)} 条。")
        show_compare(df, df_synth, symbol=symbol, synth_name=compare_name, default_tf=args.display_tf)


# ------------------------------------------------------------------
# synthetic 子命令
# ------------------------------------------------------------------


def _cmd_synthetic(args: argparse.Namespace) -> None:
    """处理 synthetic 子命令 — 生成合成标的。"""
    from newchan.cache import save_df
    from newchan.synthetic import make_ratio, make_spread

    sym_a = args.sym_a.upper()
    sym_b = args.sym_b.upper()
    interval = args.interval
    op = args.op

    df_a = _load_cache_or_exit(f"{sym_a}_{interval}_raw", sym_a, interval)
    df_b = _load_cache_or_exit(f"{sym_b}_{interval}_raw", sym_b, interval)

    if op == "ratio":
        from newchan.equivalence import validate_pair

        vr = validate_pair(df_a, df_b)
        if not vr.valid:
            print(f"⚠ 等价对验证未通过: {vr.reason}")
            print("  继续生成比价K线，但分析结果可能不可靠。")

    print(f"正在计算 {sym_a} {op} {sym_b} …")
    if op == "spread":
        df_synth = make_spread(df_a, df_b)
    else:
        df_synth = make_ratio(df_a, df_b)

    synth_name = f"{sym_a}_{sym_b}_{op}"
    cache_name = f"{synth_name}_{interval}_raw"
    path = save_df(cache_name, df_synth)
    print(f"已保存 {len(df_synth)} 条到 {path}")
    print(f"查看: python -m newchan.cli plot --symbol {sym_a} --compare {synth_name} --interval {interval}")
    print(df_synth.tail())


# ------------------------------------------------------------------
# fetch-db 子命令（Databento 批量拉取）
# ------------------------------------------------------------------


def _cmd_fetch_db(args: argparse.Namespace) -> None:
    """从 Databento 拉取完整历史数据。"""
    from newchan.data_databento import fetch_and_cache, DEFAULT_SYMBOLS

    if args.fetch_all:
        symbols = DEFAULT_SYMBOLS
    elif args.symbol:
        symbols = [s.strip().upper() for s in args.symbol.split(",")]
    else:
        print("错误: 请指定 --symbol 或 --all", file=sys.stderr)
        sys.exit(1)

    intervals = [s.strip() for s in args.intervals.split(",")]
    start = args.start
    total = len(symbols) * len(intervals)
    done = 0

    print(f"Databento 批量拉取: {len(symbols)} 标的 × {len(intervals)} 周期 = {total} 任务")
    print(f"起始日期: {start}")
    print(f"标的: {', '.join(symbols)}")
    print(f"周期: {', '.join(intervals)}")
    print("-" * 60)

    for symbol in symbols:
        for interval in intervals:
            done += 1
            print(f"[{done}/{total}] {symbol} {interval} ...", end=" ", flush=True)
            try:
                cache_name, count = fetch_and_cache(symbol, interval, start=start)
                print(f"✓ {count} 条 → {cache_name}")
            except Exception as e:
                print(f"✗ 失败: {e}")

    print("-" * 60)
    print("完成！")


# ------------------------------------------------------------------
# 入口
# ------------------------------------------------------------------


def main() -> None:
    parser = build_parser()
    args = parser.parse_args()

    if args.command is None:
        parser.print_help()
        return

    if args.command == "fetch":
        _cmd_fetch(args)
    elif args.command == "plot":
        _cmd_plot(args)
    elif args.command == "chart":
        from newchan.server import run_server
        run_server(port=args.port)
    elif args.command == "synthetic":
        _cmd_synthetic(args)
    elif args.command == "fetch-db":
        _cmd_fetch_db(args)


if __name__ == "__main__":
    main()
