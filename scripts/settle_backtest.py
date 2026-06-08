"""QQQ settle 阶梯（PH 右侧确认）× 缠论中枢过滤 —— 历史在线回测。

存在论位置
----------
本脚本把两条互补的视角合在一根 K 线流上做**因果在线回测**：

- **PH 右侧视角（settle 阶梯）**：`OnlineMergeTree` 的 alive→settled 跃迁是缠论
  "分型需后续 K 线确认"的拓扑形式（a_online_persistence 模块顶部已证 L0 定理）。
  一段下跌腿 settle = 底被反弹涨过屏障**因果确认** = 右侧买入信号。
  一段上涨腿 settle（喂 -close 的对称树）= 顶被跌破屏障确认 = 右侧卖出信号。

- **缠论左侧/结构视角（中枢）**：detect_zhongshu 在截至当下的 merge bars 上算中枢数。
  "中枢 ≥ 2" ≈ 趋势成立（缠论：两个同级别中枢之间是趋势）。作为买入的**合取过滤器**。

信号的对称来源（关键，必读）
----------------------------
OnlineMergeTree 只能"看见" valley（sublevel-set H0）。要对称地拿到买/卖两个方向：
  t_buy  = OnlineMergeTree()  喂  close  → 下跌腿 settle = 底确认 = 买
  t_sell = OnlineMergeTree()  喂 -close  → 上涨腿 settle = 顶确认 = 卖
取负是把"顶"翻译进 sublevel 语言的唯一手段（与 a_dual_merge_tree 同一原理，但这里用
close 而非 high/low —— QQQ 用 close 避免日内 HL 聚合噪声，与 settle 阶梯模板一致）。

`update(price)` **直接返回本次新 settle 的分量**，天然就是 "alive→settled 的那一刻"，
无需自己 diff 快照 —— 这就是 PH 右侧确认的因果时刻。

策略矩阵（用户裁定：两种离场都跑对比）
--------------------------------------
离场规则 {对称settle, 反向腿否定} × 缠论过滤 {无, 中枢≥2} = 4 变体 + buy-and-hold：
  S1 对称settle，无过滤        S2 对称settle，中枢≥2
  S3 反向腿否定出，无过滤      S4 反向腿否定出，中枢≥2
- 对称settle：买入信号进，卖出信号（上涨腿 settle = 顶确认）出。纯 PH 右侧对称。
- 反向腿否定：买入信号进，stop_signal 的 reverse_exceeds 出（入场后新生下跌腿
  persistence 超过入场确认底幅 = "对象否定对象"，005b号）。

认识论等级（formalization-validity-domain）
-------------------------------------------
- PH settle / merge tree / stop_signal：**L0**（确定性算法）。
- "settle = 分型确认"、"中枢≥2 = 趋势"：**L0 算法 + 候选同构**。
- 单标的（QQQ）历史回测胜率/收益：**L2**（可否证 —— 这是本脚本的信息增量所在；
  否定性结果，如"过滤器无效"或"否定出过早止损跑输 buy-hold"，比确认更有价值）。
- 跨标的稳健性（多 ETF/多市场）：**L3，未做**（本脚本严格限定 QQQ 单标的）。

诚实约束（no-patch-mentality）
------------------------------
- buy-and-hold 全期 vs 策略：策略只在部分时间持仓，故必报"市场暴露占比"使对比公平。
- 未平仓单（最后一根仍持仓）：单列浮盈，不计入已实现胜率，不强行造一笔假平仓。
- 最大回撤用 **mark-to-market 逐日权益**（持仓期间用 close 估值），非仅交易点。
"""
from __future__ import annotations

import json
import math
import sys
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

import yfinance as yf  # noqa: E402

from newchan.a_persistence_barcode import atr_noise_threshold  # noqa: E402
from newchan.a_online_persistence import OnlineMergeTree, EntryState  # noqa: E402
from newchan.a_ph_zhongshu import detect_zhongshu, ZhongshuPolicy  # noqa: E402

CACHE = ROOT / "analysis" / "data_cache"
OUT_MD = ROOT / "analysis" / "settle_backtest_qqq.md"
SYMBOL = "QQQ"
PERIOD = "max"


# ====================================================================
# 数据拉取（复用 brn 模板的 NaN 剔除逻辑）
# ====================================================================


def pull(symbol: str, period: str, interval: str = "1d") -> dict:
    df = yf.download(symbol, period=period, interval=interval,
                     auto_adjust=False, progress=False)
    if df.empty:
        raise SystemExit(f"yfinance 返回空数据: {symbol}")

    def col(name: str):
        if (name,) in df.columns or name in df.columns:
            s = df[name]
        else:
            s = df.xs(name, axis=1, level=0)
        return [float(x) for x in s.to_numpy().ravel()]

    raw = {
        "closes": col("Close"), "highs": col("High"),
        "lows": col("Low"), "opens": col("Open"),
        "dates": [d.strftime("%Y-%m-%d") for d in df.index],
    }
    keep = [i for i in range(len(raw["closes"]))
            if not any(math.isnan(raw[k][i]) for k in ("closes", "highs", "lows", "opens"))]
    return {
        "symbol": symbol,
        "closes": [raw["closes"][i] for i in keep],
        "highs": [raw["highs"][i] for i in keep],
        "lows": [raw["lows"][i] for i in keep],
        "opens": [raw["opens"][i] for i in keep],
        "dates": [raw["dates"][i] for i in keep],
    }


# ====================================================================
# 单个策略变体的因果状态机（long-only）
# ====================================================================


@dataclass
class Strategy:
    """一个回测变体的持仓状态机 + 已实现/逐日权益记录。

    exit_rule : 'symmetric'（卖出信号=上涨腿settle出）| 'negate'（反向下跌腿否定出）
    use_filter: True → 仅在买入信号 + 中枢数≥2 时进场
    """

    name: str
    exit_rule: str
    use_filter: bool
    pos: dict | None = None                 # 持仓快照；None=空仓
    trades: list[dict] = field(default_factory=list)
    realized_equity: float = 1.0            # 已平仓复利权益
    curve: list[float] = field(default_factory=list)  # 逐日 mark-to-market 权益
    days_in_market: int = 0

    def step(self, i: int, closes: list[float], dates: list[str],
             buy_sig: bool, sell_sig: bool, entry_pers: float,
             zc: int | None, t_buy: OnlineMergeTree) -> None:
        """推进一根 K 线：先处理进/出，再记 mark-to-market 权益。"""
        if self.pos is None:
            ok = buy_sig and (not self.use_filter or (zc is not None and zc >= 2))
            if ok:
                self.pos = {
                    "entry_idx": i,
                    "entry_close": closes[i],
                    "entry_date": dates[i],
                    "entry_state": EntryState(
                        birth_idx=i, entry_persistence=entry_pers,
                        direction=1, n_at_entry=i + 1),
                }
        else:
            if self.exit_rule == "symmetric":
                exit_now = sell_sig
            else:  # negate：只取 reverse_exceeds（入场后新生下跌腿超入场确认底幅）
                ss = t_buy.stop_signal(self.pos["entry_state"])
                exit_now = ss.reason in ("reverse_exceeds", "both")
            if exit_now:
                self._close(i, closes, dates)

        if self.pos is not None:
            self.days_in_market += 1
            eq = self.realized_equity * closes[i] / self.pos["entry_close"]
        else:
            eq = self.realized_equity
        self.curve.append(eq)

    def _close(self, i: int, closes: list[float], dates: list[str]) -> None:
        p = self.pos
        assert p is not None
        ret = closes[i] / p["entry_close"] - 1.0
        self.realized_equity *= (1.0 + ret)
        self.trades.append({
            "entry_date": p["entry_date"], "exit_date": dates[i],
            "entry": round(p["entry_close"], 2), "exit": round(closes[i], 2),
            "ret_pct": round(ret * 100, 2),
            "days": i - p["entry_idx"],
        })
        self.pos = None

    # ---- 收尾指标 ----
    def metrics(self, closes: list[float], dates: list[str]) -> dict:
        wins = [t for t in self.trades if t["ret_pct"] > 0]
        n = len(self.trades)
        win_rate = (len(wins) / n * 100) if n else 0.0
        avg_ret = (sum(t["ret_pct"] for t in self.trades) / n) if n else 0.0
        avg_days = (sum(t["days"] for t in self.trades) / n) if n else 0.0
        avg_win = (sum(t["ret_pct"] for t in wins) / len(wins)) if wins else 0.0
        losses = [t for t in self.trades if t["ret_pct"] <= 0]
        avg_loss = (sum(t["ret_pct"] for t in losses) / len(losses)) if losses else 0.0
        max_dd = _max_drawdown(self.curve)
        # 未平仓浮盈（最后一根仍持仓）
        open_pnl = None
        if self.pos is not None:
            open_pnl = {
                "entry_date": self.pos["entry_date"],
                "entry": round(self.pos["entry_close"], 2),
                "last": round(closes[-1], 2),
                "float_ret_pct": round((closes[-1] / self.pos["entry_close"] - 1) * 100, 2),
            }
        total_days = len(closes)
        return {
            "name": self.name, "exit_rule": self.exit_rule, "use_filter": self.use_filter,
            "n_trades": n, "win_rate_pct": round(win_rate, 1),
            "avg_ret_pct": round(avg_ret, 2),
            "avg_win_pct": round(avg_win, 2), "avg_loss_pct": round(avg_loss, 2),
            "avg_hold_days": round(avg_days, 1),
            "final_equity": round(self.realized_equity, 4),
            "total_ret_pct": round((self.realized_equity - 1) * 100, 1),
            "max_drawdown_pct": round(max_dd * 100, 1),
            "market_exposure_pct": round(self.days_in_market / total_days * 100, 1),
            "open_position": open_pnl,
            "trades": self.trades,
        }


def _max_drawdown(curve: list[float]) -> float:
    """逐日权益曲线的最大回撤（正数，0.2 = -20%）。"""
    peak = float("-inf")
    mdd = 0.0
    for v in curve:
        peak = max(peak, v)
        if peak > 0:
            mdd = max(mdd, (peak - v) / peak)
    return mdd


# ====================================================================
# 回测主循环（一对树驱动 4 个策略，中枢按需缓存）
# ====================================================================


def run_backtest(data: dict) -> dict:
    closes, highs, lows, dates = data["closes"], data["highs"], data["lows"], data["dates"]
    n = len(closes)
    tau = atr_noise_threshold(highs, lows, closes, period=14, multiple=1.0)

    t_buy = OnlineMergeTree()    # 喂 close：下跌腿 settle = 买
    t_sell = OnlineMergeTree()   # 喂 -close：上涨腿 settle = 卖

    strategies = [
        Strategy("S1 对称settle·无过滤", "symmetric", False),
        Strategy("S2 对称settle·中枢≥2", "symmetric", True),
        Strategy("S3 否定出·无过滤", "negate", False),
        Strategy("S4 否定出·中枢≥2", "negate", True),
    ]

    zs_policy = ZhongshuPolicy(min_members=3, same_level_ratio=3.0, require_overlap=True)
    zc_cache: dict[int, int] = {}

    def zhongshu_count_at(i: int) -> int:
        if i in zc_cache:
            return zc_cache[i]
        tmp = OnlineMergeTree()
        for p in closes[: i + 1]:
            tmp.update(p)
        final = tmp.finalize()
        zc = len(detect_zhongshu(list(final.all_bars), bars_per_day=1.0,
                                 noise_floor=tau, policy=zs_policy))
        zc_cache[i] = zc
        return zc

    n_buy_signals = 0
    n_sell_signals = 0
    for i in range(n):
        newly_buy = t_buy.update(closes[i])
        newly_sell = t_sell.update(-closes[i])
        buy_legs = [b for b in newly_buy if b.persistence >= tau]
        sell_legs = [b for b in newly_sell if b.persistence >= tau]
        buy_sig = bool(buy_legs)
        sell_sig = bool(sell_legs)
        entry_pers = max((b.persistence for b in buy_legs), default=tau)
        if buy_sig:
            n_buy_signals += 1
        if sell_sig:
            n_sell_signals += 1

        # 中枢过滤只在买入信号根按需算一次（4 策略共享）
        zc = zhongshu_count_at(i) if buy_sig else None

        for s in strategies:
            s.step(i, closes, dates, buy_sig, sell_sig, entry_pers, zc, t_buy)

    # buy-and-hold 基准（全期）
    bh_ret = closes[-1] / closes[0] - 1.0
    bh_curve = [c / closes[0] for c in closes]
    bh = {
        "total_ret_pct": round(bh_ret * 100, 1),
        "max_drawdown_pct": round(_max_drawdown(bh_curve) * 100, 1),
        "annualized_pct": round(((closes[-1] / closes[0]) ** (252.0 / n) - 1) * 100, 1),
    }

    return {
        "meta": {
            "symbol": data["symbol"], "n_bars": n,
            "date_start": dates[0], "date_end": dates[-1],
            "price_start": round(closes[0], 2), "price_end": round(closes[-1], 2),
            "price_min": round(min(closes), 2), "price_max": round(max(closes), 2),
            "tau_atr": round(tau, 4),
            "n_buy_signals": n_buy_signals, "n_sell_signals": n_sell_signals,
        },
        "buy_and_hold": bh,
        "strategies": [s.metrics(closes, dates) for s in strategies],
    }


# ====================================================================
# 报告渲染
# ====================================================================


def render_md(res: dict) -> str:
    m = res["meta"]
    bh = res["buy_and_hold"]
    L: list[str] = []
    L.append(f"# QQQ settle 阶梯（PH 右侧确认）× 缠论中枢过滤 — 在线回测\n")
    L.append("> 引擎：`OnlineMergeTree`（买=喂close下跌腿settle / 卖=喂-close上涨腿settle）"
             "+ `stop_signal`（否定出）+ `detect_zhongshu`（中枢过滤）。\n"
             "> 脚本：`scripts/settle_backtest.py`。数据：yfinance `QQQ` 日线 `period=\"max\"`。\n")
    L.append("---\n")
    L.append("## 0. 数据与认识论\n")
    L.append(f"- 区间：**{m['date_start']} → {m['date_end']}**（{m['n_bars']} 根日线）")
    L.append(f"- 价格：起 {m['price_start']} → 终 {m['price_end']}；"
             f"区间 [{m['price_min']}, {m['price_max']}]")
    L.append(f"- ATR14 噪声阈 τ = **{m['tau_atr']}**（settle 分量 persistence < τ 视为噪声，不触发信号）")
    L.append(f"- 信号计数：买入信号 **{m['n_buy_signals']}** 次（下跌腿settle≥τ）/ "
             f"卖出信号 **{m['n_sell_signals']}** 次（上涨腿settle≥τ）")
    L.append("- **认识论等级**：PH/merge tree/stop_signal = L0；"
             "「settle=分型确认」「中枢≥2=趋势」= L0算法+候选同构；"
             "**单标的回测胜率/收益 = L2（可否证，本脚本信息增量所在）**；跨标的 L3 未做。\n")

    L.append("---\n")
    L.append("## 1. buy-and-hold 基准（全期）\n")
    L.append(f"| 指标 | 值 |")
    L.append(f"|---|---|")
    L.append(f"| 全期总收益 | **{bh['total_ret_pct']:+.1f}%** |")
    L.append(f"| 年化收益 | {bh['annualized_pct']:+.1f}% |")
    L.append(f"| 最大回撤(逐日) | **−{bh['max_drawdown_pct']:.1f}%** |")
    L.append("\n> buy-and-hold 全程 100% 暴露。策略只在部分时间持仓（见下表"
             "「市场暴露」），对比时须结合暴露占比与最大回撤，不能只看总收益。\n")

    L.append("---\n")
    L.append("## 2. 四策略变体对比\n")
    L.append("| 策略 | 笔数 | 胜率 | 平均收益 | 平均盈/亏 | 平均持有(交易日) | "
             "总收益 | 最大回撤 | 市场暴露 |")
    L.append("|---|---|---|---|---|---|---|---|---|")
    for s in res["strategies"]:
        L.append(
            f"| {s['name']} | {s['n_trades']} | {s['win_rate_pct']:.0f}% | "
            f"{s['avg_ret_pct']:+.2f}% | {s['avg_win_pct']:+.1f}/{s['avg_loss_pct']:+.1f}% | "
            f"{s['avg_hold_days']:.0f} | **{s['total_ret_pct']:+.1f}%** | "
            f"−{s['max_drawdown_pct']:.1f}% | {s['market_exposure_pct']:.0f}% |"
        )
    L.append("")
    for s in res["strategies"]:
        if s["open_position"]:
            op = s["open_position"]
            L.append(f"- **{s['name']}** 末根仍持仓（{op['entry_date']} @ {op['entry']} → "
                     f"今 {op['last']}，浮盈 {op['float_ret_pct']:+.1f}%，未计入已实现胜率）")
    L.append("")

    L.append("---\n")
    L.append("## 3. 解读（过滤器与离场规则的效力）\n")
    s_map = {s["name"]: s for s in res["strategies"]}
    s1, s2 = res["strategies"][0], res["strategies"][1]
    s3, s4 = res["strategies"][2], res["strategies"][3]
    # 过滤器效力：中枢≥2 是否提升胜率
    def cmp_line(a, b, label):
        dw = b["win_rate_pct"] - a["win_rate_pct"]
        dn = b["n_trades"] - a["n_trades"]
        dr = b["total_ret_pct"] - a["total_ret_pct"]
        verdict = "提升" if dw > 0 else ("无改善" if abs(dw) < 1e-9 else "下降")
        return (f"- **{label}**：中枢≥2 过滤后胜率 {a['win_rate_pct']:.0f}% → "
                f"{b['win_rate_pct']:.0f}%（{dw:+.0f}pp，{verdict}），"
                f"笔数 {a['n_trades']}→{b['n_trades']}（{dn:+d}），"
                f"总收益 {a['total_ret_pct']:+.1f}%→{b['total_ret_pct']:+.1f}%（{dr:+.1f}pp）")
    L.append("**缠论中枢过滤器（中枢≥2）是否有效？**")
    L.append(cmp_line(s1, s2, "对称settle 下"))
    L.append(cmp_line(s3, s4, "否定出 下"))
    L.append("")
    L.append("**两种离场规则对比？**")
    L.append(f"- 对称settle vs 否定出（无过滤）：胜率 {s1['win_rate_pct']:.0f}% vs "
             f"{s3['win_rate_pct']:.0f}%；平均持有 {s1['avg_hold_days']:.0f} vs "
             f"{s3['avg_hold_days']:.0f} 日；总收益 {s1['total_ret_pct']:+.1f}% vs "
             f"{s3['total_ret_pct']:+.1f}%；最大回撤 −{s1['max_drawdown_pct']:.1f}% vs "
             f"−{s3['max_drawdown_pct']:.1f}%")
    best = max(res["strategies"], key=lambda s: s["total_ret_pct"])
    L.append(f"- 总收益最高变体：**{best['name']}**（{best['total_ret_pct']:+.1f}%），"
             f"对比 buy-and-hold {bh['total_ret_pct']:+.1f}%"
             f"（暴露仅 {best['market_exposure_pct']:.0f}%）")
    L.append("")

    L.append("---\n")
    L.append("## 结果包（六要素）\n")
    L.append(f"1. **结论**：QQQ {m['date_start']}→{m['date_end']} 在线回测，PH settle "
             f"右侧确认信号共触发买入 {m['n_buy_signals']} / 卖出 {m['n_sell_signals']} 次。"
             f"四变体总收益区间 "
             f"[{min(s['total_ret_pct'] for s in res['strategies']):+.1f}%, "
             f"{max(s['total_ret_pct'] for s in res['strategies']):+.1f}%]，"
             f"buy-and-hold 全期 {bh['total_ret_pct']:+.1f}%。胜率见 §2 表。")
    L.append("2. **定义依据**：")
    L.append("   - 买入信号 = `OnlineMergeTree(close).update()` 返回非空且 persistence≥τ "
             "（下跌腿 settle = 底分型因果确认，a_online_persistence §10 定理）。")
    L.append("   - 卖出信号 = `OnlineMergeTree(-close).update()` 返回非空且 persistence≥τ "
             "（上涨腿 settle = 顶分型确认，sublevel 对称）。")
    L.append("   - 否定出 = `stop_signal.reason∈{reverse_exceeds,both}`（入场后新生下跌腿 "
             "persistence 超入场确认底幅 = 对象否定对象，005b号）。")
    L.append("   - 中枢过滤 = `detect_zhongshu`（min_members=3, ratio=3.0, overlap）计数≥2。")
    L.append("3. **边界条件**（结论翻转条件）：")
    L.append("   - τ 倍数变化（当前 multiple=1.0）→ 信号频率与笔数改变，胜率随之漂移。")
    L.append("   - 用 close 而非 HL 双树：换 HL 双树（顶/底建立在 high/low）信号会变。")
    L.append("   - 复权口径：yfinance `auto_adjust=False`（含派息缺口）；改全复权 close 阶梯改变。")
    L.append("   - 单标的：QQQ 是长牛标的，long-only 天然顺风；换熊市/震荡标的结论可能翻转（L3 未做）。")
    L.append("4. **下游推论**：")
    L.append("   - 若中枢过滤显著提升胜率 → 「PH 右侧 settle + 缠论结构合取」优于纯 PH，"
             "支持「右侧确认 + 趋势在场」的合取条件。若无改善/下降 → 中枢过滤在日线 QQQ "
             "上不是有效增量（否定性结果，缩小有效域）。")
    L.append("   - 若否定出最大回撤显著小于对称settle但总收益跑输 → 否定出是「降波动换收益」"
             "的风控档，符合 alive persistence 用 running_max 封顶导致的早离场特性。")
    L.append("5. **谱系引用**：")
    L.append("   - settle=分型确认 / 全局低点不可反弹 settle —— `persistence_theory.md` §7.5/§10/§17.3；"
             "090号声明膨胀禁令。")
    L.append("   - 否定出=对象否定对象 —— 005b号 / `stop_signal`。")
    L.append("   - close 取负拿卖出信号的对称性 —— `a_dual_merge_tree` 取负原理（此处用 close 非 HL）。")
    L.append("   - 单标的回测有效域 ≠ 跨标的定义域 —— formalization-validity-domain（L2 vs L3）。")
    L.append("6. **影响声明**：新增 `scripts/settle_backtest.py`、`analysis/settle_backtest_qqq.md`、"
             "`analysis/data_cache/settle_backtest_qqq.json` + `QQQ_1d_max.json`。"
             "不改动任何 `src/newchan/` 引擎模块、不改动任何定义。")
    L.append("")
    return "\n".join(L)


def main() -> None:
    data = pull(SYMBOL, PERIOD)
    CACHE.mkdir(parents=True, exist_ok=True)
    (CACHE / "QQQ_1d_max.json").write_text(json.dumps(data, ensure_ascii=False))
    res = run_backtest(data)
    (CACHE / "settle_backtest_qqq.json").write_text(
        json.dumps(res, ensure_ascii=False, indent=2))
    md = render_md(res)
    OUT_MD.write_text(md)

    m = res["meta"]
    bh = res["buy_and_hold"]
    print("=" * 78)
    print(f"QQQ settle 回测  {m['date_start']} → {m['date_end']}  (n={m['n_bars']})")
    print("=" * 78)
    print(f"价格 {m['price_start']} → {m['price_end']}  [{m['price_min']}, {m['price_max']}]  τ={m['tau_atr']}")
    print(f"信号：买入 {m['n_buy_signals']} / 卖出 {m['n_sell_signals']}")
    print(f"buy&hold 全期 {bh['total_ret_pct']:+.1f}%  年化 {bh['annualized_pct']:+.1f}%  "
          f"最大回撤 −{bh['max_drawdown_pct']:.1f}%")
    print("-" * 78)
    print(f"{'策略':<20}{'笔数':>5}{'胜率':>7}{'平均':>8}{'持有':>6}{'总收益':>9}{'回撤':>8}{'暴露':>7}")
    for s in res["strategies"]:
        print(f"{s['name']:<20}{s['n_trades']:>5}{s['win_rate_pct']:>6.0f}%"
              f"{s['avg_ret_pct']:>7.2f}%{s['avg_hold_days']:>5.0f}d"
              f"{s['total_ret_pct']:>8.1f}%{-s['max_drawdown_pct']:>7.1f}%"
              f"{s['market_exposure_pct']:>6.0f}%")
        if s["open_position"]:
            op = s["open_position"]
            print(f"    └ 末根持仓 {op['entry_date']}@{op['entry']} 浮盈 {op['float_ret_pct']:+.1f}%")
    print(f"\n已写 {OUT_MD.relative_to(ROOT)} + data_cache/settle_backtest_qqq.json")


if __name__ == "__main__":
    main()
