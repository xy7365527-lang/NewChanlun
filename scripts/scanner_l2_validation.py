"""scanner-l2-validation: 真实数据获取 + L2 验证。

目标：用真实 A 股日线数据验证 fold_equivalence 商空间排序是否
      产生与扁平排序（单纯按 tightness 排）不同的结果，以及
      该差异是否有选股意义。

认识论等级：L2（真实数据验证，单时段多标的）。

谱系引用：292号折叠拓扑本体论、352号多标的扫描器设计。
"""

from __future__ import annotations

import json
import logging
import sys
import time
from dataclasses import asdict, dataclass
from datetime import datetime
from pathlib import Path

# 确保项目 src 在 path 上
_project_root = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(_project_root / "src"))

import baostock as bs

from newchan.orchestrator.recursive import RecursiveOrchestrator
from newchan.trading.fold_equivalence import (
    DTriState,
    EquivalenceClass,
    FoldChannel,
    TargetAttributes,
    build_quotient_space,
    rank_quotient_space,
)
from newchan.types import Bar

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(name)s %(message)s")
logger = logging.getLogger("scanner_l2")

# ═══════════════════════════════════════════════════════════════
# 标的配置：覆盖多折叠通道 + 多板块
# ═══════════════════════════════════════════════════════════════

# Au 通道：黄金股（ETF 用不同接口，这里用个股）
AU_SYMBOLS = {
    "600547": {"name": "山东黄金", "channel": FoldChannel.AU},
    "002155": {"name": "湖南黄金", "channel": FoldChannel.AU},
    "600489": {"name": "中金黄金", "channel": FoldChannel.AU},
}

# Oil 通道：石油石化
OIL_SYMBOLS = {
    "600028": {"name": "中国石化", "channel": FoldChannel.OIL},
    "601857": {"name": "中国石油", "channel": FoldChannel.OIL},
    "600688": {"name": "上海石化", "channel": FoldChannel.OIL},
}

# Bond 通道：利率敏感股（替代 ETF）
BOND_SYMBOLS = {
    "601398": {"name": "工商银行", "channel": FoldChannel.BOND},
    "601288": {"name": "农业银行", "channel": FoldChannel.BOND},
}

# RE 通道：房地产
RE_SYMBOLS = {
    "600048": {"name": "保利发展", "channel": FoldChannel.RE},
    "000002": {"name": "万科A", "channel": FoldChannel.RE},
    "001979": {"name": "招商蛇口", "channel": FoldChannel.RE},
}

# EQUITY 通道：多板块
EQUITY_SYMBOLS = {
    # 科技板块
    "002230": {"name": "科大讯飞", "channel": FoldChannel.EQUITY, "sector": "tech"},
    "300059": {"name": "东方财富", "channel": FoldChannel.EQUITY, "sector": "tech"},
    "000725": {"name": "京东方A", "channel": FoldChannel.EQUITY, "sector": "tech"},
    # 金融板块
    "601318": {"name": "中国平安", "channel": FoldChannel.EQUITY, "sector": "finance"},
    "600036": {"name": "招商银行", "channel": FoldChannel.EQUITY, "sector": "finance"},
    "601166": {"name": "兴业银行", "channel": FoldChannel.EQUITY, "sector": "finance"},
    # 消费板块
    "600519": {"name": "贵州茅台", "channel": FoldChannel.EQUITY, "sector": "consumer"},
    "000858": {"name": "五粮液", "channel": FoldChannel.EQUITY, "sector": "consumer"},
    "002304": {"name": "洋河股份", "channel": FoldChannel.EQUITY, "sector": "consumer"},
    # 医药板块
    "300760": {"name": "迈瑞医疗", "channel": FoldChannel.EQUITY, "sector": "pharma"},
    "600276": {"name": "恒瑞医药", "channel": FoldChannel.EQUITY, "sector": "pharma"},
    # 能源板块
    "601985": {"name": "中国核电", "channel": FoldChannel.EQUITY, "sector": "energy"},
    "600900": {"name": "长江电力", "channel": FoldChannel.EQUITY, "sector": "energy"},
}

ALL_SYMBOLS = {}
ALL_SYMBOLS.update(AU_SYMBOLS)
ALL_SYMBOLS.update(OIL_SYMBOLS)
ALL_SYMBOLS.update(BOND_SYMBOLS)
ALL_SYMBOLS.update(RE_SYMBOLS)
ALL_SYMBOLS.update(EQUITY_SYMBOLS)

DATA_DIR = _project_root / "data" / "scanner_l2"


# ═══════════════════════════════════════════════════════════════
# 第一步：数据获取
# ═══════════════════════════════════════════════════════════════


def _symbol_to_baostock(symbol: str) -> str:
    """将纯6位代码转为 baostock 格式（sh.XXXXXX 或 sz.XXXXXX）。"""
    if symbol.startswith(("6", "5")):
        return f"sh.{symbol}"
    return f"sz.{symbol}"


def fetch_daily_data(
    start_date: str = "2020-01-01",
    end_date: str = "2026-03-01",
) -> dict[str, list[Bar]]:
    """获取所有标的的日线数据（5年跨度，确保足够结构深度）。

    优先从本地缓存读取，缓存不存在时从 baostock 拉取。
    """
    DATA_DIR.mkdir(parents=True, exist_ok=True)
    result: dict[str, list[Bar]] = {}

    # baostock 需要 login/logout
    lg = bs.login()
    if lg.error_code != "0":
        logger.error("baostock login failed: %s", lg.error_msg)
        return result

    try:
        for symbol, info in ALL_SYMBOLS.items():
            cache_path = DATA_DIR / f"{symbol}_daily.json"

            # 尝试读缓存
            if cache_path.exists():
                bars = _load_bars_from_cache(cache_path)
                if len(bars) > 1000:  # 5年数据约1400+bars
                    logger.info("缓存命中: %s (%s) — %d bars", symbol, info["name"], len(bars))
                    result[symbol] = bars
                    continue

            # 从 baostock 拉取
            bs_code = _symbol_to_baostock(symbol)
            logger.info("拉取数据: %s (%s) → %s", symbol, info["name"], bs_code)
            rs = bs.query_history_k_data_plus(
                bs_code,
                "date,open,high,low,close,volume",
                start_date=start_date,
                end_date=end_date,
                frequency="d",
                adjustflag="2",  # 前复权
            )
            if rs.error_code != "0":
                logger.warning("拉取失败: %s — %s", symbol, rs.error_msg)
                continue

            rows: list[list[str]] = []
            while rs.next():
                rows.append(rs.get_row_data())

            if not rows:
                logger.warning("空数据: %s", symbol)
                continue

            # 转为 Bar 列表
            bars = _baostock_rows_to_bars(rows)
            if not bars:
                logger.warning("转换后空: %s", symbol)
                continue

            # 写缓存
            _save_bars_to_cache(cache_path, bars)
            logger.info("已缓存: %s — %d bars", symbol, len(bars))
            result[symbol] = bars

            time.sleep(0.3)  # baostock 限流较宽松
    finally:
        bs.logout()

    return result


def _baostock_rows_to_bars(rows: list[list[str]]) -> list[Bar]:
    """将 baostock 行数据转为 Bar 列表。"""
    bars: list[Bar] = []
    for row in rows:
        # row = [date, open, high, low, close, volume]
        if not row[1] or row[1] == "":
            continue
        try:
            bars.append(Bar(
                ts=datetime.strptime(row[0], "%Y-%m-%d"),
                open=float(row[1]),
                high=float(row[2]),
                low=float(row[3]),
                close=float(row[4]),
                volume=float(row[5]) if row[5] else None,
            ))
        except (ValueError, IndexError):
            continue
    bars.sort(key=lambda b: b.ts)
    return bars


def _save_bars_to_cache(path: Path, bars: list[Bar]) -> None:
    """将 Bar 列表序列化为 JSON 缓存。"""
    data = [
        {
            "ts": b.ts.isoformat(),
            "open": b.open,
            "high": b.high,
            "low": b.low,
            "close": b.close,
            "volume": b.volume,
        }
        for b in bars
    ]
    path.write_text(json.dumps(data, ensure_ascii=False), encoding="utf-8")


def _load_bars_from_cache(path: Path) -> list[Bar]:
    """从 JSON 缓存读取 Bar 列表。"""
    data = json.loads(path.read_text(encoding="utf-8"))
    return [
        Bar(
            ts=datetime.fromisoformat(d["ts"]),
            open=d["open"],
            high=d["high"],
            low=d["low"],
            close=d["close"],
            volume=d["volume"],
        )
        for d in data
    ]


# ═══════════════════════════════════════════════════════════════
# 第二步：跑缠论引擎 → 获取 tightness
# ═══════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class SymbolAnalysis:
    """单标的分析结果。"""

    symbol: str
    name: str
    channel: FoldChannel
    sector: str
    bar_count: int
    tightness: float
    operation_level: str
    avg_daily_volume: float
    level_magnitude: float  # 用有买点的最高 level_id 作为 proxy


def _compute_structural_tightness(snap) -> tuple[float, str]:
    """从快照中计算结构紧度。

    不依赖 compute_nesting_tightness（仅检查 buy 方向），
    而是检查所有已确认的 BSP（buy 和 sell），因为 L2 验证
    的目标是排序机制，不是交易方向。

    紧度 = level1 有确认 BSP 贡献 1.0 + 每个递归层有 BSP 额外 +1.0。
    """
    tightness = 0.0
    max_level = 0

    # Level 1: 检查所有已确认 BSP
    for bp in snap.bsp_snapshot.buysellpoints:
        if bp.confirmed:
            tightness += 1.0
            max_level = 1
            break  # 只计一次

    # 递归层：检查 moves（有 move = 有结构深度）
    for rs in snap.recursive_snapshots:
        if len(rs.moves) > 0:
            tightness += 1.0
            if rs.level_id > max_level:
                max_level = rs.level_id

    if max_level == 0:
        return 0.0, ""
    return tightness, f"L{max_level}"


def run_engine_on_symbol(
    symbol: str,
    bars: list[Bar],
    info: dict,
) -> SymbolAnalysis:
    """对单个标的跑 RecursiveOrchestrator，提取 tightness。

    扫描整个历史，记录最大 tightness（而非仅看最后一根 bar 的快照），
    因为买点是瞬时事件——日线 277 bars 中最后一根恰好有活跃买点的概率很低。
    """
    orch = RecursiveOrchestrator(stream_id=symbol, max_levels=4)

    max_tightness = 0.0
    max_op_level = ""

    for bar in bars:
        snap = orch.process_bar(bar)
        t, op_level = _compute_structural_tightness(snap)
        if t > max_tightness:
            max_tightness = t
            max_op_level = op_level

    # 日均成交量
    volumes = [b.volume for b in bars if b.volume is not None and b.volume > 0]
    avg_vol = sum(volumes) / len(volumes) if volumes else 0.0

    # level_magnitude: 有买点的最高级别
    level_mag = 0.0
    if max_op_level.startswith("L"):
        try:
            level_mag = float(max_op_level[1:])
        except ValueError:
            pass

    return SymbolAnalysis(
        symbol=symbol,
        name=info["name"],
        channel=info["channel"],
        sector=info.get("sector", ""),
        bar_count=len(bars),
        tightness=max_tightness,
        operation_level=max_op_level,
        avg_daily_volume=avg_vol,
        level_magnitude=level_mag,
    )


# ═══════════════════════════════════════════════════════════════
# 第三步：构造 TargetAttributes → 商空间 + 扁平排序对比
# ═══════════════════════════════════════════════════════════════


def analysis_to_target(a: SymbolAnalysis) -> TargetAttributes | None:
    """将分析结果转为 TargetAttributes。tightness=0 的排除。"""
    if a.tightness <= 0.0:
        return None

    return TargetAttributes(
        symbol=a.symbol,
        fold_channel=a.channel,
        sector=a.sector,
        d_tri_state=DTriState.RETAIN,  # 简化：L2 阶段暂不区分三态
        tightness=a.tightness,
        level_magnitude=a.level_magnitude,
        liquidity=a.avg_daily_volume,
    )


def compare_rankings(analyses: list[SymbolAnalysis]) -> dict:
    """对比商空间排序 vs 扁平排序。

    Returns
    -------
    dict
        包含两种排序结果和差异分析。
    """
    # 构造 TargetAttributes
    targets_list: list[TargetAttributes] = []
    for a in analyses:
        t = analysis_to_target(a)
        if t is not None:
            targets_list.append(t)

    if not targets_list:
        return {
            "error": "无有效标的（所有 tightness=0）",
            "total_symbols": len(analyses),
            "filtered_count": 0,
        }

    targets = tuple(targets_list)

    # 1. 商空间排序
    quotient = build_quotient_space(targets)
    ranked = rank_quotient_space(quotient)

    quotient_order = []
    for rank_val, ec, rep in ranked:
        quotient_order.append({
            "rank": round(rank_val, 4),
            "symbol": rep.symbol,
            "name": ALL_SYMBOLS.get(rep.symbol, {}).get("name", "?"),
            "channel": ec.fold_channel.name,
            "weight": ec.weight,
            "tightness": round(ec.max_tightness, 4),
            "class_size": ec.size,
            "members": [m.symbol for m in ec.members],
        })

    # 2. 扁平排序（仅按 tightness 降序）
    flat_sorted = sorted(targets_list, key=lambda t: (-t.tightness, -t.liquidity))
    flat_order = []
    for t in flat_sorted:
        flat_order.append({
            "symbol": t.symbol,
            "name": ALL_SYMBOLS.get(t.symbol, {}).get("name", "?"),
            "tightness": round(t.tightness, 4),
            "channel": t.fold_channel.name,
        })

    # 3. 差异分析
    quotient_symbols = [entry["symbol"] for entry in quotient_order]
    flat_symbols = [entry["symbol"] for entry in flat_order]

    # 扁平排序中每个 symbol 只出现一次，商空间中取代表元
    # 计算 Kendall tau 式的排序差异
    rank_divergence = _compute_rank_divergence(quotient_symbols, flat_symbols)

    # 折叠合并效果：有多少等价类包含 >1 个成员
    multi_member_classes = [ec for ec in quotient if ec.size > 1]
    fold_merges = len(multi_member_classes)
    total_merged = sum(ec.size for ec in multi_member_classes)

    return {
        "total_symbols": len(analyses),
        "tightness_gt_zero": len(targets_list),
        "equivalence_classes": len(quotient),
        "fold_merges": fold_merges,
        "total_symbols_merged": total_merged,
        "rank_divergence": rank_divergence,
        "quotient_ranking": quotient_order,
        "flat_ranking": flat_order,
    }


def _compute_rank_divergence(
    quotient_order: list[str],
    flat_order: list[str],
) -> dict:
    """计算两种排序的差异。

    使用 Spearman rank correlation 的简化版：
    对两个排序中都出现的 symbol，计算排名差异。
    """
    # 商空间 symbol 到排名
    q_rank = {s: i for i, s in enumerate(quotient_order)}

    # 扁平排序中只取商空间代表元对应的 symbol
    # 扁平排序中所有 symbol 都有排名
    f_rank = {s: i for i, s in enumerate(flat_order)}

    # 公共 symbol 集合
    common = set(q_rank.keys()) & set(f_rank.keys())
    if not common:
        return {"common_symbols": 0, "rank_differences": []}

    diffs = []
    for s in sorted(common, key=lambda x: q_rank[x]):
        diffs.append({
            "symbol": s,
            "quotient_rank": q_rank[s],
            "flat_rank": f_rank[s],
            "delta": f_rank[s] - q_rank[s],
        })

    total_displacement = sum(abs(d["delta"]) for d in diffs)
    max_possible = len(common) * (len(common) - 1) // 2 if len(common) > 1 else 1

    return {
        "common_symbols": len(common),
        "total_displacement": total_displacement,
        "normalized_displacement": round(total_displacement / max_possible, 4) if max_possible > 0 else 0.0,
        "rank_differences": diffs,
    }


# ═══════════════════════════════════════════════════════════════
# 主程序
# ═══════════════════════════════════════════════════════════════


def main() -> None:
    logger.info("=== Scanner L2 Validation: 真实数据获取 + L2 验证 ===")
    logger.info("标的数: %d", len(ALL_SYMBOLS))

    # 第一步：数据获取
    logger.info("--- 第一步：数据获取 ---")
    all_bars = fetch_daily_data()
    logger.info("成功获取 %d / %d 标的数据", len(all_bars), len(ALL_SYMBOLS))

    if len(all_bars) < 5:
        logger.error("数据不足（<5 标的），无法进行有意义的 L2 验证")
        sys.exit(1)

    # 第二步：跑引擎
    logger.info("--- 第二步：跑缠论引擎 ---")
    analyses: list[SymbolAnalysis] = []
    for symbol, bars in all_bars.items():
        info = ALL_SYMBOLS[symbol]
        logger.info("分析: %s (%s) — %d bars", symbol, info["name"], len(bars))
        try:
            a = run_engine_on_symbol(symbol, bars, info)
            analyses.append(a)
            logger.info(
                "  → tightness=%.2f, level=%s, avg_vol=%.0f",
                a.tightness, a.operation_level, a.avg_daily_volume,
            )
        except Exception as e:
            logger.error("分析失败: %s — %s", symbol, e)

    # 第三步：商空间 vs 扁平排序
    logger.info("--- 第三步：商空间 vs 扁平排序对比 ---")
    comparison = compare_rankings(analyses)

    # 输出结果
    report_path = DATA_DIR / "l2_validation_report.json"
    DATA_DIR.mkdir(parents=True, exist_ok=True)
    report_path.write_text(
        json.dumps(comparison, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )
    logger.info("报告已保存: %s", report_path)

    # 打印摘要
    print("\n" + "=" * 70)
    print("Scanner L2 Validation 结果摘要")
    print("=" * 70)
    print(f"总标的数: {comparison.get('total_symbols', 0)}")
    print(f"tightness > 0 标的数: {comparison.get('tightness_gt_zero', 0)}")
    print(f"等价类数: {comparison.get('equivalence_classes', 0)}")
    print(f"折叠合并（>1成员等价类）: {comparison.get('fold_merges', 0)}")
    print(f"被折叠合并的标的数: {comparison.get('total_symbols_merged', 0)}")

    rd = comparison.get("rank_divergence", {})
    print(f"\n排序差异（归一化）: {rd.get('normalized_displacement', 'N/A')}")
    if rd.get("rank_differences"):
        print("\n具体排名差异:")
        for d in rd["rank_differences"]:
            name = ALL_SYMBOLS.get(d["symbol"], {}).get("name", "?")
            print(f"  {d['symbol']} ({name}): "
                  f"商空间#{d['quotient_rank']+1} vs 扁平#{d['flat_rank']+1} "
                  f"(Δ={d['delta']:+d})")

    print("\n商空间排序:")
    for i, entry in enumerate(comparison.get("quotient_ranking", []), 1):
        print(f"  #{i} {entry['symbol']} ({entry['name']}) "
              f"rank={entry['rank']:.2f} "
              f"[{entry['channel']} W={entry['weight']}] "
              f"T={entry['tightness']:.2f} "
              f"类内={entry['class_size']}标的")

    print("\n扁平排序（仅按 T）:")
    for i, entry in enumerate(comparison.get("flat_ranking", []), 1):
        print(f"  #{i} {entry['symbol']} ({entry['name']}) "
              f"T={entry['tightness']:.2f} [{entry['channel']}]")

    print("\n" + "=" * 70)
    print("认识论等级: L2（真实数据验证，单时段多标的）")
    print("验证命题: 商空间排序(T*W)与扁平排序(T)产生不同选股序列")
    result_verdict = "肯定" if rd.get("total_displacement", 0) > 0 else "否定"
    print(f"验证结果: {result_verdict}性 — 排序差异{'存在' if rd.get('total_displacement', 0) > 0 else '不存在'}")
    if rd.get("total_displacement", 0) == 0 and comparison.get("tightness_gt_zero", 0) > 0:
        print("注: 排序无差异可能因为 W 权重未改变排名。需 L3 交叉验证不同时段。")
    print("=" * 70)


if __name__ == "__main__":
    main()
