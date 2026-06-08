#!/usr/bin/env python3
"""港股/原油 TV 笔端点回测 v2 — HSI / SHCOMP / BRN 日线 A/B/C/D 组。

与 v1（hk_three_stage_backtest.py）的存在论差异
------------------------------------------------
v1 用 labels 重建逐 bar 收盘序列、lag=1 执行、PH 5-bar 门控。
v2 改为完全以 **TV 笔端点（line.new 端点）** 为本体：

  1. 价格本体     = TV 笔端点序列（si=3, width=2, sol 的 line 端点 union）。
  2. 执行价       = 买卖点标注后的**第 3 个笔端点**价格
                    （跳过标注点 + 2 根确认 + 1 根执行 = 等 3 笔走完再成交）。
  3. 走势方向     = 笔端点序列的 swing 结构（HH/HL/LH/LL）——"用 TV lines 方向判断"。
  4. 仓位管理     = cost_reduction_fsm（267 号满仓满融降成本，杠杆参数化）。
  5. PH settle    = 笔端点序列上的 OnlineMergeTree，τ = 序列 ATR × 1.0。

统一规则（脚本2，A/B 共用）
----------------------------
  - 走势方向用 TV lines 的方向判断（swing 状态机）。
  - 走势方向转空（dir == down）= 退出。
  - 走势方向没转 + 卖点 = 次级别短差（SUB_LEVEL_SELL_POINT）。
  - cost_reduction_fsm 管仓位。

A/B 组
------
  A：纯缠论。dir==down 即退出。
  B：A + PH settle 退出门控。dir==down 时，额外要求在「卖点端点 → 执行端点」窗口内
     出现过一次 persistence ≥ τ 的 settled 下跌结构（PH 因果确认转空）；否则不退出
     （视为假转空，转为次级别短差或继续持有）。

认识论等级：**L2**（真实数据，三标的，单时段；可产生否定性结果）。
形式化有效域：τ=ATR×1.0 与 swing 判据是 L0 运营定义，其经验有效性由本 L2 回测检验，
有效域不超过本三标的单时段——不外推（formalization-validity-domain 规则）。

谱系引用：
  267 号 操作方法论 v1（满仓满融降成本）→ cost_reduction_fsm.py
  §7.5 因果 settle 判据 → a_online_persistence.py OnlineMergeTree

数据源：analysis/data_cache/{hsi,shcomp,brn}_daily_chanlun.json（TV 缓存，无 CDP）。
运行：.venv/bin/python scripts/hk_tv_backtest_v2.py
"""

from __future__ import annotations

import json
import sys
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path

_REPO = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(_REPO / "src"))

from newchan.a_online_persistence import OnlineMergeTree  # noqa: E402
from newchan.trading.cost_reduction_fsm import (  # noqa: E402
    CostReductionFSM,
    CostState,
    FsmEvent,
    FsmEventType,
    IllegalTransitionError,
    transition,
)

# ════════════════════════════════════════════════════════════════
# 配置
# ════════════════════════════════════════════════════════════════

DATA_DIR = _REPO / "analysis" / "data_cache"
OUTPUT_MD = _REPO / "analysis" / "hk_tv_backtest_v2.md"

THIRD_ENDPOINT = 3       # 执行价 = 标注后第 3 个笔端点（跳过 +2 确认 +1 执行）
OWN_CAPITAL = 100_000.0
LEVERAGE = 1.0           # 杠杆（1.0=无融资；满融可线性放大，267 号核心为满仓满融）
SUB_RATIO = 0.3          # 次级别短差仓位比例
FRICTION = 0.0005        # 单次操作摩擦（5bp）
TAU_MULT = 1.0           # PH settle 阈值 = 序列 ATR × TAU_MULT

# 笔层筛选键（si=3 主笔层；零 bar 冲突已验证）
BI_SI = 3
BI_WIDTH = 2
BI_STYLE = "sol"

_BSP_KEYWORDS = ["1买", "2买", "3买", "1卖", "2卖", "3卖"]

SYMBOLS = [
    {"name": "hsi",    "label": "恒生指数",  "yf_ticker": "^HSI"},
    {"name": "shcomp", "label": "上证指数",  "yf_ticker": "000001.SS"},
    {"name": "brn",    "label": "Brent原油", "yf_ticker": "BZ=F"},
]


# ════════════════════════════════════════════════════════════════
# 1. 笔端点序列 + 买卖点定位
# ════════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class Endpoint:
    """笔端点：(bar 索引, 价格)。"""
    bar: int
    price: float


@dataclass(frozen=True, slots=True)
class Bsp:
    """买卖点信号（已定位到端点序列）。"""
    ep_idx: int          # 在笔端点序列中的索引
    bar: int
    pivot_price: float   # 标注处（顶/底分型）价
    side: str            # "buy" / "sell"
    level: int           # 0=主级别(1买1卖), 1=次级别(2/3买卖)
    raw_type: str


def load_data(sym_name: str) -> dict:
    path = DATA_DIR / f"{sym_name}_daily_chanlun.json"
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def build_endpoints(pine: dict) -> list[Endpoint]:
    """从笔层 lines union 出笔端点序列（按 bar 排序，零冲突已验证）。

    取 si=3 / width=2 / sol 的全部 line，收集其两端 (bar,price)。同一 bar 唯一价
    （跨三标的 bars_with_conflict=0 验证），故按 bar 去重安全。
    """
    seg = [
        l for l in pine["lines"]
        if l.get("si") == BI_SI and l.get("width") == BI_WIDTH
        and l.get("style") == BI_STYLE and l.get("bar1") is not None
    ]
    bar_price: dict[int, float] = {}
    for l in seg:
        bar_price.setdefault(int(l["bar1"]), float(l["price1"]))
        bar_price[int(l["bar2"])] = float(l["price2"])  # 端点优先取终点态
    return [Endpoint(b, p) for b, p in sorted(bar_price.items())]


def locate_bsps(pine: dict, eps: list[Endpoint]) -> tuple[list[Bsp], int]:
    """解析买卖点 labels 并定位到笔端点序列。

    定位判据：label.time == endpoint.bar 且 |price 差| < 1e-2（精确落点）。
    无法精确定位的 bsp 被丢弃并计数（数据边界：落在被排除的 width=3/dot 笔层）。
    """
    bar_to_idx = {ep.bar: i for i, ep in enumerate(eps)}
    bsps: list[Bsp] = []
    dropped = 0
    for lbl in pine["labels"]:
        if lbl.get("si") != BI_SI:
            continue
        text = lbl.get("text") or ""
        price = lbl.get("price")
        bar = lbl.get("time")
        if price is None or bar is None:
            continue
        matched = next((k for k in _BSP_KEYWORDS if k in text), None)
        if matched is None:
            continue
        bar = int(bar)
        idx = bar_to_idx.get(bar)
        if idx is None or abs(eps[idx].price - float(price)) >= 1e-2:
            dropped += 1
            continue
        bsps.append(Bsp(
            ep_idx=idx, bar=bar, pivot_price=float(price),
            side="buy" if "买" in matched else "sell",
            level=0 if matched.startswith("1") else 1,
            raw_type=matched,
        ))
    bsps.sort(key=lambda b: (b.ep_idx, b.level))
    return bsps, dropped


def exec_price(bsp: Bsp, eps: list[Endpoint]) -> float:
    """执行价 = 标注端点后第 THIRD_ENDPOINT 个笔端点价（越界用末端点）。"""
    j = min(bsp.ep_idx + THIRD_ENDPOINT, len(eps) - 1)
    return eps[j].price


# ════════════════════════════════════════════════════════════════
# 2. 走势方向 swing 状态机（"用 TV lines 方向判断"）
# ════════════════════════════════════════════════════════════════


def trend_direction_series(eps: list[Endpoint]) -> list[str]:
    """逐端点计算走势方向 dir ∈ {up, down, neutral}（因果，只用 ≤i 端点）。

    笔端点上下交替。端点 i 相对 i-1 上升 → 该端点是 swing 高点；下降 → swing 低点。
      - 新高点 > 上一个高点  ⟹ HH  ⟹ dir = up（走势向上创新高）
      - 新低点 < 上一个低点  ⟹ LL  ⟹ dir = down（走势转空创新低）
      - HL / LH（未破前极值）⟹ dir 保持
    边界条件（可质询）：判据为「严格破前极值」。若改为「破幅 > k·ATR」则 down 信号更稀、
    退出更晚——结论对 k 敏感（formalization-validity-domain：本判据 k=0）。
    """
    dirs: list[str] = ["neutral"] * len(eps)
    cur = "neutral"
    prev_high: float | None = None
    prev_low: float | None = None
    for i in range(1, len(eps)):
        p, prev = eps[i].price, eps[i - 1].price
        if p > prev:  # 端点 i 是 swing 高点
            if prev_high is not None and p > prev_high:
                cur = "up"
            prev_high = p
        elif p < prev:  # 端点 i 是 swing 低点
            if prev_low is not None and p < prev_low:
                cur = "down"
            prev_low = p
        dirs[i] = cur
    return dirs


# ════════════════════════════════════════════════════════════════
# 3. PH settle 时间线（B 组退出门控）
# ════════════════════════════════════════════════════════════════


def series_atr(eps: list[Endpoint]) -> float:
    """笔端点序列波动度量：相邻端点幅度均值 mean(|Δprice|)（ATR 代理）。"""
    diffs = [abs(eps[i + 1].price - eps[i].price) for i in range(len(eps) - 1)]
    return sum(diffs) / len(diffs) if diffs else 0.0


def tau_settle_indices(eps: list[Endpoint], tau: float) -> set[int]:
    """逐端点推进 OnlineMergeTree，记录「该端点 update 后新增 settled 下跌结构
    persistence ≥ τ」的端点索引集合（PH 因果确认转空的时刻）。"""
    tree = OnlineMergeTree()
    idxs: set[int] = set()
    for i, ep in enumerate(eps):
        newly = tree.update(float(ep.price))
        if any(b.persistence >= tau for b in newly):
            idxs.add(i)
    return idxs


def ph_settle_in_window(bsp_idx: int, exec_idx: int, settle_idxs: set[int]) -> bool:
    """窗口 [bsp_idx, exec_idx] 内有 τ-settle 事件。

    B 组（退出门控，越界用法）：卖点窗口有 settle = PH「确认转空」。
    C 组（入场 candidate 过滤，521号正典用法）：买点窗口有 settle = 一个 ≥τ 下跌腿被
    反弹封闭 = 底分型形态前置确认（candidate 形态必要条件，见 `a_settle_trigger`）。
    """
    return any(bsp_idx <= s <= exec_idx for s in settle_idxs)


# ════════════════════════════════════════════════════════════════
# 3b. PH 方向裁决（D 组：零参数 + τ 正则化）
# ════════════════════════════════════════════════════════════════


def ph_direction_series(eps: list[Endpoint]) -> list[str]:
    """PH 方向裁决：零参数，方向翻转 = global dominant 被 settle。

    sublevel tree（原始价格）：追踪谷（下跌结构）。global dominant = 最深谷
    （max alive persistence）。dominant settle = 价格上穿使其死亡 = 全局最大
    上涨走势完成 → direction = short。
    superlevel tree（-price）：追踪峰（上涨结构的镜像）。dominant settle =
    价格下穿使其死亡 = 全局最大下跌走势完成 → direction = long。
    非 dominant 的 settle 不翻转方向。
    """
    sub_tree = OnlineMergeTree(track_dominant=False)
    sup_tree = OnlineMergeTree(track_dominant=False)
    direction = "none"
    directions: list[str] = []

    for i, ep in enumerate(eps):
        # 记录 update 前的 global dominant birth_idx
        sub_dom_idx = (min(sub_tree._stack, key=lambda c: c.val).idx
                       if sub_tree._stack else None)
        sup_dom_idx = (min(sup_tree._stack, key=lambda c: c.val).idx
                       if sup_tree._stack else None)

        sub_settled = sub_tree.update(ep.price)
        sup_settled = sup_tree.update(-ep.price)

        sub_dom_died = sub_dom_idx is not None and any(
            b.birth_idx == sub_dom_idx for b in sub_settled)
        sup_dom_died = sup_dom_idx is not None and any(
            b.birth_idx == sup_dom_idx for b in sup_settled)

        if sup_dom_died:
            direction = "long"
        elif sub_dom_died:
            direction = "short"

        directions.append(direction)

    return directions


# ════════════════════════════════════════════════════════════════
# 3c. 双向交易循环（D 组：PH 方向 + 级别匹配 + 做空）
# ════════════════════════════════════════════════════════════════


def _close_cycle(
    cur: "Cycle",
    fsm: CostReductionFSM,
    idx: int,
    price: float,
    bsp_level: int,
    bsp_type: str,
) -> tuple[CostReductionFSM, float]:
    """关闭当前 cycle，返回 (new_fsm, running_capital)。"""
    cur.close(idx, price)
    cur.exit_bsp_level = bsp_level
    cur.exit_bsp_type = bsp_type
    if fsm.state in (CostState.POSITION_OPEN, CostState.COST_REDUCING):
        cur.deepest_state = _deeper(cur.deepest_state, fsm.state.name)
        fsm = transition(fsm, FsmEvent(FsmEventType.BUY_POINT_NEGATED, price, "main"))
    elif fsm.state == CostState.PRINCIPAL_WITHDRAWN:
        cur.deepest_state = "PRINCIPAL_WITHDRAWN"
        fsm = transition(fsm, FsmEvent(FsmEventType.MAIN_LEVEL_SELL_POINT, price, "main"))
    rc = cur.net_equity
    if fsm.state == CostState.STOPPED_OUT:
        fsm = transition(fsm, FsmEvent(
            FsmEventType.RESET, price, "main",
            new_own_capital=max(rc, 0.0),
        ))
    return fsm, rc


def run_group_bidirectional(
    bsps: list[Bsp],
    eps: list[Endpoint],
    ph_dirs: list[str],
) -> list["Cycle"]:
    """D 组：PH 方向裁决 + 双向交易 + 级别匹配退出。

    方向由 ph_dirs（ph_direction_series）决定。
    long 方向：买点入场做多，卖点降成本/平仓。
    short 方向：卖点入场做空，买点降成本/平空。
    方向翻转 → 清主仓。主级别反向信号 → 清主仓（级别匹配）。
    """
    bsp_at: dict[int, list[Bsp]] = {}
    for b in bsps:
        eidx = b.ep_idx + THIRD_ENDPOINT
        if eidx < len(eps):
            bsp_at.setdefault(eidx, []).append(b)

    fsm = CostReductionFSM.create(
        own_capital=OWN_CAPITAL,
        margin_amount=OWN_CAPITAL * (LEVERAGE - 1.0),
        sub_ratio=SUB_RATIO,
    )
    cycles: list[Cycle] = []
    cur: Cycle | None = None
    running_capital = OWN_CAPITAL
    pos_dir = "none"
    last_ph = "none"

    for i, ep in enumerate(eps):
        ph = ph_dirs[i]

        # ── PH 方向翻转 → 清主仓 ──
        if (cur is not None and ph != last_ph
                and ph != "none" and last_ph != "none"):
            fsm, running_capital = _close_cycle(
                cur, fsm, i, ep.price, -2, f"PH_flip_{ph}")
            cycles.append(cur)
            cur = None
            pos_dir = "none"

        last_ph = ph

        # ── BSP 处理 ──
        for b in bsp_at.get(i, []):
            if ph == "none":
                continue

            is_entry = ((b.side == "buy" and ph == "long")
                        or (b.side == "sell" and ph == "short"))
            is_exit = ((b.side == "sell" and pos_dir == "long")
                       or (b.side == "buy" and pos_dir == "short"))

            # 无持仓 → 入场
            if cur is None or fsm.state == CostState.SCANNING:
                if is_entry and running_capital > 0:
                    d = "long" if b.side == "buy" else "short"
                    fsm = transition(fsm, FsmEvent(
                        FsmEventType.BUY_POINT_CONFIRMED, ep.price, "main"))
                    cur = Cycle(entry_idx=i, entry_price=ep.price,
                                own_capital=running_capital, leverage=LEVERAGE,
                                direction=d)
                    pos_dir = d
                continue

            # 有持仓
            if is_exit and b.level == 0:
                # 主级别反向信号 → 全部清仓
                fsm, running_capital = _close_cycle(
                    cur, fsm, i, ep.price, b.level, b.raw_type)
                cycles.append(cur)
                cur = None
                pos_dir = "none"
            elif is_exit:
                # 次级别反向信号 → 降成本（减仓）
                if fsm.state in (CostState.POSITION_OPEN, CostState.COST_REDUCING,
                                 CostState.PRINCIPAL_WITHDRAWN):
                    try:
                        new_fsm = transition(fsm, FsmEvent(
                            FsmEventType.SUB_LEVEL_SELL_POINT, ep.price, "sub"))
                    except IllegalTransitionError:
                        continue
                    fsm = new_fsm
                    cur.open_short(ep.price)
                    cur.deepest_state = _deeper(cur.deepest_state, "COST_REDUCING")
            elif is_entry and fsm.state == CostState.COST_REDUCING:
                # 同方向信号 + 降成本中 → 补仓
                try:
                    new_fsm = transition(fsm, FsmEvent(
                        FsmEventType.SUB_LEVEL_BUY_POINT, ep.price, "sub"))
                except IllegalTransitionError:
                    continue
                fsm = new_fsm
                cur.close_short(ep.price)
                if fsm.state == CostState.PRINCIPAL_WITHDRAWN:
                    cur.deepest_state = "PRINCIPAL_WITHDRAWN"

    if cur is not None and not cur.closed:
        cur.close(len(eps) - 1, eps[-1].price)
        cycles.append(cur)

    return cycles


# ════════════════════════════════════════════════════════════════
# 4. 仓位账本（FSM 决策 + 现金账本算 PnL）
# ════════════════════════════════════════════════════════════════


@dataclass
class Cycle:
    """一个完整的建仓→退出循环。FSM 决定操作，账本记录现金与持仓。

    direction="long"：买入持有，卖出平仓。sub-cycle = 卖出减仓 → 买回补仓。
    direction="short"：做空持有，买入平空。sub-cycle = 买入减空 → 再卖空补仓。
    现金流方向在 open_sub/close_sub/net_equity 中按 direction 翻转。
    """
    entry_idx: int
    entry_price: float
    own_capital: float
    leverage: float
    direction: str = "long"

    total_shares: float = field(init=False)
    debt: float = field(init=False)
    cash: float = field(init=False)
    active_short_shares: float = 0.0
    short_diff_count: int = 0
    deepest_state: str = "POSITION_OPEN"
    exit_idx: int = -1
    exit_price: float = 0.0
    exit_bsp_level: int = -1
    exit_bsp_type: str = ""

    def __post_init__(self) -> None:
        total_capital = self.own_capital * self.leverage
        self.total_shares = total_capital / self.entry_price
        self.debt = self.own_capital * (self.leverage - 1.0)
        if self.direction == "long":
            self.cash = -total_capital * FRICTION
        else:
            self.cash = total_capital * (1 - FRICTION)

    @property
    def closed(self) -> bool:
        return self.exit_idx >= 0

    def open_short(self, price: float) -> None:
        """减仓：long=卖出部分，short=买回部分空头。"""
        if self.active_short_shares > 0:
            return
        sub = self.total_shares * SUB_RATIO
        self.active_short_shares = sub
        if self.direction == "long":
            self.cash += sub * price * (1 - FRICTION)
        else:
            self.cash -= sub * price * (1 + FRICTION)

    def close_short(self, price: float) -> None:
        """补仓：long=买回部分，short=再卖空部分。"""
        if self.active_short_shares <= 0:
            return
        if self.direction == "long":
            self.cash -= self.active_short_shares * price * (1 + FRICTION)
        else:
            self.cash += self.active_short_shares * price * (1 - FRICTION)
        self.short_diff_count += 1
        self.active_short_shares = 0.0

    def close(self, exit_idx: int, exit_price: float) -> None:
        self.exit_idx = exit_idx
        self.exit_price = exit_price

    @property
    def remaining_shares(self) -> float:
        return self.total_shares - self.active_short_shares

    @property
    def net_equity(self) -> float:
        if self.direction == "long":
            liquidation = self.remaining_shares * self.exit_price * (1 - FRICTION)
            return liquidation + self.cash - self.debt
        cover_cost = self.remaining_shares * self.exit_price * (1 + FRICTION)
        return self.own_capital + self.cash - cover_cost - self.debt

    @property
    def return_mult(self) -> float:
        return self.net_equity / self.own_capital


_STATE_RANK = {"POSITION_OPEN": 0, "COST_REDUCING": 1,
               "PRINCIPAL_WITHDRAWN": 2, "STOPPED_OUT": -1}


def _deeper(a: str, b: str) -> str:
    return a if _STATE_RANK.get(a, 0) >= _STATE_RANK.get(b, 0) else b


# ════════════════════════════════════════════════════════════════
# 5. 主回测循环
# ════════════════════════════════════════════════════════════════


def run_group(
    bsps: list[Bsp],
    eps: list[Endpoint],
    dirs: list[str],
    settle_idxs: set[int] | None,
    ph_mode: str = "none",
    level_matched_exit: bool = False,
) -> list[Cycle]:
    """A/B/C/D 组统一回测。

    ph_mode="none"（A 组，纯缠论）：dir==down 即退出，无 PH。
    ph_mode="exit"（B 组，PH 退出门控，越界用法）：dir==down 额外需窗口内 τ-settle。
    ph_mode="entry"（C 组，PH 入场 candidate 过滤，521号正典用法）：买点建仓需窗口内
        τ-settle（底分型形态前置），退出仍用纯 swing（不用 PH 做退出充分信号）。
    level_matched_exit=True（D 组，级别匹配退出）：仅主级别卖点（level==0，即1卖）
        触发全部清仓；次级别卖点（level==1，即2卖/3卖）无论走势方向如何，均只做降成本
        短差——严格按 267号/338号 cost_reduction_fsm 三阶段执行。

    资金链：running_capital 在循环间复利再投资（退出净值 = 下笔本金），故
    metrics 的连乘 return_mult 是真实复利（非固定资金分批）。
    """
    fsm = CostReductionFSM.create(
        own_capital=OWN_CAPITAL,
        margin_amount=OWN_CAPITAL * (LEVERAGE - 1.0),
        sub_ratio=SUB_RATIO,
    )
    cycles: list[Cycle] = []
    cur: Cycle | None = None
    running_capital = OWN_CAPITAL

    for b in bsps:
        # 执行端点越界（标注后不足 THIRD_ENDPOINT 个笔端点）→ 信号无执行价，跳过
        if b.ep_idx + THIRD_ENDPOINT >= len(eps):
            continue
        eidx = b.ep_idx + THIRD_ENDPOINT
        ep_exec = eps[eidx].price
        d = dirs[eidx]  # 执行端点处的走势方向（等 3 笔后已可观测）

        # ── 买点 ─────────────────────────────────────────────
        if b.side == "buy":
            if fsm.state == CostState.SCANNING:
                # 建仓：走势未转空才进（dir==down 不逆势建仓）
                if d == "down" or running_capital <= 0:
                    continue
                # C 组：PH candidate 形态前置门控——买点窗口需 τ-settle（底分型确认）
                if ph_mode == "entry" and settle_idxs is not None \
                        and not ph_settle_in_window(b.ep_idx, eidx, settle_idxs):
                    continue
                fsm = transition(fsm, FsmEvent(FsmEventType.BUY_POINT_CONFIRMED, ep_exec, "main"))
                cur = Cycle(entry_idx=eidx, entry_price=ep_exec,
                            own_capital=running_capital, leverage=LEVERAGE)
            elif fsm.state == CostState.COST_REDUCING and cur is not None:
                # 次级别买点 → 短差买回（FSM 拒绝时账本不动）
                try:
                    new_fsm = transition(fsm, FsmEvent(FsmEventType.SUB_LEVEL_BUY_POINT, ep_exec, "sub"))
                except IllegalTransitionError:
                    continue
                fsm = new_fsm
                cur.close_short(ep_exec)
                if fsm.state == CostState.PRINCIPAL_WITHDRAWN:
                    cur.deepest_state = "PRINCIPAL_WITHDRAWN"
            continue

        # ── 卖点 ─────────────────────────────────────────────
        if fsm.state == CostState.SCANNING or cur is None:
            continue

        trend_turned_short = (d == "down")
        if ph_mode == "exit" and settle_idxs is not None and trend_turned_short:
            # B 组：PH 必须确认转空，否则降级为「走势没转」
            if not ph_settle_in_window(b.ep_idx, eidx, settle_idxs):
                trend_turned_short = False

        # D组：级别匹配退出——仅主级别卖点（1卖）触发全部清仓
        if level_matched_exit and trend_turned_short and b.level != 0:
            trend_turned_short = False

        if trend_turned_short:
            # 走势转空 → 退出
            cur.close(eidx, ep_exec)
            cur.exit_bsp_level = b.level
            cur.exit_bsp_type = b.raw_type
            if fsm.state in (CostState.POSITION_OPEN, CostState.COST_REDUCING):
                cur.deepest_state = _deeper(cur.deepest_state, fsm.state.name)
                fsm = transition(fsm, FsmEvent(FsmEventType.BUY_POINT_NEGATED, ep_exec, "main"))
            else:  # PRINCIPAL_WITHDRAWN
                cur.deepest_state = "PRINCIPAL_WITHDRAWN"
                fsm = transition(fsm, FsmEvent(FsmEventType.MAIN_LEVEL_SELL_POINT, ep_exec, "main"))
            cycles.append(cur)
            running_capital = cur.net_equity  # 复利再投资链
            cur = None
            if fsm.state == CostState.STOPPED_OUT:
                fsm = transition(fsm, FsmEvent(FsmEventType.RESET, ep_exec, "main",
                                               new_own_capital=max(running_capital, 0.0)))
        else:
            # 走势没转 + 卖点 → 次级别短差（FSM 拒绝时账本不动）
            if fsm.state in (CostState.POSITION_OPEN, CostState.COST_REDUCING,
                             CostState.PRINCIPAL_WITHDRAWN):
                try:
                    new_fsm = transition(fsm, FsmEvent(FsmEventType.SUB_LEVEL_SELL_POINT, ep_exec, "sub"))
                except IllegalTransitionError:
                    continue
                fsm = new_fsm
                cur.open_short(ep_exec)
                cur.deepest_state = _deeper(cur.deepest_state, "COST_REDUCING")

    # 末尾未平仓 → MTM 末端点（已建仓持仓必须平，不受 THIRD_ENDPOINT 截断影响）
    if cur is not None and not cur.closed:
        cur.close(len(eps) - 1, eps[-1].price)
        cycles.append(cur)

    return cycles


# ════════════════════════════════════════════════════════════════
# 6. 指标
# ════════════════════════════════════════════════════════════════


def metrics(cycles: list[Cycle]) -> dict:
    closed = [c for c in cycles if c.closed]
    if not closed:
        return {"n_cycles": 0}
    mults = [c.return_mult for c in closed]
    wins = [m for m in mults if m > 1.0]
    compound, equity = 1.0, [1.0]
    for m in mults:
        compound *= m
        equity.append(compound)
    peak, max_dd = 1.0, 0.0
    for e in equity:
        peak = max(peak, e)
        max_dd = max(max_dd, (peak - e) / peak)
    reached_cr = sum(1 for c in closed if c.deepest_state in ("COST_REDUCING", "PRINCIPAL_WITHDRAWN"))
    reached_pw = sum(1 for c in closed if c.deepest_state == "PRINCIPAL_WITHDRAWN")
    exit_main = sum(1 for c in closed if c.exit_bsp_level == 0)
    exit_sub = sum(1 for c in closed if c.exit_bsp_level == 1)
    exit_mtm = sum(1 for c in closed if c.exit_bsp_level == -1)
    exit_ph_flip = sum(1 for c in closed if c.exit_bsp_level == -2)
    n_long = sum(1 for c in closed if c.direction == "long")
    n_short = sum(1 for c in closed if c.direction == "short")
    return {
        "n_cycles": len(closed),
        "win_rate": len(wins) / len(closed) * 100,
        "compound_return_pct": (compound - 1.0) * 100,
        "avg_ret_pct": sum(m - 1.0 for m in mults) / len(mults) * 100,
        "best_pct": (max(mults) - 1.0) * 100,
        "worst_pct": (min(mults) - 1.0) * 100,
        "max_drawdown_pct": max_dd * 100,
        "reached_cost_reducing": reached_cr,
        "reached_principal_withdrawn": reached_pw,
        "total_short_diffs": sum(c.short_diff_count for c in closed),
        "exits_main_level": exit_main,
        "exits_sub_level": exit_sub,
        "exits_mtm": exit_mtm,
        "exits_ph_flip": exit_ph_flip,
        "n_long": n_long,
        "n_short": n_short,
    }


def buy_hold_pct(eps: list[Endpoint]) -> float:
    """笔端点首→末的持有收益（同源基准，无需外部数据）。"""
    return (eps[-1].price / eps[0].price - 1.0) * 100


# ════════════════════════════════════════════════════════════════
# 7. 报告
# ════════════════════════════════════════════════════════════════


def _pct(v: float, d: int = 1) -> str:
    return "N/A" if v != v else f"{v:+.{d}f}%"


def _fpct(v: float) -> str:
    return "N/A" if v != v else f"{v:.1f}%"


def build_report(results: dict) -> str:
    L: list[str] = []
    a = L.append

    a("# 港股/原油 TV 笔端点回测 v2 — HSI / SHCOMP / BRN 日线 A/B/C/D 组")
    a("")
    a("> 自动生成：`scripts/hk_tv_backtest_v2.py`（无 CDP/MCP，纯 TV 缓存 JSON）")
    a("> 认识论等级：**机械层 L2**（账本/因果真实可验证）；**方法论层 L0/空**"
      "（结论对缠论方法的评价能力为零——见「认识论限制」段，异质质询吸收）")
    a(f"> 执行价：买卖点标注后**第 {THIRD_ENDPOINT} 个笔端点**（跳过 +2 确认 +1 执行）")
    a(f"> 杠杆：{LEVERAGE}×　短差仓位：{SUB_RATIO:.0%}　摩擦：{FRICTION*1e4:.0f}bp/次　τ：序列 ATR×{TAU_MULT}")
    a("")
    a("## 方法论")
    a("")
    a("| 组别 | 系统 | PH settle 用途 |")
    a("|------|------|---------------|")
    a("| **A** | 纯缠论：swing 走势方向转空（dir=down）即退出 | 无 PH |")
    a("| **B** | A + PH 用作**退出充分门控**：dir=down 需窗口内 persistence≥τ settle 才退出 | ⚠️ 越界（L1d：充分性使用） |")
    a("| **C** | A + PH 用作**入场 candidate 过滤**：买点需窗口内 τ-settle（底分型形态前置）才建仓；退出仍用纯 swing | ✓ 正典（521号：形态必要条件） |")
    a("| **D** | PH 方向裁决（零参数）+ 双向交易 + 级别匹配退出：global dominant settle "
      "翻转方向；买/卖点按方向入场；主级别反向信号/PH翻转→全清；次级别→降成本 | PH=方向裁决者（第三层） |")
    a("")
    a("> **D 组设计意图**：PH 从入场过滤（C，已证伪）和退出门控（B，越界）提升为**方向裁决者**——"
      "五层管线第三层。零参数：sublevel global dominant settle = 全局最大上涨走势完成 → "
      "做空方向；superlevel global dominant settle = 全局最大下跌走势完成 → 做多方向。"
      "非 dominant settle 不翻转方向。双向交易：下跌段做空盈利，上涨段做多盈利。级别匹配退出保持。")
    a("")
    a("> **B/C 对照的设计意图**：B 把 PH 用在退出充分层（越界，L1d），C 把 PH 用在入场 candidate "
      "形态必要层（521号正典）。C vs B 经验对照「PH 用途分层」差异——把 L1d 概念诊断转为可验证结果，"
      "回应 Task「candidate+PH settle」字面。**注意**：C 的退出仍是 swing 代理（L1c 标签错配仍在），"
      "故 C 也不是缠论方法证据，只隔离 PH 用途维度。")
    a("")
    a("共用统一规则：走势方向用 TV 笔端点 swing 判断；走势没转 + 卖点 = 次级别短差；"
      "cost_reduction_fsm（267 号满仓满融降成本）管仓位。")
    a("")
    a("> ⚠️ **「走势方向」是 swing 代理，与缠论正典对象否定层级越级**（见边界 0 + 认识论限制 L1c）：")
    a("> 缠论正典中「走势方向」由**走势中枢**严格定义（`zoushi.md` 已结算：走势方向转折="
      "**中枢破坏**=次级别走势否定中枢=第三类买卖点）。本回测按任务规格用 **swing 笔端点"
      "破前极值**（HH/LL）——按 005b 对象否定对象，这是**层级越级**（笔端点层级否定代替走势/"
      "中枢层级否定），不是「弱于」而是**标签错配**：被检验的系统不是缠论系统。故本回测的 A/B "
      "对比**不能作为缠论方法有效性的证据**（评价能力为零，见认识论限制段）。")
    a("")
    a("**笔端点本体**：si=3 / width=2 / sol 笔层 line 端点 union（同 bar 零价格冲突，跨三标的验证）。")
    a("")
    a("---")
    a("")

    # 全局汇总
    a("## 全局汇总")
    a("")
    a("| 标的 | 组 | 循环 | 胜率 | 复利总收益 | 平均/笔 | 最佳 | 最差 | 最大回撤 | 同源B-H |")
    a("|------|----|------|------|----------|--------|------|------|---------|--------|")
    for sym in SYMBOLS:
        r = results[sym["name"]]
        bah = _pct(r["bah"])
        for grp in ("A", "B", "C", "D"):
            m = r[f"metrics_{grp.lower()}"]
            n = m.get("n_cycles", 0)
            wr = _fpct(m.get("win_rate", float("nan")))
            cr = _pct(m.get("compound_return_pct", float("nan")))
            av = _pct(m.get("avg_ret_pct", float("nan")))
            bp = _pct(m.get("best_pct", float("nan")))
            wp = _pct(m.get("worst_pct", float("nan")))
            dd = _fpct(m.get("max_drawdown_pct", float("nan")))
            bh = bah if grp == "A" else "—"
            a(f"| {sym['label']} | **{grp}** | {n} | {wr} | {cr} | {av} | {bp} | {wp} | {dd} | {bh} |")
    a("")
    a("---")
    a("")

    # 核心发现 + 认识论限制（异质质询吸收）
    a("## 认识论限制（异质质询吸收，优先于发现）")
    a("")
    a("> ⚠️ 经异质质询（gemini-challenger 5 裂隙）+ 「candidate」语义溯源（521号/`a_buypoint_score`）"
      "揭示 4 类认识论限制（L1a–d），本回测**对「缠论方法论」的评价能力为零**——不是「评价为无效」，"
      "而是「没有评价能力」。下列「机械观察」可信（账本/因果已验证），但**不可作为缠论方法有效性的证据**。")
    a("")
    a("- **L1a（结论「无 alpha」信息增量为零，不可证伪）**：笔端点是 TV 全历史压缩"
      "（HSI +1667%、BRN +24483% 单调上行——见边界 4），**任何**会退出的择时系统"
      "（均线/RSI/随机数）在此数据上都必然远逊 buy-hold。「缠论择时无 alpha」由数据跨度"
      "必然导出，与缠论信号质量无关 → 不可证伪 → 信息增量为零。前版将其标为「关键否定性"
      "结果」属声明膨胀（090号），此处撤销。")
    a("- **L1b（「B 优于 A」与同义反复共线，无统计意义）**：B 门控只是「少退出」→ 持仓更长"
      "→ 在上行历史中趋近 buy-hold → 自然更好。PH settle 的**真实信息增量与「少动」完全"
      "共线，无法分离**（随机以 p≈0.7 拒绝退出的门控会得到近似结果）。且 B 组仅 4~7 样本，"
      "胜率 CI 无法排除随机。「B 优于 A」不能归因于 PH settle 的因果语义。")
    a("- **L1c（swing 代理与缠论对象否定层级越级 → 主语标签错配）**：走势方向用 swing 笔端点"
      "破前低，而缠论走势方向转折=中枢破坏（次级别走势否定中枢）。按 005b 对象否定对象，"
      "否定不可越级——swing 用笔端点层级否定，**越级**代替走势/中枢层级否定。故被检验的系统"
      "**根本不是缠论系统**，A/B 结论的主语「缠论买卖点择时」标签错配。在补做中枢定义的正典"
      "回测前，本结论不应被引用为缠论方法的证据。")
    a("- **L1d（PH settle 越过形态学→动力学边界，521号）**：B 组把 PH settle 用作退出**充分**"
      "门控，但 521号已结算「纯拓扑因果动量不存在」——PH（纯拓扑）定义上**无法产生**买卖点的"
      "动力学充分确认（MACD 力度背驰，第24课）。PH settle 测的是 persistence≈幅度∈ker(D)，"
      "即**振幅结构衰减/成型**（形态学**必要**条件，candidate 层），**不是力度背驰**（confirmed 层，"
      "见 `a_buypoint_score.py` 521号边界 + `test_bsp_confirmed_semantic`）。故 B 门控「PH 确认转空」"
      "在概念上越用 PH：把形态学候选当成操作充分信号。这与 Task「candidate+PH settle」的正典语义"
      "（PH 绑定 candidate 形态层，不跨 confirmed 动力学层）相悖——B 组的 PH 用法本身是越界的。")
    a("")
    a("## 机械观察（账本/因果可信，非方法论结论）")
    a("")
    a("1. **PH settle 门控把循环数从 21~28 收敛到 4~7**：这是确定的机械效果（门控否决"
      "swing 转空但无 ≥τ settle 的退出）。其收益含义被 L1b 解构——不主张方法论优越。")
    a("2. **次级别短差在单边市逐笔逆势亏损**：BRN B 首循环（上行段）7 次短差 mult<1（卖低买高）；"
      "且 **BRN B 组每笔均收益 -16.3% 差于 A 组 -11.0%、胜率亦更低**——直接**内部证伪**"
      "「门控=更高质量持仓」的机制叙事（裂隙 5）。B 在 BRN 的较少总亏损纯由循环数减少（14 笔"
      "高频亏损被略过）解释，非持仓质量。")
    a("3. **复利链在小样本下路径依赖**：running_capital 滚动乘积在 4~7 样本下，循环次序"
      "（历史偶然）对总收益有决定性影响 → 「复利总收益」是单次路径实现，非期望值估计。")
    # B/C 对照结论（动态生成，把 L1b 诊断转为经验对照）
    rows_bc = []
    for sym in SYMBOLS:
        r = results[sym["name"]]
        rows_bc.append(
            f"{sym['label']} A={_pct(r['metrics_a'].get('compound_return_pct',float('nan')))}"
            f"({r['metrics_a'].get('n_cycles',0)}) / "
            f"C={_pct(r['metrics_c'].get('compound_return_pct',float('nan')))}"
            f"({r['metrics_c'].get('n_cycles',0)}) / "
            f"B={_pct(r['metrics_b'].get('compound_return_pct',float('nan')))}"
            f"({r['metrics_b'].get('n_cycles',0)})")
    a("4. **B/C 对照隔离证实 L1b（关键经验结果）**：把同一 PH settle 分别用在退出层（B，越界）"
      "与入场 candidate 层（C，521号正典），跨三标的：" + "；".join(rows_bc) + "。")
    a("   **C（PH 正典用法）≈ A，远不如 B**——PH 用在 candidate 入场层（不改退出频率）**几乎无收益"
      "效果**；B 的「优于 A」完全来自退出门控降低退出频率（少动 artifact），与 PH 因果语义无关。"
      "这是 L1b 的经验隔离证实，也印证 521号：PH（形态必要条件）不产生操作 alpha。")
    # D vs B 对照（核心实验）
    rows_db = []
    for sym in SYMBOLS:
        r = results[sym["name"]]
        ma_ = r["metrics_a"]
        rows_db.append(
            f"{sym['label']} A={_pct(ma_.get('compound_return_pct',float('nan')))}"
            f"({ma_.get('n_cycles',0)}, 主{ma_.get('exits_main_level',0)}/次{ma_.get('exits_sub_level',0)}) / "
            f"D={_pct(r['metrics_d'].get('compound_return_pct',float('nan')))}"
            f"({r['metrics_d'].get('n_cycles',0)}) / "
            f"B={_pct(r['metrics_b'].get('compound_return_pct',float('nan')))}"
            f"({r['metrics_b'].get('n_cycles',0)})")
    a("5. **A 组退出级别诊断 + D vs B 对照（编排者假说检验）**：A 组退出中，"
      "主级别/次级别/MTM 退出构成——" + "；".join(rows_db) + "。")
    a("   **如果 D ≈ B**：证实「B 组的少退出 = 过滤掉了次级别卖点的越级清仓」假说——"
      "PH 门控无意中实现了级别匹配退出。B 的优势不是 PH 的因果语义，而是级别匹配这个缠论"
      "操盘方法的核心规则。**如果 D ≠ B**：说明 PH 和级别匹配过滤的退出集合不同，"
      "假说被否证或需要更细粒度的分析。")
    a("")
    a("---")
    a("")

    # 逐标的细节
    for sym in SYMBOLS:
        r = results[sym["name"]]
        diag = r["diag"]
        a(f"## {sym['label']}（{r['symbol_tv']}）")
        a("")
        a(f"- 笔端点数：**{diag['n_eps']}**　bar 跨度：{diag['bar_lo']}→{diag['bar_hi']}　"
          f"价格：{diag['price_lo']:.1f}→{diag['price_hi']:.1f}")
        a(f"- 买卖点：**{diag['n_bsp']}** 定位（丢弃 {diag['dropped']} 个非主笔层落点）　"
          f"主级别 {diag['main']} / 次级别 {diag['sub']}")
        a(f"- 序列 ATR：{diag['atr']:.2f}　τ={diag['tau']:.2f}　"
          f"PH τ-settle 事件：{diag['n_settle']}")
        a(f"- 走势方向分布：up {diag['dir_up']} / down {diag['dir_down']} / neutral {diag['dir_neutral']}")
        a("")
        a("| 指标 | A（无PH） | B（PH退出门控） | C（PH入场过滤） | D（级别匹配退出） |")
        a("|------|------|------|------|------|")
        ma, mb, mc, md_ = r["metrics_a"], r["metrics_b"], r["metrics_c"], r["metrics_d"]
        rows = [
            ("循环数", "n_cycles", None),
            ("胜率", "win_rate", "pct"),
            ("复利总收益", "compound_return_pct", "signed"),
            ("平均/笔", "avg_ret_pct", "signed"),
            ("最佳", "best_pct", "signed"),
            ("最差", "worst_pct", "signed"),
            ("最大回撤", "max_drawdown_pct", "pct"),
            ("达降成本", "reached_cost_reducing", None),
            ("达本金退出", "reached_principal_withdrawn", None),
            ("短差总数", "total_short_diffs", None),
            ("退出-主级别", "exits_main_level", None),
            ("退出-次级别", "exits_sub_level", None),
            ("退出-PH翻转", "exits_ph_flip", None),
            ("退出-MTM", "exits_mtm", None),
            ("做多循环", "n_long", None),
            ("做空循环", "n_short", None),
        ]
        for name, key, fmt in rows:
            va, vb, vc, vd = ma.get(key, 0), mb.get(key, 0), mc.get(key, 0), md_.get(key, 0)
            if fmt == "pct":
                sa, sb, sc, sd = _fpct(va), _fpct(vb), _fpct(vc), _fpct(vd)
            elif fmt == "signed":
                sa, sb, sc, sd = _pct(va), _pct(vb), _pct(vc), _pct(vd)
            else:
                sa, sb, sc, sd = str(va), str(vb), str(vc), str(vd)
            a(f"| {name} | {sa} | {sb} | {sc} | {sd} |")
        a("")

    a("---")
    a("")
    a("## 边界条件（可质询）")
    a("")
    a("0. **走势方向代理 = 对象否定层级越级（标签错配，非「有效域过窄」）**：本回测「走势方向转空」"
      "= swing 笔端点破前低。缠论正典走势方向转折 = 中枢破坏（次级别走势否定中枢，第三类买卖点）。"
      "按 005b 对象否定对象（已结算），否定不可越级——swing **越级**用笔端点层级否定，代替走势/"
      "中枢层级否定。故被检验系统**不是缠论系统**，A/B 结论主语「缠论买卖点择时」标签错配。"
      "**这不是有效域过窄，是标签错配**：在补做中枢破坏判据的正典回测前，结论不可引用为缠论方法证据。")
    a("1. **走势方向判据（swing 内部）**：swing 用「严格破前 swing 极值」(k=0)。改为「破幅>k·ATR」→ down 更稀、"
      "退出更晚、回撤更大。结论对 k 敏感。")
    a(f"2. **PH τ**：τ=序列ATR×{TAU_MULT}。τ↑ → settle 事件减少 → B 组退出更少 → 趋同买入持有。")
    a(f"3. **执行价第 {THIRD_ENDPOINT} 端点**：等 3 笔确认。改为第 1/2 端点 → 执行更快、滑点风险换确认度。")
    a("4. **笔端点 bar 非连续日 K**：价格跨度远大于 bar 数所含日历跨度，"
      "说明 TV 笔端点 bar 索引非逐日 K 序号（含更长历史压缩）；本回测以端点为时序本体，"
      "B-H 基准用同源端点首末，不依赖外部日历对齐。")
    a(f"5. **杠杆={LEVERAGE}**：满融（267 号本意）可线性放大盈亏与回撤；本主跑取 1.0 聚焦规则 alpha。")
    a("6. **PH settle 触发于反弹端点（非下跌延续）**：OnlineMergeTree 的 settle 事件在"
      "价格上升越过屏障峰时确认（younger valley 死亡），故 τ-settle 标记的是「一个 ≥τ "
      "的下跌摆动被反弹**封闭**而因果确定」的端点——同构缠论「底分型需后续 K 线确认」"
      "（§7.5 因果 settle 判据）。B 门控语义 = 卖点窗口内存在已因果确认的大下跌结构，"
      "非「价格进一步下行」。这是设计意图（解释 A），有 a_online_persistence 模块定义依据。")
    a("7. **复利再投资链**：每笔以上一笔退出净值为本金（running_capital），故"
      "「复利总收益」是真实滚动净值比；与固定资金分批的差异在 ∏mult 恒等下数值相同，"
      "但概念上本跑为持续操作同一标的（267 号语义），非独立分批。")
    a("")
    a("## 影响声明")
    a("")
    a("- 新增 `scripts/hk_tv_backtest_v2.py`、`analysis/hk_tv_backtest_v2.md`。")
    a("- 不修改任何定义、谱系、既有模块；仅消费 cost_reduction_fsm 与 OnlineMergeTree 的公开 API。")
    a("- 谱系引用：267号（降成本 FSM）、§7.5 因果 settle、005b（对象否定层级）、"
      "521号（纯拓扑无动量→PH 限于形态学候选层）、formalization-validity-domain（有效域规则）。")
    a("- 认识论结论：本回测是「swing 代理 + PH 形态门控 + 降成本 FSM」系统在全历史压缩数据上的"
      "**机械验证**，非缠论方法有效性检验。要使其有方法论评价能力，需（任一选择类方向）："
      "①中枢破坏判据替换 swing；②PH 限用于 candidate 形态过滤（不作退出充分门控）+ MACD 背驰做 confirmed；"
      "③定长均衡窗口数据 + 随机门控对照。")
    return "\n".join(L)


# ════════════════════════════════════════════════════════════════
# 8. 主流程
# ════════════════════════════════════════════════════════════════


def run_symbol(sym: dict) -> dict:
    data = load_data(sym["name"])
    pine = data["pine"]
    eps = build_endpoints(pine)
    bsps, dropped = locate_bsps(pine, eps)
    dirs = trend_direction_series(eps)
    atr = series_atr(eps)
    tau = atr * TAU_MULT
    settle_idxs = tau_settle_indices(eps, tau)

    cyc_a = run_group(bsps, eps, dirs, settle_idxs=None, ph_mode="none")
    cyc_b = run_group(bsps, eps, dirs, settle_idxs=settle_idxs, ph_mode="exit")
    cyc_c = run_group(bsps, eps, dirs, settle_idxs=settle_idxs, ph_mode="entry")
    ph_dirs = ph_direction_series(eps)
    cyc_d = run_group_bidirectional(bsps, eps, ph_dirs)

    diag = {
        "n_eps": len(eps),
        "bar_lo": eps[0].bar, "bar_hi": eps[-1].bar,
        "price_lo": min(e.price for e in eps), "price_hi": max(e.price for e in eps),
        "n_bsp": len(bsps), "dropped": dropped,
        "main": sum(1 for b in bsps if b.level == 0),
        "sub": sum(1 for b in bsps if b.level == 1),
        "atr": atr, "tau": tau, "n_settle": len(settle_idxs),
        "dir_up": dirs.count("up"), "dir_down": dirs.count("down"),
        "dir_neutral": dirs.count("neutral"),
        "ph_long": ph_dirs.count("long"), "ph_short": ph_dirs.count("short"),
        "ph_none": ph_dirs.count("none"),
        "bsp_types": dict(Counter(b.raw_type for b in bsps)),
    }
    return {
        "symbol_tv": data.get("symbol_tv", sym["name"]),
        "diag": diag,
        "metrics_a": metrics(cyc_a),
        "metrics_b": metrics(cyc_b),
        "metrics_c": metrics(cyc_c),
        "metrics_d": metrics(cyc_d),
        "bah": buy_hold_pct(eps),
    }


def main() -> None:
    results = {}
    for sym in SYMBOLS:
        print(f"[{sym['name']}] 回测中…")
        results[sym["name"]] = run_symbol(sym)
        d = results[sym["name"]]["diag"]
        r = results[sym["name"]]
        ma, mb, mc, md = r["metrics_a"], r["metrics_b"], r["metrics_c"], r["metrics_d"]
        print(f"  端点{d['n_eps']} bsp{d['n_bsp']}(丢{d['dropped']}) settle{d['n_settle']} "
              f"| A:{ma.get('n_cycles',0)}笔 {_pct(ma.get('compound_return_pct',float('nan')))} "
              f"(主{ma.get('exits_main_level',0)}/次{ma.get('exits_sub_level',0)})"
              f"| B(PH门控):{mb.get('n_cycles',0)}笔 {_pct(mb.get('compound_return_pct',float('nan')))} "
              f"| C(入场过滤):{mc.get('n_cycles',0)}笔 {_pct(mc.get('compound_return_pct',float('nan')))} "
              f"| D(级别匹配):{md.get('n_cycles',0)}笔 {_pct(md.get('compound_return_pct',float('nan')))}")

    report = build_report(results)
    OUTPUT_MD.write_text(report, encoding="utf-8")
    print(f"\n报告已写入：{OUTPUT_MD}")


if __name__ == "__main__":
    main()
