#!/usr/bin/env python3
"""持仓三阶段联合回测 — QQQ 日线。

把仓库已有的三个独立部件接成一条联合回测管线：

  1. TV 缠论买卖点（analysis/data_cache/qqq_chanlun_labels.json）
     —— 全部 137 个买卖点都在指标实例 si=3 上（si=12 是更粗的中枢/MACD 数值层，
        无买卖点，且 time 轴独立 2..210，不与 si=3 对齐 → 本回测只用 si=3）。
        级别按**缠论买卖点类型**划分（见下）。
  2. 持仓三阶段系统（scripts/position_manager.py，267 号 v1 + 338 号 v2）
     —— Phase.COST_POSITIVE / COST_ZERO / FREE_POSITION 三阶段。
  3. PH settle（src/newchan/a_online_persistence.py，在线因果 merge tree）
     —— trend_health 的否定性必要条件过滤买点。

级别划分（按买卖点类型，缠论依据）
----------------------------------
单一 si=3 指标只给一套买卖点，无现成的双级别。按**缠论买卖点类型**赋级别：

  · 第一类买卖点（1买/1卖）= **操作级别（level 0）**
    1买 = 下跌趋势背驰转折 → 建仓；1卖 = 上涨趋势背驰转折 → 清仓/结构退出。
  · 第二/三类买卖点（2买/3买/2卖/3卖）= **次级别（level 1）**
    级别内的回抽/突破确认结构 → 短差降成本（次级别卖点减仓、次级别买点回补）。

这是缠论的标准语义：一类点定级别转折（操作级别进出），二三类点是级别内运动
（次级别短差）。边界条件见报告（若把 2/3 买当加仓而非短差，映射翻转）。

走势方向自动判断
----------------
不人为指定牛熊：建仓只由一类买点（1买）触发，方向由缠论买卖点类型自身携带
（1买只在下跌背驰后出现 = 走势类型已隐含方向判断）。

价格序列来源（formalization-validity-domain）
---------------------------------------------
PH 的逐 bar 价格序列由 si=3 的全部带价标签**重建**（504 个标签覆盖 time 8..464 的
~97%，前值填充缺口）——与买卖点的 bar 索引天然对齐，就是 TV 图当时用的 QQQ 日线
收盘价（区间 129~742，对应 QQQ ~2018→2026 的真实连续序列，yfinance 可佐证此价区
确为 QQQ 历史）。yfinance 仅作旁证（无法逐 bar 对齐 TV 索引：复权口径 + bar0 日期
未知），不进回测主路径。

认识论等级
----------
- 级别映射（类型→level）、价格重建：L0（从缠论定义 + 数据结构推导）。
- 三组收益对比：L2（真实数据，单标的 QQQ 单时段 ~486 bar，可产生否定性结果）。
- "PH 过滤提升表现"若成立：仅 L2，不外推到其他标的/时段（需 L3 交叉验证）。

谱系引用
--------
- 267 号操作方法论 v1（满仓满融降成本）/ 338 号 v2 修正（强趋势浅回调不加回等）
- a_online_persistence.py §5/§7.5（在线 merge tree + PH 只给否定性必要条件的边界）
- 220 号缠论交易策略体系
"""

from __future__ import annotations

import json
import sys
from collections import Counter
from dataclasses import dataclass, replace
from pathlib import Path

# ── 路径接线：src（newchan 包） + scripts（position_manager） ──
_REPO = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(_REPO / "src"))
sys.path.insert(0, str(_REPO / "scripts"))

import position_manager as pm  # noqa: E402  scripts/position_manager.py
from newchan.a_online_persistence import OnlineMergeTree, TrendHealth  # noqa: E402


# ════════════════════════════════════════════════════════════════
# 配置常量（不入语法的外部参数，显式声明）
# ════════════════════════════════════════════════════════════════

LABELS_PATH = _REPO / "analysis" / "data_cache" / "qqq_chanlun_labels.json"
OUTPUT_MD = _REPO / "analysis" / "three_stage_backtest_qqq.md"

BSP_SI = 3                  # 全部买卖点 + 逐 bar 价格都在 si=3

OWN_CAPITAL = 100_000.0     # 每个循环的自有资金
LEVERAGE = 2.0              # 满仓满融 = 2.0（267 号）
SHORT_DIFF_RATIO = 0.3      # 每层短差动用比例
FRICTION = 0.0005           # 单次操作摩擦成本（佣金+滑点），日线 QQQ 取 5bp

# ── B 组 PH 过滤器的调参旋钮（见 ph_supports_buy 的 trade-off 说明）──
PH_GROWTH_THRESHOLD = 0.0   # 主导下跌分量 persistence 增长率 <= 此值 → 视为衰竭 → 放行买点

# ── 因果执行延迟（关键 — 见模块顶部"前视偏差"说明）──
# TV 缠论买卖点标签是**事后**画在 pivot（局部底/顶）上的；pivot 价不可在当根因果成交。
# 缠论"买卖点需后续 K 线确认"→ 真实成交价 = pivot 后第 EXECUTION_LAG 根的收盘。
# lag=0 = 用 pivot 价 = 前视偏差（100% 胜率假象，仅作对照）；lag>=1 = 因果。
EXECUTION_LAG = 1


# ════════════════════════════════════════════════════════════════
# 1. TV 买卖点解析（按类型赋级别）
# ════════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class TvBsp:
    """一个 TV 缠论买卖点。"""

    bar: int            # bar 索引（= TV label 的 time 字段）
    side: str           # "buy" / "sell"
    kind: str           # "type1" / "type2" / "type3"
    level: int          # 0 = 操作级别(一类点)，1 = 次级别(二/三类点)
    price: float
    raw_type: str       # 原始类型标签（"1买"/"2卖" ...），保留可追溯


_KIND_MAP = {"1": "type1", "2": "type2", "3": "type3"}


def _classify(text: str) -> tuple[str, str] | None:
    """label text → (side, raw_type)。无买卖语义返回 None。

    text 形如 " 2买 0" / " 1卖(盘整) -3"（含 MACD 柱值/盘整趋势后缀）。
    """
    for side, ch in (("buy", "买"), ("sell", "卖")):
        if ch in text:
            for n in ("1", "2", "3"):
                if f"{n}{ch}" in text:
                    return side, f"{n}{ch}"
            return side, ch
    return None


def parse_tv_bsps(labels_path: Path) -> tuple[list[TvBsp], dict]:
    """解析 TV labels（si=3）→ 买卖点列表 + 诊断字典。

    级别由**买卖点类型**决定：一类点(type1)=操作级别(0)，二/三类点=次级别(1)。
    """
    data = json.loads(labels_path.read_text(encoding="utf-8"))
    labels = data["pine"]["labels"]

    bsps: list[TvBsp] = []
    for lbl in labels:
        if lbl.get("si") != BSP_SI:
            continue
        price = lbl.get("price")
        bar = lbl.get("time")
        text = lbl.get("text", "")
        if price is None or bar is None:
            continue
        c = _classify(text)
        if c is None:
            continue
        side, raw_type = c
        kind = "type1"
        for n in ("1", "2", "3"):
            if raw_type.startswith(n):
                kind = _KIND_MAP[n]
                break
        level = 0 if kind == "type1" else 1
        bsps.append(TvBsp(
            bar=int(bar), side=side, kind=kind, level=level,
            price=float(price), raw_type=raw_type,
        ))

    bsps.sort(key=lambda b: (b.bar, b.level))
    diag = {
        "clean_total": len(bsps),
        "bar_min": min((b.bar for b in bsps), default=0),
        "bar_max": max((b.bar for b in bsps), default=0),
        "price_min": round(min((b.price for b in bsps), default=0), 2),
        "price_max": round(max((b.price for b in bsps), default=0), 2),
        "main_level": sum(1 for b in bsps if b.level == 0),
        "sub_level": sum(1 for b in bsps if b.level == 1),
        "type_counts": dict(Counter(b.raw_type for b in bsps)),
    }
    return bsps, diag


# ════════════════════════════════════════════════════════════════
# 2. 逐 bar 价格序列重建（PH 输入，来自 si=3 标签）
# ════════════════════════════════════════════════════════════════


def reconstruct_price_series(labels_path: Path) -> list[float]:
    """从 si=3 全部带价标签重建逐 bar 收盘价序列。

    si=3 覆盖 time 轴 ~97%；缺口前值填充（forward-fill），起始缺口用首个可用值
    回填。序列长度 = max(time)+1，索引与买卖点 bar 对齐。
    """
    data = json.loads(labels_path.read_text(encoding="utf-8"))
    labels = data["pine"]["labels"]

    by_bar: dict[int, float] = {}
    for lbl in labels:
        if lbl.get("si") != BSP_SI:
            continue
        price = lbl.get("price")
        bar = lbl.get("time")
        if price is None or bar is None:
            continue
        by_bar[int(bar)] = float(price)   # 同 bar 多标签取最后（值相同，无差异）

    if not by_bar:
        raise ValueError("si=3 无有效价格标签，无法重建价格序列")

    n = max(by_bar) + 1
    first = by_bar[min(by_bar)]
    series: list[float] = []
    last = first
    for t in range(n):
        if t in by_bar:
            last = by_bar[t]
        series.append(last)
    return series


# ════════════════════════════════════════════════════════════════
# 3. PH settle 层（B 组过滤器）
# ════════════════════════════════════════════════════════════════


def build_ph_health_snapshots(prices: list[float]) -> list[TrendHealth | None]:
    """逐 bar 推进 OnlineMergeTree，记录每根 bar 收盘后的 trend_health 快照。

    返回 health[t]：喂入 prices[0..t] 后的主导 alive 分量健康度（早期 None）。
    """
    tree = OnlineMergeTree()
    snaps: list[TrendHealth | None] = []
    for price in prices:
        tree.update(price)
        snaps.append(tree.trend_health())
    return snaps


def ph_supports_buy(health: TrendHealth | None) -> bool:
    """B 组核心过滤谓词：PH 是否支持在此 bar 建仓（开多）。

    缠论 + PH 语义（a_online_persistence.py 模块顶部 §5 边界）：
      trend_health.persistence_growth_rate > 0  ⟺  主导**下跌**分量仍在创新低
      （幅度扩大、力度未减）⟺ 一类买点大概率仍是**假底/中继** → 应过滤。
      growth_rate <= 0  ⟺  下跌幅度停止扩大（衰竭）→ 反转买点可靠性上升 → 放行。

    这是**否定性必要条件**过滤器：能否定假买点，不能肯定真买点（真假最终判别需
    MACD 力度维度，§5 不可约）。所以 B 组期望相对 A 组**减少假买点入场**，而非
    保证每笔都赢。

    trade-off（PH_GROWTH_THRESHOLD 旋钮）：
      阈值越低（如 -0.5）→ 越严格 → 过滤更多 → 交易更少、单笔质量更高、可能错过反弹。
      阈值越高（如 +0.5）→ 越宽松 → 接近 A 组（不过滤）。
      health 为 None（数据不足）→ 无法判断，**放行**（不因缺数据而拒绝，诚实）。

    边界条件：本谓词假设主导 alive 分量是"下跌腿"（OnlineMergeTree 喂 close 跟踪
    valley）。在单边强势上涨段，alive 分量持续增长是上涨健康而非下跌创新低 —— 此时
    本过滤器语义弱化（见报告边界条件节）。
    """
    if health is None:
        return True
    return health.persistence_growth_rate <= PH_GROWTH_THRESHOLD


# ════════════════════════════════════════════════════════════════
# 4. 杠杆账本（把三阶段 Action 流翻译成权益曲线）
# ════════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class Cycle:
    """一个完整持仓循环的结算结果。"""

    entry_bar: int
    exit_bar: int
    entry_price: float
    exit_price: float
    own_capital: float
    ending_equity: float          # 还掉融资后的自有权益
    return_mult: float            # ending_equity / own_capital
    reached_phase: str            # 该循环到达的最深阶段
    short_diff_count: int
    closed: bool                  # True=结构/主级别卖点平仓，False=序列末 MTM


class LeverageLedger:
    """满仓满融杠杆账本。

    逐 Action 推进现金/持仓/负债；循环平仓（exit_structure/final_exit）时结算权益。
    跨循环用 return_mult 复利（268a：每循环独立核算实际资金）。
    """

    def __init__(self, own_capital: float, leverage: float, friction: float) -> None:
        self._own0 = own_capital
        self._lev = leverage
        self._friction = friction
        self.cycles: list[Cycle] = []
        self._reset_cycle()

    def _reset_cycle(self) -> None:
        self._cash = 0.0
        self._debt = 0.0
        self._holding = 0.0
        self._own = 0.0
        self._entry_bar = -1
        self._entry_price = 0.0
        self._deepest = "idle"
        self._short_diffs = 0
        self._open = False

    @property
    def open(self) -> bool:
        return self._open

    def _fric(self, notional: float) -> float:
        return abs(notional) * self._friction

    def on_action(self, action, position, bar: int, own_capital: float) -> None:
        """处理一个三阶段 Action，推进账本。"""
        a = action.action_type
        price = action.price
        q = action.shares

        if a == "enter":
            self._own = own_capital
            self._debt = own_capital * (self._lev - 1.0)
            invested = own_capital * self._lev
            self._holding = q
            self._cash = own_capital + self._debt - invested - self._fric(invested)
            self._entry_bar = bar
            self._entry_price = price
            self._deepest = "cost_positive"
            self._open = True

        elif a == "short_sell":
            self._cash += q * price - self._fric(q * price)
            self._holding -= q
            self._deepest = "cost_positive"

        elif a == "short_buy":
            self._cash -= q * price + self._fric(q * price)
            self._holding += q
            self._short_diffs += 1

        elif a == "skip_buyback":
            pass  # 338 修正4：强趋势浅回调，无现金动作

        elif a == "extract_capital":
            self._cash += q * price - self._fric(q * price)
            self._holding -= q
            self._deepest = "free_position"

        elif a in ("final_exit", "exit_structure"):
            self._cash += self._holding * price - self._fric(self._holding * price)
            self._holding = 0.0
            self._close_cycle(bar, price, closed=True)

        # hold / unknown：无动作

        ph = getattr(position, "phase", None)
        if ph is not None and ph.value in ("cost_zero", "free_position"):
            if self._deepest != "free_position":
                self._deepest = ph.value

    def _close_cycle(self, bar: int, price: float, closed: bool) -> None:
        if not self._open:
            return
        equity = self._cash - self._debt + self._holding * price
        self.cycles.append(Cycle(
            entry_bar=self._entry_bar,
            exit_bar=bar,
            entry_price=self._entry_price,
            exit_price=price,
            own_capital=self._own,
            ending_equity=equity,
            return_mult=equity / self._own if self._own else 1.0,
            reached_phase=self._deepest,
            short_diff_count=self._short_diffs,
            closed=closed,
        ))
        self._reset_cycle()

    def finalize_mtm(self, last_bar: int, last_price: float) -> None:
        """序列结束：未平仓循环按最后价 mark-to-market 结算。"""
        if self._open:
            self._close_cycle(last_bar, last_price, closed=False)


# ════════════════════════════════════════════════════════════════
# 5. 三阶段驱动（A / B 组共用，B 组多一道 PH 闸门）
# ════════════════════════════════════════════════════════════════


def run_three_stage(
    bsps: list[TvBsp],
    prices: list[float],
    ph_snaps: list[TrendHealth | None] | None,
    lag: int = EXECUTION_LAG,
) -> LeverageLedger:
    """三阶段联合回测主循环。

    Parameters
    ----------
    bsps : 买卖点（按 bar、level 排序）
    prices : 逐 bar 价格序列（最后价用于 MTM）
    ph_snaps : None=A 组（不过滤）；非 None=B 组（PH 过滤建仓）

    驱动规则（267/338 + position_manager 状态机）：
      · IDLE + 一类买点(level0 buy) → 建仓（B 组先过 PH 闸门）
      · COST_POSITIVE + 次级别卖点(level1 sell) → 短差减仓；次级别买点 → 回补/不加回
      · COST_POSITIVE + 一类卖点(level0 sell) → 结构退出（invalidated，满融止损果断）
      · COST_ZERO + 一类卖点 → 退出本金 → FREE_POSITION
      · FREE_POSITION + 一类卖点 → 清仓
    """
    config = pm.PositionConfig(
        own_capital=OWN_CAPITAL,
        leverage_ratio=LEVERAGE,
        short_diff_ratio=SHORT_DIFF_RATIO,
        min_operable_level=2,
        friction_cost=FRICTION,
    )
    ledger = LeverageLedger(OWN_CAPITAL, LEVERAGE, FRICTION)
    position = pm.Position()
    n = len(prices)

    for b in bsps:
        # 因果成交价：pivot 后第 lag 根的收盘（lag>=1 消除前视偏差）
        exec_bar = min(max(b.bar + lag, 0), n - 1)
        exec_price = prices[exec_bar]
        bar = b.bar if 0 <= b.bar < n else min(max(b.bar, 0), n - 1)
        sig = pm.Signal(
            kind=b.kind, side=b.side, level=b.level,
            price=exec_price, timestamp=float(b.bar), confirmed=True,
        )

        # 相位感知的结构退出：COST_POSITIVE 阶段的一类卖点 = 买入程序被否定
        # （满仓满融下还没降到成本=0就操作级别转向 → 结构失败，第13课）。
        if (sig.level == 0 and sig.side == "sell"
                and position.phase == pm.Phase.COST_POSITIVE):
            sig = replace(sig, invalidated=True)

        # B 组 PH 闸门：仅过滤"空仓→建仓"的一类买点
        if (ph_snaps is not None and sig.level == 0 and sig.side == "buy"
                and position.phase == pm.Phase.IDLE):
            health = ph_snaps[bar] if 0 <= bar < len(ph_snaps) else None
            if not ph_supports_buy(health):
                continue  # PH 不支持 → 跳过这个建仓信号

        position, action = pm.process_signal(position, sig, config)
        ledger.on_action(action, position, bar, OWN_CAPITAL)

    ledger.finalize_mtm(n - 1, prices[-1])
    return ledger


# ════════════════════════════════════════════════════════════════
# 6. C 组 baseline（简化版全仓进出，无杠杆/无短差/无三阶段）
# ════════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class BaselineTrade:
    entry_bar: int
    exit_bar: int
    entry_price: float
    exit_price: float
    return_pct: float


def run_baseline(
    bsps: list[TvBsp], prices: list[float], lag: int = EXECUTION_LAG,
) -> list[BaselineTrade]:
    """C 组：任意买点全仓进、任意卖点全仓出，交替。无杠杆、无短差、无级别区分。

    "简化版"= 忽略级别/阶段，把全部 137 个买卖点当成进出开关（最朴素 baseline）。
    成交价同样用因果延迟价 prices[bar+lag]（lag>=1）以消除前视偏差。
    """
    n = len(prices)

    def px(b: TvBsp) -> float:
        return prices[min(max(b.bar + lag, 0), n - 1)]

    trades: list[BaselineTrade] = []
    entry: TvBsp | None = None
    for b in bsps:
        if entry is None and b.side == "buy":
            entry = b
        elif entry is not None and b.side == "sell":
            ep, xp = px(entry), px(b)
            trades.append(BaselineTrade(
                entry_bar=entry.bar, exit_bar=b.bar,
                entry_price=ep, exit_price=xp,
                return_pct=(xp - ep) / ep * 100,
            ))
            entry = None
    return trades


# ════════════════════════════════════════════════════════════════
# 7. 指标计算
# ════════════════════════════════════════════════════════════════


def metrics_three_stage(ledger: LeverageLedger) -> dict:
    cycles = ledger.cycles
    if not cycles:
        return {"n_cycles": 0}
    mults = [c.return_mult for c in cycles]
    compound = 1.0
    equity_curve = []
    for m in mults:
        compound *= m
        equity_curve.append(compound)
    wins = [m for m in mults if m > 1.0]
    peak = 1.0
    max_dd = 0.0
    for e in equity_curve:
        peak = max(peak, e)
        max_dd = max(max_dd, (peak - e) / peak)
    return {
        "n_cycles": len(cycles),
        "win_rate": len(wins) / len(cycles) * 100,
        "compound_return_pct": (compound - 1.0) * 100,
        "avg_cycle_pct": sum((m - 1.0) for m in mults) / len(mults) * 100,
        "best_cycle_pct": (max(mults) - 1.0) * 100,
        "worst_cycle_pct": (min(mults) - 1.0) * 100,
        "max_drawdown_pct": max_dd * 100,
        "reached_cost_zero": sum(1 for c in cycles if c.reached_phase in ("cost_zero", "free_position")),
        "reached_free": sum(1 for c in cycles if c.reached_phase == "free_position"),
        "total_short_diffs": sum(c.short_diff_count for c in cycles),
        "closed_cycles": sum(1 for c in cycles if c.closed),
    }


def metrics_baseline(trades: list[BaselineTrade]) -> dict:
    if not trades:
        return {"n_trades": 0}
    rets = [t.return_pct for t in trades]
    wins = [r for r in rets if r > 0]
    compound = 1.0
    for r in rets:
        compound *= (1 + r / 100)
    return {
        "n_trades": len(trades),
        "win_rate": len(wins) / len(trades) * 100,
        "compound_return_pct": (compound - 1.0) * 100,
        "avg_trade_pct": sum(rets) / len(rets),
        "best_pct": max(rets),
        "worst_pct": min(rets),
    }


# ════════════════════════════════════════════════════════════════
# 8. 报告输出
# ════════════════════════════════════════════════════════════════


def _fmt(d: dict, keys: list[tuple[str, str, str]]) -> str:
    lines = []
    for k, label, fmt in keys:
        if k in d:
            v = d[k]
            lines.append(f"| {label} | {format(v, fmt) if isinstance(v, float) else v} |")
    return "\n".join(lines)


def write_markdown(
    diag: dict,
    m_a: dict, m_b: dict, m_c: dict,
    m_a0: dict, m_c0: dict,
    led_a: LeverageLedger, led_b: LeverageLedger,
    n_bars: int, bh_pct: float, yf_note: str,
) -> None:
    common = [
        ("n_cycles", "完整循环数", "d"),
        ("n_trades", "交易数", "d"),
        ("win_rate", "胜率 %", ".1f"),
        ("compound_return_pct", "复利总收益 %", ".1f"),
        ("avg_cycle_pct", "平均每循环 %", ".2f"),
        ("avg_trade_pct", "平均每笔 %", ".2f"),
        ("best_cycle_pct", "最佳循环 %", ".1f"),
        ("best_pct", "最佳 %", ".1f"),
        ("worst_cycle_pct", "最差循环 %", ".1f"),
        ("worst_pct", "最差 %", ".1f"),
        ("max_drawdown_pct", "循环级最大回撤 %", ".1f"),
        ("reached_cost_zero", "达到成本归零的循环", "d"),
        ("reached_free", "达到零成本持股的循环", "d"),
        ("total_short_diffs", "短差回补总次数", "d"),
    ]

    md = f"""# 持仓三阶段联合回测 — QQQ 日线

> 自动生成：`scripts/three_stage_backtest.py`
> 认识论等级：**L2**（真实数据，单标的 QQQ，单时段 {n_bars} bar）

## 结论（因果执行，lag={EXECUTION_LAG}）

| 组别 | 系统 | 杠杆 | 完整循环/交易 | 胜率 | 复利总收益 |
|------|------|------|--------------|------|-----------|
| 基准 | QQQ buy-and-hold（窗口首末） | 1.0 | — | — | {bh_pct:+.1f}% |
| **A** | TV买卖点 + 三阶段持仓 | {LEVERAGE} | {m_a.get('n_cycles', 0)} | {m_a.get('win_rate', 0):.1f}% | **{m_a.get('compound_return_pct', 0):.1f}%** |
| **B** | A + PH settle 过滤 | {LEVERAGE} | {m_b.get('n_cycles', 0)} | {m_b.get('win_rate', 0):.1f}% | **{m_b.get('compound_return_pct', 0):.1f}%** |
| **C** | 简化版全仓进出 (baseline) | 1.0 | {m_c.get('n_trades', 0)} | {m_c.get('win_rate', 0):.1f}% | **{m_c.get('compound_return_pct', 0):.1f}%** |

## ⚠️ 前视偏差警示（最重要的方法论发现 — no-workaround）

TV 缠论指标的买卖点标签是**事后**画在 pivot（局部底/顶）上的：1买画在底、1卖画在顶。
若直接用 **pivot 价**进出（lag=0），等于"在每个事后确认的局部底买入、局部顶卖出"
→ **必然接近 100% 胜率、收益爆表**，这是前视偏差，不是策略有效性：

| 执行口径 | A 组胜率 | A 组收益 | C 组胜率 | C 组收益 |
|----------|---------|---------|---------|---------|
| **lag=0（pivot 价，前视偏差❌）** | {m_a0.get('win_rate', 0):.0f}% | {m_a0.get('compound_return_pct', 0):+.0f}% | {m_c0.get('win_rate', 0):.0f}% | {m_c0.get('compound_return_pct', 0):+.0f}% |
| **lag={EXECUTION_LAG}（确认后成交，因果✓）** | {m_a.get('win_rate', 0):.0f}% | {m_a.get('compound_return_pct', 0):+.0f}% | {m_c.get('win_rate', 0):.0f}% | {m_c.get('compound_return_pct', 0):+.0f}% |

缠论原文（第27/37课）：买卖点需**后续 K 线确认**才成立——pivot 当根不可因果成交。
故上表 lag=0 行**不可作为业绩**，仅证明"用事后标签的 pivot 价回测必假"。本报告
**头部结论表全部采用 lag={EXECUTION_LAG} 的因果口径**。这与 `settle_backtest.py` 的"右侧确认"
和 a_online_persistence §10"分型需后续确认"完全同构。

> 即便 lag={EXECUTION_LAG}，因数据是 TV 指标事后标定的 pivot 邻域，仍可能残留乐观偏差
> （pivot+1 根通常离极值很近）。要彻底因果，需用指标的**实时确认时刻**而非 pivot
> 时刻，但导出数据未携带确认时刻 → 这是数据层的有效域上限（L2，已标注）。

## 关键限制（诚实声明 — no-patch-mentality）

**A/B 两组在本窗口的阶段推进**：达到成本归零 {m_a.get('reached_cost_zero', 0)} 次、
达到零成本持股 {m_a.get('reached_free', 0)} 次（A 组）。position_manager 的阶段推进条件是
「累计短差利润 ≥ 自有资金」；短差只动用 {SHORT_DIFF_RATIO:.0%} 仓位，若本窗口次级别信号
不足以让累计短差利润回收 100% 自有资金，则循环停留在 COST_POSITIVE，**实际测的是
「满仓满融做多 + 次级别短差」叠加，而非完整三阶段生命周期**。这是数据/时段的客观
约束，不是实现缺陷——代码逻辑正确，只是触发条件是否满足取决于行情。具体推进次数见
上方与下方 A 组指标。

## 数据依据

**信号源**：TV 缠论指标导出（`analysis/data_cache/qqq_chanlun_labels.json`）。
全部 **{diag['clean_total']} 个买卖点都在指标实例 si=3**（si=12 是更粗的中枢/MACD 数值层，
无买卖点且 time 轴独立，本回测不使用）。

**级别按缠论买卖点类型划分**（非按 si）：

| 级别 | 类型 | 数量 | 角色 |
|------|------|------|------|
| 操作级别 (level 0) | 一类点 1买/1卖 | {diag['main_level']} | 建仓 / 退出本金 / 清仓 / 结构退出 |
| 次级别 (level 1) | 二三类点 2买/3买/2卖/3卖 | {diag['sub_level']} | 短差降成本 |

类型分布：`{diag['type_counts']}`

**价格序列**：由 si=3 全部带价标签重建（逐 bar，前值填充），长度 {n_bars} bar，
区间 {diag['price_min']}~{diag['price_max']}（QQQ ~2018→2026 真实连续序列）。这是 TV 图当时
用的 QQQ 日线收盘价，与买卖点 bar 索引天然对齐。

**走势方向自动判断**：不人为指定牛熊。建仓只由一类买点（1买，下跌背驰转折）触发，
方向由缠论买卖点类型自身携带。

## 三阶段系统定义依据

引擎 = `scripts/position_manager.py`（267 号 v1 + 338 号 v2），三阶段：

| Phase | 触发 | 动作 |
|-------|------|------|
| COST_POSITIVE | 一类买点（IDLE→） | 满仓满融建仓 + 次级别短差降成本 |
| COST_ZERO | 累计短差回收 ≥ 自有资金 | 一类卖点退出本金（1x 乘数） |
| FREE_POSITION | 本金已退出 | 零成本持股，次级别全额回补收集 |

阶段推进统计（A 组）：达到成本归零 {m_a.get('reached_cost_zero', 0)} 次，
达到零成本持股 {m_a.get('reached_free', 0)} 次，短差回补 {m_a.get('total_short_diffs', 0)} 次。

## A / B / C 详细指标

### A 组：TV + 三阶段（满仓满融）
| 指标 | 值 |
|------|-----|
{_fmt(m_a, common)}

### B 组：A + PH settle 过滤
| 指标 | 值 |
|------|-----|
{_fmt(m_b, common)}

PH 过滤器：`ph_supports_buy` —— 主导下跌分量 `persistence_growth_rate <= {PH_GROWTH_THRESHOLD}`
（下跌幅度停止扩大=衰竭）时放行买点；仍在创新低（growth>0）时过滤（假底/中继）。
A 组建仓 {len(led_a.cycles)} 循环 → B 组 {len(led_b.cycles)} 循环，
PH 过滤掉 {max(0, len(led_a.cycles) - len(led_b.cycles))} 个潜在建仓（净效果见上表）。

### C 组：简化版全仓进出（baseline）
| 指标 | 值 |
|------|-----|
{_fmt(m_c, common)}

## 边界条件（结论翻转的条件）

1. **级别映射规则**：本回测设 一类点=操作级别、二三类点=次级别（缠论标准语义：一类点
   定级别转折，二三类点是级别内运动）。若改为"二三类点也作加仓而非短差"，建仓/短差
   角色变化，A/B 结果翻转。这是最关键的可质询点。
2. **PH 阈值**：PH_GROWTH_THRESHOLD={PH_GROWTH_THRESHOLD}。调低→更严格→B 组交易更少；
   调高→趋近 A 组。B 组优劣对此参数敏感。
3. **PH 方向语义**：`ph_supports_buy` 把主导 alive 分量当"下跌腿"。QQQ 此窗口是长牛，
   多数时间 alive 分量增长来自上涨而非下跌创新低 → 过滤器语义在强势段弱化（可能误杀
   或形同虚设）。这是 §5"PH 只给幅度必要条件"的体现。
4. **杠杆与摩擦**：A/B leverage={LEVERAGE}、friction={FRICTION}。满融放大双向波动，摩擦
   侵蚀短差利润。leverage→1 则 A/B 趋近 C 组口径。
5. **价格序列来源**：PH 用 si=3 标签重建价（非 yfinance）。若改用 yfinance 且 bar 对齐
   错误，PH 的 settle 时刻全部错位 → B 组失真。

## 下游推论

- 若 B 组（PH 过滤）复利收益 > A 组：PH 的否定性必要条件过滤器在 QQQ 单时段上**确实
  剔除了部分假买点**（L2 确认性，未否证）；但**不能外推**到其他标的/时段（需 L3）。
- 若 B 组 ≤ A 组：PH 过滤在此长牛时段**过度保守**（误杀真买点）或方向语义错配——
  与 §5/边界条件 3 一致，是**否定性结果**（更有价值，缩小了 PH 过滤器有效域边界）。
- 三阶段（A/B）相对全仓（C）：若 A/B 在杠杆下仍跑赢 C，说明降成本+免费仓位结构提供了
  正的风险调整收益；若跑输，说明满融在此波段放大了回撤、且阶段未推进时仅等价于杠杆多头。

## 谱系引用

- 267 号操作方法论 v1 / 338 号 v2（三阶段降成本，`position_manager.py` 实现）
- a_online_persistence.py §5/§7.5（PH 在线 merge tree + 否定性必要条件边界）
- 220 号缠论交易策略体系
- 形式化有效域规则：本结论 L2，未交叉验证（L3），有效域 = QQQ 此 {n_bars}-bar 窗口

## 影响声明

- 新增 `scripts/three_stage_backtest.py`、`analysis/three_stage_backtest_qqq.md`
- 未改动任何已有模块（只读消费 position_manager / a_online_persistence / TV labels）

## yfinance 旁证（cross-check，不进回测主路径）

{yf_note}
"""
    OUTPUT_MD.write_text(md, encoding="utf-8")


# ════════════════════════════════════════════════════════════════
# 9. yfinance 旁证
# ════════════════════════════════════════════════════════════════


def yfinance_crosscheck(prices: list[float]) -> str:
    """拉 yfinance QQQ 日线，给出与 TV 标签价区间的对照（说明为何不用它对齐）。"""
    try:
        import yfinance as yf
        hist = yf.Ticker("QQQ").history(period="max", interval="1d")
        closes = [float(c) for c in hist["Close"].tolist()]
        if not closes:
            return "yfinance 返回空数据，跳过 cross-check。"
        tv_lo, tv_hi = min(prices), max(prices)
        in_band = [(str(hist.index[i].date()), closes[i])
                   for i in range(len(closes)) if tv_lo <= closes[i] <= tv_hi]
        span = (f"{in_band[0][0]} .. {in_band[-1][0]}（{len(in_band)} 个 yf bar）"
                if in_band else "无")
        return (
            f"- TV 标签价区间：{tv_lo:.2f} ~ {tv_hi:.2f}（{len(prices)} bar）\n"
            f"- yfinance QQQ 全历史：{len(closes)} bar，区间 {min(closes):.1f} ~ {max(closes):.1f}\n"
            f"- yfinance 落入 TV 价区间的时段：{span}\n"
            f"- **结论**：TV 价区间确为 QQQ 真实历史价（yfinance 佐证），但 yfinance 落入该区间的"
            f" bar 数与 TV 的 {len(prices)} bar 不一一对应（复权口径 + bar0 日期未知），无法逐 bar"
            f" 对齐 → PH 价格序列改用 si=3 标签重建（与买卖点 bar 天然对齐）。"
        )
    except Exception as e:  # noqa: BLE001  cross-check 失败不应中断回测
        return f"yfinance cross-check 失败（{type(e).__name__}: {e}），不影响主回测。"


# ════════════════════════════════════════════════════════════════
# main
# ════════════════════════════════════════════════════════════════


def main() -> None:
    print("=== 持仓三阶段联合回测 QQQ ===\n")

    bsps, diag = parse_tv_bsps(LABELS_PATH)
    print(f"买卖点 {diag['clean_total']} 个（操作级别/一类 {diag['main_level']} / "
          f"次级别/二三类 {diag['sub_level']}），bar {diag['bar_min']}..{diag['bar_max']}")

    prices = reconstruct_price_series(LABELS_PATH)
    n_bars = len(prices)
    print(f"重建逐 bar 价格序列 {n_bars} 根，区间 {min(prices):.2f}~{max(prices):.2f}\n")

    ph_snaps = build_ph_health_snapshots(prices)

    # 因果执行（lag>=1，主结果）
    led_a = run_three_stage(bsps, prices, ph_snaps=None, lag=EXECUTION_LAG)
    led_b = run_three_stage(bsps, prices, ph_snaps=ph_snaps, lag=EXECUTION_LAG)
    trades_c = run_baseline(bsps, prices, lag=EXECUTION_LAG)
    m_a = metrics_three_stage(led_a)
    m_b = metrics_three_stage(led_b)
    m_c = metrics_baseline(trades_c)

    # 前视偏差对照（lag=0，pivot 价直接成交 → 必然近 100% 胜率，仅作警示）
    m_a0 = metrics_three_stage(run_three_stage(bsps, prices, ph_snaps=None, lag=0))
    m_c0 = metrics_baseline(run_baseline(bsps, prices, lag=0))

    # buy-and-hold 基准（窗口首末价，无杠杆）
    bh_pct = (prices[-1] / prices[0] - 1.0) * 100

    def show(name: str, m: dict) -> None:
        print(f"--- {name} ---")
        for k, v in m.items():
            print(f"  {k}: {v:.2f}" if isinstance(v, float) else f"  {k}: {v}")
        print()

    print(f"[基准] QQQ buy-and-hold（窗口首末，无杠杆）: {bh_pct:+.1f}%\n")
    show(f"A 组 TV+三阶段 (因果 lag={EXECUTION_LAG})", m_a)
    show(f"B 组 +PH过滤 (因果 lag={EXECUTION_LAG})", m_b)
    show(f"C 组 baseline全仓 (因果 lag={EXECUTION_LAG})", m_c)
    print(f"[前视偏差对照 lag=0] A 胜率{m_a0.get('win_rate', 0):.0f}% "
          f"收益{m_a0.get('compound_return_pct', 0):+.0f}% | "
          f"C 胜率{m_c0.get('win_rate', 0):.0f}% 收益{m_c0.get('compound_return_pct', 0):+.0f}%\n")

    yf_note = yfinance_crosscheck(prices)
    write_markdown(diag, m_a, m_b, m_c, m_a0, m_c0, led_a, led_b,
                   n_bars, bh_pct, yf_note)
    print(f"报告已写入 {OUTPUT_MD.relative_to(_REPO)}")


if __name__ == "__main__":
    main()
