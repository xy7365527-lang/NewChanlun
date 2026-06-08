"""版本 I（完整版，取消全部阉割）— 全仓 + 每级独立 BSP + 多 FSM 多重赋格降成本。

═══════════════════════════════════════════════════════════════════════
本次重写：取消 6 个阉割（用户指令）
═══════════════════════════════════════════════════════════════════════
1. **单 FSM → 多 FSM**：核心仓位**全仓**（INITIAL_CAPITAL，方向利润底仓，不做短差）；
   机动仓（MANEUVER_RATIO×总仓，从总仓划出）在 entry 层以下各降成本级别间均分，每级别
   一个独立 `CostReductionFSM`，各自追踪 cost_basis（多声部 = 多重赋格，每层资金独立——
   第40课"每一重对应一定资金与筹码"）。不是一个 FSM 管所有级别。
2. **每层独立 BSP**（经 per_level_bsp 适配层，任务卡强制）：每个 **中枢承载层**（走势 L1 +
   递归 L≥2）都产生真实 confirmed type1/2/3 买卖点——走势级用 `confirmed_bsp_level1`
   （透传引擎 `snap.bsp_snapshot`），递归层用 `confirmed_bsp_for_level`（per_level_bsp 内部
   组合 `buysellpoints_from_level` + `divergences_from_moves_v1`，525号组件来源无关性 +
   well-formedness 索引证明）。**取消 "move settle + 背驰" 近似，不在本文件重复实现适配器**。
3. **不排除 bar 级**：bar/笔/线段级以最小级别下限参数 `MIN_FLOOR_LADDER` 控制（默认 0
   = 含 bar），不再硬编码"bar 级不作降成本触发"的主观排除。⚠ 见下方"缠师原文冲突"。
4. **认真处理高层**：递归层完整跑 BSP，不因稀疏退化。稀疏性作为回测结果（归属分布 +
   每级 FSM 贡献度）自然呈现。
5. **去 sub_ratio=0.3 硬编码 + 区分两个量**（考据设计点2）：原文唯一固定比例"例如 1/10"
   是**机动仓占总仓比例**（非单笔短差量）。重写区分 `MANEUVER_RATIO`（机动仓占比，默认
   0.1=原文，env `BT_MANEUVER_RATIO`）与 `_level_trade_fraction`（单笔量，级别驱动，非
   固定 0.3）。删除把两者混为一谈的 SUB_RATIO=0.1 单标量。
6. **删除伪造的"267号"课号引用**：原文只有 108 课。注意：267号在 `.chanlun/genealogy/`
   中是**真实的谱系号**（267-operational-methodology-v1，被 268a 质询），非课号。本文件
   docstring 不再以"267号"冒充原文权威；FSM 状态语义的原文依据见下方课号引用。

═══════════════════════════════════════════════════════════════════════
缠论原文依据（一级权威：缠师 108 课博文。考据见本次 session）
═══════════════════════════════════════════════════════════════════════
- **多级别立体降成本（多重赋格）**：第35课"其次级别的次级别，也可以用来部分操作……
  整个操作就有一定的立体性"。→ 每个操作级别一个独立降成本循环有据。
- **买卖点驱动短差**：第27课答疑"以日线买点进，看30分钟买卖点做短差降成本"；第33课
  "每次向下离开中枢只要出现底背驰，那就可以介入了"。→ 次级别卖点高抛、买点低吸有据。
- **三段式 / 成本为0 / 挣股数**：第31课"成本为0前……股数不增加；成本为0后……卖出多少
  资金就买入多少资金做短差赚股票"；第43课答疑"先卖后买……股数越来越多"。
- **机动资金比例**：第31课"每只股票留 1/10"（单标的）；第25课"占仓位 1/4 到 1/3"；
  第80课"组合 10-30%"。三处口径互斥 → 0.3 无单标的依据，默认取 1/10。
- ⚠ **缠师原文与指令#3 的冲突（不隐藏）**：第53课"最小也不应该小于5分钟"；第35课
  "那些太小的级别……从长期的角度看，是没有意义的"；第31课答疑"1分钟背驰都每个弄一下
  ……那是很累的"。→ 缠师**明确反对** bar 级/过小级别降成本（理由：交易成本/误差相对
  波幅不可忽略）。指令#3 要求纳入 bar 级。严格解法：下限改为透明参数（消除"主观排除"），
  默认纳入 bar 级（遵指令），并**输出每级 FSM 独立贡献度，让数据经验裁决** bar 级是否
  为噪音（若是 → 经验印证缠师原文）。

═══════════════════════════════════════════════════════════════════════
级别阶梯 ladder（低→高）与 BSP 来源
═══════════════════════════════════════════════════════════════════════
  ladder 0 = bar          → PH proxy（close 树）          [sub-走势，无中枢]
  ladder 1 = 笔(bi)        → PH proxy（stroke 端点树）     [sub-走势，无中枢]
  ladder 2 = segment(笔中枢) → 真实 BSP（笔中枢，525号）    [中枢承载，type1/2/3] ★本次新增
  ladder 3 = 走势(L1)      → 真实 BSP（snap.bsp_snapshot） [中枢承载，type1/2/3]
  ladder L+2 = 递归 L(≥2)  → 真实 BSP（适配器+纯函数）      [中枢承载，type1/2/3]

- **归属/进出场候选** = 中枢承载层（ladder≥2，有真实 type1 买卖点）。
  ★本次：FIRST_BSP_LADDER 从 3 下放到 2——525号笔中枢（三笔重叠→笔中枢→笔级别走势
  →type1/2/3）给 segment 级一个真实中枢承载层，故 entry_level 可下探到 segment。
- **降成本触发** = entry 层以下的所有 ladder（≥ MIN_FLOOR_LADDER），各自独立 FSM。
- bar/bi（ladder 0/1）仍无中枢（单笔不是走势）→ 无 type1/2/3，用 PH settle 作短差触发
  （该粒度可得的最细买卖点代理）。**segment 级原为 PH proxy，本次升级为笔中枢真实 BSP。**
- 笔中枢路径不违反 107号"笔不裁决"（后者禁笔作**段**中枢组件；笔中枢是退化基底中枢，
  笔为终端递归单位，第17课"单位"从 K线提升为笔）。谱系 525号。

═══════════════════════════════════════════════════════════════════════
认识论等级与力度口径（formalization-validity-domain 规则）
═══════════════════════════════════════════════════════════════════════
- **认识论 L2**：真实数据（447K OKLO），含高层稀疏 + 降成本拖累 + bar 级噪音的否定性结果。
- **力度口径 = 价格振幅 × 持续（引擎 fallback）**，**非 MACD**：缠论正典背驰用 MACD
  （第24/25课），但 orchestrator 的 `enable_macd_divergence=True` 每 bar 重建 DataFrame
  为 O(N²)，447K 上实测 ~1500us/bar 且超线性增长（不可行）。故全层统一用引擎自带的价格
  振幅力度 fallback。这是已知有效域边界：type1 的 force_c/force_a≤0.9 判据成立，但背驰
  的 MACD 维度（黄白线/柱子）未纳入。**这是诚实的力度口径声明，非阉割。**

谱系：521号（纯拓扑无动量）/ project_divergence_locator_entry_exit /
      project_costreduction_moneyprinter_bug / project_complete_fugue_v2 /
      project_best_combo_version_i。
"""

from __future__ import annotations

import json
import os
import sys
import time
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

from newchan.a_macd import OnlineMacdState  # noqa: E402
from newchan.events import MoveSettleV1, SegmentSettleV1  # noqa: E402
from newchan.orchestrator.recursive import RecursiveOrchestrator  # noqa: E402
from newchan.trading.cost_reduction_fsm import (  # noqa: E402
    CostReductionFSM,
    CostState,
    FsmEvent,
    FsmEventType,
    transition,
)
from newchan.types import Bar  # noqa: E402

import fugue_alpha_diagnosis as _ef  # noqa: E402
from fugue_alpha_diagnosis import (  # noqa: E402
    INITIAL_CAPITAL,
    MODE_NONE,
    PHLevelState,
    CompletedTrade,
    MoveRecord,
    UpSegRecord,
    _median_alive_persistence,
    _move_force_simple,
    _persistence_ratio,
    _refined_gate,
    check_divergence,
    compute_metrics,
    run_swing_trading,
)
from fugue_complete_v2 import BarSignalV2  # noqa: E402
from per_level_bsp import (  # noqa: E402
    confirmed_bsp_bi_zhongshu,
    confirmed_bsp_for_level,
    confirmed_bsp_level1,
)

DATA_DIR = ROOT / "analysis" / "data_cache"
OUTPUT_MD = ROOT / "analysis" / "fugue_version_i_results.md"

# 默认 447K 完整 OKLO（bars-schema，含真实时间戳）。
DATA_FILES = {
    "OKLO": "oklo_1m_databento.json",
}

MAX_LEVELS = 8
# ladder: 0=bar 1=bi 2=segment 3=走势(L1) ; 递归 L → ladder L+2（L2→4 … L8→10）。
LADDER_BAR, LADDER_BI, LADDER_SEG, LADDER_MOVE = 0, 1, 2, 3
MAX_LADDER = MAX_LEVELS + 3  # 0..10 → 11 槽
# 中枢承载层下沿（真实 type1/2/3 BSP）下放到 ladder 2（segment 级）：
# 525号笔中枢路径（三笔重叠 → 笔中枢 → 笔级别走势 → type1/2/3）给 segment 级
# 一个真实中枢承载层，故 entry_level/降成本可下探到 segment。bar/bi（ladder 0/1）
# 仍无中枢（单笔不是走势），保持 PH proxy。
FIRST_BSP_LADDER = LADDER_SEG  # ladder≥2 = 中枢承载层（segment=笔中枢，走势+=线段/递归中枢）

# ── 可配置参数（去硬编码 + 区分两个量，阉割#5 + 考据设计点2）──
# 考据裁决（complete_rewrite_textual_basis.md 设计点2）：原文唯一固定比例"例如 1/10"
# （blog/031 26行）修饰的是**机动仓占总仓比例**，不是"每次短差吃掉的量"。把两者混为
# 一谈是概念混淆。本重写**区分两个量**：
#
#   1. MANEUVER_RATIO（机动仓占总仓）：默认 0.1 = 原文"例如 1/10"。机动仓在各活跃降成本
#      级别间均分（disjoint 切片，每声部独立资金——第40课"每一重对应一定资金与筹码"）。
#   2. 单笔短差量（_level_trade_fraction）：由级别驱动（原文 chan99/0033 11行"量基本只和
#      级别有关，日线级别买卖量比 1分钟多多了"），**非固定标量 0.3**。返回"占本级别 slice
#      的比例"，级别越高量越大。
#
# 删除原 SUB_RATIO=0.1 的"单次短差比例"语义（它把机动仓占比错当成单笔量）。
MANEUVER_RATIO = float(os.environ.get("BT_MANEUVER_RATIO", "0.1"))
# 降成本最小级别下限（ladder）。默认 0 = 含 bar（遵指令#3）。
# ⚠ 缠师原文（第53/35/31课）主张 ≥ 线段/5分钟级（≈ladder 2~3）。设 env BT_FLOOR_LADDER=3
#   可恢复缠师口径，对照 bar 级贡献度判断噪音。
MIN_FLOOR_LADDER = int(os.environ.get("BT_FLOOR_LADDER", "0"))

PERSIST_MED_WINDOW = 50


def _level_trade_fraction(ladder: int) -> float:
    """单笔短差吃掉本级别 slice 的比例——由级别驱动（原文"量基本只和级别有关"）。

    chan99/0033 11行："日线级别的买卖量当然比 1分钟级别的要多多了"——量随操作级别递增。
    这里把单笔比例做成 ladder 的单调递增函数，**不是固定 0.3 标量**。返回值是"占本级别
    机动 slice 的比例"，非占总仓比例。

    认识论 L0：从原文"量由级别决定"的定性陈述推导出单调递增形状；具体斜率（base/step）
    无原文数值依据，是 L0 形式化引申（标注），由回测调参。clip 到 [0.05, 1.0]。
    """
    base = 0.25
    step = 0.10
    frac = base + step * max(0, ladder - LADDER_SEG)
    return min(1.0, max(0.05, frac))


_COST_OPEN_STATES = frozenset({
    CostState.POSITION_OPEN, CostState.COST_REDUCING,
    CostState.PRINCIPAL_WITHDRAWN, CostState.EARNING_SHARES,
})
_COST_ACTIVE_DIFF_STATES = frozenset({
    CostState.COST_REDUCING, CostState.EARNING_SHARES,
})


def ladder_name(ladder: int) -> str:
    """ladder → 人类可读级别名（用于报告与贡献度归属）。"""
    if ladder == LADDER_BAR:
        return "bar"
    if ladder == LADDER_BI:
        return "bi"
    if ladder == LADDER_SEG:
        return "segment"
    if ladder == LADDER_MOVE:
        return "move(L1)"
    return f"recL{ladder - 2}"


# ════════════════════════════════════════════════════════════
# 数据加载（bars-schema：[{ts,open,high,low,close,volume}, ...]）
# ════════════════════════════════════════════════════════════

def load_symbol(symbol: str) -> tuple[list, list, list, list, list | None]:
    path = DATA_DIR / DATA_FILES[symbol.upper()]
    raw = json.loads(path.read_text())
    if "bars" in raw:  # 447K databento bars-schema
        bars = raw["bars"]
        opens = [float(b["open"]) for b in bars]
        highs = [float(b["high"]) for b in bars]
        lows = [float(b["low"]) for b in bars]
        closes = [float(b["close"]) for b in bars]
        years = [int(str(b["ts"])[:4]) for b in bars]
        return opens, highs, lows, closes, years
    # 兼容旧 parallel-array schema
    opens = [float(x) for x in raw["opens"]]
    highs = [float(x) for x in raw["highs"]]
    lows = [float(x) for x in raw["lows"]]
    closes = [float(x) for x in raw["closes"]]
    years = [int(str(d)[:4]) for d in raw["dates"]] if "dates" in raw else None
    return opens, highs, lows, closes, years


def iter_bars(opens, highs, lows, closes):
    base = datetime(2020, 1, 1)
    from datetime import timedelta
    for i in range(len(closes)):
        yield Bar(ts=base + timedelta(minutes=i), open=opens[i], high=highs[i],
                  low=lows[i], close=closes[i], volume=0.0)


# ════════════════════════════════════════════════════════════
# 每层真实 BSP（取消近似，阉割#2）——经 per_level_bsp 适配层
# ════════════════════════════════════════════════════════════
#
# 任务卡强制：用 analysis/per_level_bsp.py 替换近似买卖点（原 LevelDivState 的
# "move settle + 背驰" 近似）。per_level_bsp 是已实装并带测试（test_per_level_bsp.py）
# 的指定适配层，其 docstring 给出了 well-formedness 证明（component_idx 索引自洽）。
# 不在本文件内重复实现适配器（no-patch-mentality：禁止重复逻辑）。
#
# 索引契约（per_level_bsp 模块 docstring）：
#   confirmed_bsp_for_level(prev_level_moves, level_zhongshus, level_moves, level_id)
#   - prev_level_moves = 前一级别（N-1）全部 moves（顺序不变）
#   - level_zhongshus  = 本级别（N）全部 LevelZhongshu
#   - level_moves      = 本级别（N）全部 Move
# 引擎以 settled_moves 过滤建中枢（recursive_level_engine 93行），settled 在 moves
# 列表前缀（未闭合在尾部，缠论 move settle 单调），故全列表与 settled 前缀对 settled
# 位置索引一致——per_level_bsp 多带的尾部未闭合元素不被任何 comp_idx 引用，无害。


def _scan_confirmed_bsps(bsps) -> tuple[bool, bool, bool, bool]:
    """从 confirmed BuySellPoint 列表扫描信号。返回 (buy1, sell1, sell_any, buy_any)。

      buy1  = confirmed type1 买点（底背驰 → 进场/归属）
      sell1 = confirmed type1 卖点（顶背驰 → entry 层主出场）
      sell_any / buy_any = 任意 type confirmed 卖/买点（次级别高抛/低吸触发）
    """
    buy1 = sell1 = sell_any = buy_any = False
    for bp in bsps:
        if not bp.confirmed:
            continue
        if bp.side == "buy":
            buy_any = True
            if bp.kind == "type1":
                buy1 = True
        else:
            sell_any = True
            if bp.kind == "type1":
                sell1 = True
    return buy1, sell1, sell_any, buy_any


class _LevelBspTracker:
    """单个递归层 (level_id≥2) 的 confirmed BSP 跟踪器：per_level_bsp 全量 + 去重检测新增。

    per_level_bsp.confirmed_bsp_for_level 返回全量 BSP 列表；本类按 (kind,side,seg_idx)
    去重，只报本 bar **新出现**的 confirmed 买卖点，避免同一 BSP 跨 bar 重复触发。
    只在该层 move/zhongshu 变化时调用（move_events/zhongshu_events 非空）。
    """

    __slots__ = ("_level_id", "_seen")

    def __init__(self, level_id: int) -> None:
        self._level_id = level_id
        self._seen: set[tuple[str, str, int]] = set()

    def step(
        self, prev_level_moves: list, level_zhongshus: list, level_moves: list,
    ) -> tuple[bool, bool, bool, bool]:
        """重算 confirmed BSP，返回本 bar 新增的 (buy1, sell1, sell_any, buy_any)。"""
        bsps = confirmed_bsp_for_level(
            prev_level_moves, level_zhongshus, level_moves, self._level_id)
        b1 = s1 = sa = ba = False
        for bp in bsps:
            if not bp.confirmed:
                continue
            key = (bp.kind, bp.side, bp.seg_idx)
            if key in self._seen:
                continue
            self._seen.add(key)
            if bp.side == "buy":
                ba = True
                if bp.kind == "type1":
                    b1 = True
            else:
                sa = True
                if bp.kind == "type1":
                    s1 = True
        return b1, s1, sa, ba


class _TrendBspTracker:
    """走势级（引擎 level=1）confirmed BSP 跟踪器：confirmed_bsp_level1 透传 + 去重检测新增。"""

    __slots__ = ("_seen",)

    def __init__(self) -> None:
        self._seen: set[tuple[str, str, int]] = set()

    def step(self, bsp_snapshot) -> tuple[bool, bool, bool, bool]:
        bsps = confirmed_bsp_level1(bsp_snapshot)
        b1 = s1 = sa = ba = False
        for bp in bsps:
            if not bp.confirmed:
                continue
            key = (bp.kind, bp.side, bp.seg_idx)
            if key in self._seen:
                continue
            self._seen.add(key)
            if bp.side == "buy":
                ba = True
                if bp.kind == "type1":
                    b1 = True
            else:
                sa = True
                if bp.kind == "type1":
                    s1 = True
        return b1, s1, sa, ba


class _BiZhongshuBspTracker:
    """segment 级（笔中枢，525号）confirmed BSP 跟踪器：全量重算 + 去重检测新增。

    把 ladder 2（segment）从 PH proxy 升级为真实中枢承载层：笔 → 笔中枢 →
    笔级别走势 → type1/2/3。`confirmed_bsp_bi_zhongshu` 每次全量重算（笔引擎是 O(N²)，
    与既有递归层 tracker 同构），本类按 (kind,side,seg_idx) 去重，只报本 bar **新出现**
    的 confirmed 买卖点。仅在 strokes 增长（新 confirmed 笔）时调用，避免无谓重算。
    """

    __slots__ = ("_seen",)

    def __init__(self) -> None:
        self._seen: set[tuple[str, str, int]] = set()

    def step(self, strokes: list) -> tuple[bool, bool, bool, bool]:
        bsps = confirmed_bsp_bi_zhongshu(strokes)
        b1 = s1 = sa = ba = False
        for bp in bsps:
            if not bp.confirmed:
                continue
            key = (bp.kind, bp.side, bp.seg_idx)
            if key in self._seen:
                continue
            self._seen.add(key)
            if bp.side == "buy":
                ba = True
                if bp.kind == "type1":
                    b1 = True
            else:
                sa = True
                if bp.kind == "type1":
                    s1 = True
        return b1, s1, sa, ba


# ════════════════════════════════════════════════════════════
# 分层信号磁带（每 bar）
# ════════════════════════════════════════════════════════════

@dataclass(slots=True)
class BarSignalI:
    close: float
    buy1: tuple       # ladder → confirmed type1 买点（进场/归属，仅 ladder≥3）
    sell1: tuple      # ladder → confirmed type1 卖点（entry 层主出场，仅 ladder≥3）
    sell_any: tuple   # ladder → 高抛触发（中枢层=任意卖点；sub-走势=PH 峰 settle）
    buy_any: tuple    # ladder → 低吸触发（中枢层=任意买点；sub-走势=PH 谷 settle）
    max_ladder: int   # 当前已涌现最高 ladder
    type2_buy: bool   # 走势级 type2 买标记（仅计数）


# ════════════════════════════════════════════════════════════
# 信号层（compute-once）：单次 orch pass → E 磁带 + I 磁带
# ════════════════════════════════════════════════════════════

def compute_signals_i(
    opens: list[float], highs: list[float], lows: list[float], closes: list[float],
) -> tuple[list[BarSignalV2], list[BarSignalI]]:
    n = len(closes)
    orch = RecursiveOrchestrator(stream_id="bt_i", max_levels=MAX_LEVELS)

    # sub-走势 PH 树（bar / bi）—— ladder 0/1 无中枢，保持 PH proxy。
    # ladder 2（segment）已升级为笔中枢真实 BSP（见下 bi_zhongshu_bsp），不再用 PH proxy。
    bar_dn = PHLevelState.make(); bar_up = PHLevelState.make()
    bi_dn = PHLevelState.make(); bi_up = PHLevelState.make()
    last_stroke_n = 0

    # E 基线所需（走势级 PH + 手算 MACD 面积 + E 字段）—— 保持与既有 E 一致
    l1_down = PHLevelState.make(); l1_up = PHLevelState.make()
    l2_down = PHLevelState.make(); l2_up = PHLevelState.make()
    l0_down = PHLevelState.make(); l0_up = PHLevelState.make()
    macd = OnlineMacdState()
    pos_cum = 0.0; pos_cum_hist: list[float] = []
    neg_cum = 0.0; neg_cum_hist: list[float] = []
    l1_up_segs: list[tuple[float, float]] = []; last_l1up_i = 0
    l1_down_segs: list[tuple[float, float]] = []; last_l1down_i = 0
    down_move_hist: list[MoveRecord] = []
    up_move_hist: list[MoveRecord] = []
    l2_direction = 0

    # BSP 跟踪器（segment 笔中枢 + 走势级 + 递归层），均经 per_level_bsp 适配层
    bi_zhongshu_bsp = _BiZhongshuBspTracker()  # ladder 2 = 笔中枢真实 BSP（525号）
    trend_bsp = _TrendBspTracker()
    level_bsp: dict[int, _LevelBspTracker] = {}
    max_ladder = LADDER_SEG  # segment 级（笔中枢）已是最低中枢承载层

    e_signals: list[BarSignalV2] = []
    i_signals: list[BarSignalI] = []
    last_progress = 0

    def _macd_pos_area(a: int, b: int) -> float:
        if b < 0:
            return 0.0
        return pos_cum_hist[b] if a <= 0 else pos_cum_hist[b] - pos_cum_hist[a - 1]

    def _macd_neg_area(a: int, b: int) -> float:
        if b < 0:
            return 0.0
        return neg_cum_hist[b] if a <= 0 else neg_cum_hist[b] - neg_cum_hist[a - 1]

    base_ts = datetime(2020, 1, 1)
    from datetime import timedelta

    for i in range(n):
        c = closes[i]; h = highs[i]; lo = lows[i]
        ts = base_ts + timedelta(minutes=i)
        bar = Bar(ts=ts, open=opens[i], high=h, low=lo, close=c, volume=0.0)

        _, _, hist = macd.update(c, ts)
        pos_cum += hist if hist > 0 else 0.0
        pos_cum_hist.append(pos_cum)
        neg_cum += -hist if hist < 0 else 0.0
        neg_cum_hist.append(neg_cum)

        snap = orch.process_bar(bar)
        moves = snap.move_snapshot.moves

        # ── ladder0 bar：close PH ──
        bar_dn_s = bar_dn.tree.update(c); bar_up_s = bar_up.tree.update(-c)
        bar_buy_r1, bar_buy_nr1 = bar_dn.detect_settle(bar_dn_s)
        bar_sell_r1, bar_sell_nr1 = bar_up.detect_settle(bar_up_s)
        bar_buy = bar_buy_r1 or bar_buy_nr1
        bar_sell = bar_sell_r1 or bar_sell_nr1
        # E 基线复用 bar PH（l0）—— 与 bar 树同源，独立实例保持 E tape 不变
        l0_r1_down, _ = l0_down.detect_settle(l0_down.tree.update(c))
        l0_r1_up, _ = l0_up.detect_settle(l0_up.tree.update(-c))

        # ── ladder1 笔(bi)：新 stroke 端点 PH ── + ladder2 笔中枢 BSP（525号）──
        # 笔仍无中枢（单笔不是走势）→ ladder1 保持 PH proxy。
        # ladder2（segment）= 笔中枢真实 BSP：strokes 增长时全量重算（O(N²)，与递归层同构）。
        bi_buy = False; bi_sell = False
        seg_buy1 = seg_sell1 = seg_sell_any = seg_buy_any = False
        strokes = snap.bi_snapshot.strokes
        if len(strokes) > last_stroke_n:
            for sk in strokes[last_stroke_n:]:
                p1 = sk.p1
                if p1 > 0:
                    dn_s = bi_dn.tree.update(p1); up_s = bi_up.tree.update(-p1)
                    r1d, nr1d = bi_dn.detect_settle(dn_s)
                    r1u, nr1u = bi_up.detect_settle(up_s)
                    if r1d or nr1d:
                        bi_buy = True
                    if r1u or nr1u:
                        bi_sell = True
            last_stroke_n = len(strokes)
            # 笔中枢级 confirmed BSP（ladder 2）：仅在 strokes 增长时重算
            seg_buy1, seg_sell1, seg_sell_any, seg_buy_any = bi_zhongshu_bsp.step(strokes)

        # ── E 基线 l1 树（segment EP PH）—— 逐字保留，E tape 不变量 ──
        l1_nr1_up = False; l1_nr1_down = False
        for e in snap.seg_snapshot.events:
            if isinstance(e, SegmentSettleV1):
                ep = e.ep1_price
                if ep <= 0:
                    continue
                # E 基线 l1 树
                _, e_nr1d = l1_down.detect_settle(l1_down.tree.update(ep))
                _, e_nr1u = l1_up.detect_settle(l1_up.tree.update(-ep))
                if e_nr1u:
                    l1_nr1_up = True
                if e_nr1d:
                    l1_nr1_down = True
                if e.ep1_price > e.ep0_price:
                    seg_lo = last_l1up_i + 1 if last_l1up_i + 1 <= i else i
                    l1_up_segs.append((max(highs[seg_lo:i + 1]) if seg_lo <= i else h,
                                       _macd_pos_area(last_l1up_i, i)))
                    last_l1up_i = i
                elif e.ep1_price < e.ep0_price:
                    seg_lo2 = last_l1down_i + 1 if last_l1down_i + 1 <= i else i
                    l1_down_segs.append((min(lows[seg_lo2:i + 1]) if seg_lo2 <= i else lo,
                                         _macd_neg_area(last_l1down_i, i)))
                    last_l1down_i = i

        # ── 走势级（ladder3 = L1）：真实 BSP（per_level_bsp 透传引擎 BSP）+ E 字段 ──
        l1_buy1, l1_sell1, l1_sell_any, l1_buy_any = trend_bsp.step(snap.bsp_snapshot)
        type2_buy = any(
            type(e).__name__ == "BuySellPointCandidateV1" and e.side == "buy"
            and getattr(e, "kind", "") == "type2"
            for e in snap.bsp_snapshot.events)

        # E 字段（逐字同 v2，保持 E 基线 tape 不变）
        l2_flip_long = False; l2_flip_short = False
        down_move_settled = False; up_move_settled = False
        for e in snap.move_snapshot.events:
            if isinstance(e, MoveSettleV1):
                mv = next((m for m in moves if m.seg_start == e.seg_start
                           and m.direction == e.direction and m.settled), None)
                if mv is None:
                    continue
                if mv.direction == "down":
                    down_move_hist.append(MoveRecord(
                        direction="down", high=mv.high, low=mv.low,
                        macd_area=_macd_neg_area(mv.first_seg_s0, mv.last_seg_s1)))
                    down_move_settled = True
                else:
                    up_move_hist.append(MoveRecord(
                        direction="up", high=mv.high, low=mv.low,
                        macd_area=_macd_pos_area(mv.first_seg_s0, mv.last_seg_s1)))
                    up_move_settled = True
                ep = mv.high if mv.direction == "up" else mv.low
                if ep <= 0:
                    continue
                r1d, _ = l2_down.detect_settle(l2_down.tree.update(ep))
                r1u, _ = l2_up.detect_settle(l2_up.tree.update(-ep))
                if r1d:
                    l2_flip_long = True
                if r1u:
                    l2_flip_short = True
        if l2_flip_long:
            l2_direction = 1
        if l2_flip_short:
            l2_direction = -1

        bsp_events = snap.bsp_snapshot.events
        buy_cands: list[int] = []; sell_cands: list[int] = []; buy_invalidates: list[int] = []
        for e in bsp_events:
            nm = type(e).__name__; bid = e.bsp_id; side = e.side
            if "Candidate" in nm:
                (buy_cands if side == "buy" else sell_cands).append(bid)
            elif "Invalidate" in nm and side == "buy":
                buy_invalidates.append(bid)
        new_up: list[UpSegRecord] = []
        for e in snap.move_snapshot.events:
            if isinstance(e, MoveSettleV1) and e.direction == "up":
                m = next((mm for mm in moves if mm.seg_start == e.seg_start
                          and mm.direction == "up" and mm.settled), None)
                if m is not None:
                    new_up.append(UpSegRecord(
                        seg_start=m.seg_start, high=m.high, low=m.low,
                        bar_start=m.first_seg_s0, bar_end=m.last_seg_s1,
                        force=_move_force_simple(m),
                        macd_area=_macd_pos_area(m.first_seg_s0, m.last_seg_s1),
                        persistence=m.persistence))
        med_persistence = _median_alive_persistence(l1_down.alive) if new_up else 0.0
        if l1_nr1_up:
            l1_up_ratio = _persistence_ratio(l1_up.alive, 1)
            refined_gate_ok = _refined_gate(l1_up_segs)
        else:
            l1_up_ratio = 0.0; refined_gate_ok = False
        entry_div_ok = (check_divergence(down_move_hist[-1], down_move_hist[-2], "down")
                        if len(down_move_hist) >= 2 else False)
        exit_div_ok = (check_divergence(up_move_hist[-1], up_move_hist[-2], "up")
                       if len(up_move_hist) >= 2 else False)

        e_signals.append(BarSignalV2(
            close=c, l0_r1_down=l0_r1_down, l0_r1_up=l0_r1_up,
            l1_nr1_up=l1_nr1_up, l1_nr1_down=l1_nr1_down,
            l2_flip_long=l2_flip_long, l2_flip_short=l2_flip_short,
            l2_direction=l2_direction,
            buy_cands=tuple(buy_cands), sell_cands=tuple(sell_cands),
            buy_invalidates=tuple(buy_invalidates), new_up_moves=tuple(new_up),
            med_persistence=med_persistence, l1_up_ratio=l1_up_ratio,
            refined_gate_ok=refined_gate_ok, entry_div_ok=entry_div_ok,
            exit_div_ok=exit_div_ok, down_move_settled=down_move_settled,
            up_move_settled=up_move_settled,
            entry_zs_count=0, exit_zs_count=0, entry_persistence_high=False,
            exit_persistence_high=False, type2_buy=False,
            sub_sell_signal=False, sub_buy_signal=False))

        # ── I 分层磁带：每 ladder 的 buy1/sell1/sell_any/buy_any ──
        buy1 = [False] * MAX_LADDER
        sell1 = [False] * MAX_LADDER
        sell_any = [False] * MAX_LADDER
        buy_any = [False] * MAX_LADDER
        # bar/bi（ladder 0/1）：sub-走势 PH proxy（无中枢 → buy1/sell1 恒 False）
        sell_any[LADDER_BAR] = bar_sell; buy_any[LADDER_BAR] = bar_buy
        sell_any[LADDER_BI] = bi_sell; buy_any[LADDER_BI] = bi_buy
        # segment（ladder 2）：笔中枢真实 BSP（525号，中枢承载层下沿）
        buy1[LADDER_SEG] = seg_buy1; sell1[LADDER_SEG] = seg_sell1
        sell_any[LADDER_SEG] = seg_sell_any; buy_any[LADDER_SEG] = seg_buy_any
        # 走势级（ladder 3）：线段中枢真实 BSP
        buy1[LADDER_MOVE] = l1_buy1; sell1[LADDER_MOVE] = l1_sell1
        sell_any[LADDER_MOVE] = l1_sell_any; buy_any[LADDER_MOVE] = l1_buy_any

        # ── 递归层（真实 BSP，取消近似）──
        lower_moves_by_level: dict[int, list] = {1: list(moves)}
        for rs in snap.recursive_snapshots:
            lower_moves_by_level[rs.level_id] = list(rs.moves)
        for rs in snap.recursive_snapshots:
            ladder = rs.level_id + 2
            if ladder >= MAX_LADDER:
                continue
            if ladder > max_ladder:
                max_ladder = ladder
            if not (rs.move_events or rs.zhongshu_events):
                continue  # 该层无变化 → 无新 confirmed
            st = level_bsp.get(rs.level_id)
            if st is None:
                st = _LevelBspTracker(rs.level_id); level_bsp[rs.level_id] = st
            lower = lower_moves_by_level.get(rs.level_id - 1, [])
            b1, s1, sa, ba = st.step(lower, rs.zhongshus, rs.moves)
            buy1[ladder] = b1; sell1[ladder] = s1
            sell_any[ladder] = sa; buy_any[ladder] = ba

        i_signals.append(BarSignalI(
            close=c, buy1=tuple(buy1), sell1=tuple(sell1),
            sell_any=tuple(sell_any), buy_any=tuple(buy_any),
            max_ladder=max_ladder, type2_buy=type2_buy))

        if i - last_progress >= 50_000:
            print(f"    [{i / n * 100:5.1f}%] signal bar {i:,}/{n:,}")
            last_progress = i

    return e_signals, i_signals


# ════════════════════════════════════════════════════════════
# 多 FSM 多重赋格降成本（取消单 FSM，阉割#1/#3/#4）
# ════════════════════════════════════════════════════════════

_FLAT, _ARMED, _LONG = 0, 1, 2


@dataclass
class _Voice:
    """单个降成本级别的声部：独立 FSM + 机动仓 slice（不是方向核心切片）。

    第40课多重赋格"每一重对应一定的资金与筹码""每一层次独立又在整体中"。
    voice 只覆盖 entry 层**以下**的降成本级别——entry 层本身是全仓方向核心（不在 voices）。
    每个 voice 的 FSM own_capital = 机动仓 slice（MANEUVER_RATIO×总仓 均分），
    sub_ratio = _level_trade_fraction(ladder)（级别驱动单笔量，非固定 0.3）。
    """
    ladder: int
    own_capital: float          # 该声部的机动仓 slice（own_capital）
    fsm: CostReductionFSM

    @property
    def recovered(self) -> float:
        return self.fsm.cumulative_recovered

    @property
    def n_diffs(self) -> int:
        return len(self.fsm.completed_short_diffs)


def run_version_i(
    i_signals: list[BarSignalI],
    *,
    floor_ladder: int = MIN_FLOOR_LADDER,
    maneuver_ratio: float = MANEUVER_RATIO,
) -> tuple[list[CompletedTrade], dict]:
    """floor_ladder / maneuver_ratio 可参数化覆盖，便于在同一信号 pass 上跑多变体对比。

    存在论（考据设计点1/2）：核心仓位**全仓**（INITIAL_CAPITAL，方向利润底仓，不做短差）；
    机动仓 = maneuver_ratio×INITIAL_CAPITAL，从总仓划出，在 entry 层以下各降成本级别间
    均分，每级别一个独立 FSM（多声部）。单笔短差量由级别驱动（_level_trade_fraction），
    非固定标量。机动仓是核心仓位的子账户——清仓时其净增益叠加到核心，分母仍是 INITIAL_CAPITAL。
    """
    n = len(i_signals)
    state = _FLAT
    entry_bar = -1; entry_price = 0.0; entry_ladder = -1; arm_bar = -1
    arm_ladder = LADDER_MOVE
    core_shares = 0.0            # 全仓方向核心份额
    voices: list[_Voice] = []
    SUB_EXPIRY = _ef.SUB_EXPIRY
    n_addon = 0

    trades: list[CompletedTrade] = []
    ladder_attribution: dict[int, int] = {}
    ladder_held_bars: dict[int, int] = {}
    # 每级 FSM 累计贡献度（跨所有交易）
    fsm_recovered: dict[int, float] = {}
    fsm_diffs: dict[int, int] = {}
    fsm_short_pnl: dict[int, float] = {}  # 短差净增量（声部终值 − slice 初值）

    def _open(bar_idx: int, price: float, el: int) -> None:
        nonlocal state, entry_bar, entry_price, entry_ladder, voices, core_shares
        entry_bar = bar_idx; entry_price = price; entry_ladder = el
        # 全仓方向核心（不做短差，仅 entry 层 type1 卖点主出场）
        core_shares = INITIAL_CAPITAL / price
        # 降成本级别 = entry 层以下所有 ladder（≥ floor_ladder）。entry 层本身不降成本。
        levels = sorted({k for k in range(floor_ladder, el)})
        maneuver_total = maneuver_ratio * INITIAL_CAPITAL
        slice_cap = maneuver_total / len(levels) if levels else 0.0
        voices = []
        for k in levels:
            f0 = CostReductionFSM.create(
                own_capital=slice_cap, margin_amount=0.0,
                sub_ratio=_level_trade_fraction(k))
            f = transition(f0, FsmEvent(
                FsmEventType.BUY_POINT_CONFIRMED, price=price, level=f"L{k}"))
            voices.append(_Voice(ladder=k, own_capital=slice_cap, fsm=f))
        state = _LONG

    def _close(bar_idx: int, price: float, reason: str) -> None:
        nonlocal state, entry_bar, entry_price, entry_ladder, voices, core_shares
        if entry_price <= 0 or core_shares <= 0:
            state = _FLAT; voices = []; core_shares = 0.0; return
        # 全仓方向核心市值
        total_value = core_shares * price
        n_short_total = 0; cost_basis_min = float("inf")
        for v in voices:
            f = v.fsm
            # 清仓前回补未闭短差
            if (f.active_short_diff is not None and f.active_short_diff.is_open
                    and f.state in _COST_ACTIVE_DIFF_STATES):
                f = transition(f, FsmEvent(
                    FsmEventType.SUB_LEVEL_BUY_POINT, price=price, level="sub"))
            snap = f.snapshot()
            # 机动 slice 终值 = 回收现金 + 挣得股数×价；净增益 = 终值 − slice 初始投入
            # （slice 初始投入已计在核心 INITIAL_CAPITAL 内，故只叠加净增益，避免重复计资本）
            slice_value = snap.cumulative_recovered + snap.total_shares * price
            slice_gain = slice_value - v.own_capital
            total_value += slice_gain
            n_short_total += len(f.completed_short_diffs)
            cost_basis_min = min(cost_basis_min, snap.cost_basis)
            # 贡献度累计
            fsm_recovered[v.ladder] = fsm_recovered.get(v.ladder, 0.0) + snap.cumulative_recovered
            fsm_diffs[v.ladder] = fsm_diffs.get(v.ladder, 0) + len(f.completed_short_diffs)
            fsm_short_pnl[v.ladder] = fsm_short_pnl.get(v.ladder, 0.0) + slice_gain
        pnl_pct = (total_value - INITIAL_CAPITAL) / INITIAL_CAPITAL * 100
        held = bar_idx - entry_bar
        trades.append(CompletedTrade(
            entry_bar=entry_bar, entry_price=entry_price, exit_bar=bar_idx,
            exit_price=price, pnl_pct=round(pnl_pct, 4), exit_reason=reason,
            n_short_diffs=n_short_total,
            cost_basis_at_exit=(0.0 if cost_basis_min == float("inf") else cost_basis_min)))
        ladder_attribution[entry_ladder] = ladder_attribution.get(entry_ladder, 0) + 1
        ladder_held_bars[entry_ladder] = ladder_held_bars.get(entry_ladder, 0) + held
        state = _FLAT; entry_price = 0.0; entry_ladder = -1; voices = []; core_shares = 0.0

    for i in range(n):
        sig = i_signals[i]
        c = sig.close

        if state == _FLAT:
            # 走势级+ confirmed type1 买点（底背驰，趋势=2+中枢内蕴）→ ARM
            hi = -1
            for k in range(FIRST_BSP_LADDER, min(sig.max_ladder + 1, MAX_LADDER)):
                if sig.buy1[k]:
                    hi = k
            if hi >= FIRST_BSP_LADDER:
                state = _ARMED; arm_bar = i; arm_ladder = hi

        elif state == _ARMED:
            for k in range(FIRST_BSP_LADDER, min(sig.max_ladder + 1, MAX_LADDER)):
                if sig.buy1[k] and k > arm_ladder:
                    arm_ladder = k
            # 区间套精确入场：arm_ladder **以下**各级别低吸点（buy_any）或超时。
            # arm_ladder 自身的 buy_any 不作其区间套精度（区间套要求次级别确认在主级别内）；
            # segment 作 entry 时子级别 = bar/bi，走势作 entry 时子级别 = bar/bi/segment。
            do_enter = any(sig.buy_any[k] for k in range(LADDER_BAR, arm_ladder))
            if not do_enter and (i - arm_bar) > SUB_EXPIRY:
                do_enter = True
            if do_enter:
                _open(i, c, arm_ladder)
            elif sig.sell1[arm_ladder]:
                state = _FLAT  # 入场前归属层顶背驰 → 取消
        elif state == _LONG:
            # 主出场：entry 层 confirmed type1 卖点（顶背驰）→ 全仓清出
            if sig.sell1[entry_ladder]:
                _close(i, c, f"exit_{ladder_name(entry_ladder)}_type1sell")
            else:
                # 多重赋格降成本：各声部（entry 层以下级别）独立 FSM，
                # 触发 = 本级别 sell_any(高抛) / buy_any(低吸)。核心仓位不参与短差。
                for v in voices:
                    f = v.fsm
                    has_open = (f.active_short_diff is not None
                                and f.active_short_diff.is_open)
                    if sig.sell_any[v.ladder] and not has_open and f.state in _COST_OPEN_STATES:
                        v.fsm = transition(f, FsmEvent(
                            FsmEventType.SUB_LEVEL_SELL_POINT, price=c, level="sub"))
                    elif sig.buy_any[v.ladder] and has_open and f.state in _COST_ACTIVE_DIFF_STATES:
                        v.fsm = transition(f, FsmEvent(
                            FsmEventType.SUB_LEVEL_BUY_POINT, price=c, level="sub"))
                if sig.type2_buy:
                    n_addon += 1

    if state == _LONG and core_shares > 0:
        _close(n - 1, i_signals[-1].close, "eod_close")

    avg_held = {
        k: round(ladder_held_bars[k] / ladder_attribution[k], 1)
        for k in ladder_attribution if ladder_attribution[k]
    }
    fsm_contrib = {
        ladder_name(k): {
            "recovered_cash": round(fsm_recovered.get(k, 0.0), 1),
            "n_short_diffs": fsm_diffs.get(k, 0),
            "slice_net_gain": round(fsm_short_pnl.get(k, 0.0), 1),
        }
        for k in sorted(set(fsm_recovered) | set(fsm_diffs))
    }
    return trades, {
        "ladder_attribution": {ladder_name(k): v for k, v in ladder_attribution.items()},
        "ladder_avg_held_bars": {ladder_name(k): v for k, v in avg_held.items()},
        "fsm_contribution": fsm_contrib,
        "addon_2buy_marks": n_addon,
        "maneuver_ratio": maneuver_ratio,
        "min_floor_ladder": floor_ladder,
    }


# ════════════════════════════════════════════════════════════
# 扩展指标（compute_metrics + 盈亏比 + 按年）
# ════════════════════════════════════════════════════════════

def extended_metrics(trades: list[CompletedTrade], years: list | None = None) -> dict:
    m = dict(compute_metrics(trades))
    pnls = [t.pnl_pct for t in trades]
    gross_win = sum(p for p in pnls if p > 0)
    gross_loss = -sum(p for p in pnls if p < 0)
    m["profit_factor"] = (gross_win / gross_loss) if gross_loss > 0 else float("inf")
    by_year: dict[int, dict] = {}
    if years is not None:
        for t in trades:
            if 0 <= t.entry_bar < len(years):
                y = years[t.entry_bar]
                yb = by_year.setdefault(y, {"n": 0, "compound": 1.0, "wins": 0})
                yb["n"] += 1
                yb["compound"] *= 1 + t.pnl_pct / 100
                if t.pnl_pct > 0:
                    yb["wins"] += 1
        for y in by_year:
            yb = by_year[y]
            yb["return_pct"] = round((yb["compound"] - 1) * 100, 2)
            yb["win_rate"] = round(yb["wins"] / yb["n"] * 100, 1) if yb["n"] else 0.0
            del yb["compound"]
    m["by_year"] = by_year
    return m


# ════════════════════════════════════════════════════════════
# 单标的管线
# ════════════════════════════════════════════════════════════

def process_symbol(symbol: str) -> tuple[str, dict]:
    print(f"\n{'=' * 60}\n  {symbol} — 版本 I 完整版回测（E vs I）\n{'=' * 60}")
    print(f"  config: MANEUVER_RATIO={MANEUVER_RATIO}  MIN_FLOOR_LADDER={MIN_FLOOR_LADDER}"
          f" ({ladder_name(MIN_FLOOR_LADDER)})")
    opens, highs, lows, closes, years = load_symbol(symbol)
    n = len(closes)
    bh = (closes[-1] - closes[0]) / closes[0] * 100

    t0 = time.time()
    e_signals, i_signals = compute_signals_i(opens, highs, lows, closes)
    sig_elapsed = time.time() - t0
    print(f"  [{symbol}] {n:,} bars  BH={bh:+.2f}%  信号层 {sig_elapsed:.1f}s")

    e_trades, _ = run_swing_trading(e_signals, MODE_NONE)
    em = extended_metrics(e_trades, years)
    print(f"  [{symbol}/E] 交易={em['n']:3d} 胜率={em['win_rate']:4.0f}% "
          f"复利={em['total_compound']:+10.2f}% 超额={em['total_compound']-bh:+10.2f}% "
          f"MDD={em['max_dd']:+.1f}% 夏普={em['sharpe']:+.3f} 盈亏比={em['profit_factor']:.2f}")

    # I 多 floor 变体（同一信号 pass）：含 bar（默认/遵指令）vs 线段起 vs 缠师走势级口径。
    variants: dict[str, dict] = {}
    for tag, floor in (("I_bar0", 0), ("I_seg2", LADDER_SEG), ("I_move3", LADDER_MOVE)):
        i_trades, i_extra = run_version_i(i_signals, floor_ladder=floor)
        im = extended_metrics(i_trades, years)
        print(f"  [{symbol}/{tag} floor={ladder_name(floor)}] 交易={im['n']:3d} "
              f"胜率={im['win_rate']:4.0f}% 复利={im['total_compound']:+10.2f}% "
              f"超额={im['total_compound']-bh:+10.2f}% MDD={im['max_dd']:+.1f}% "
              f"夏普={im['sharpe']:+.3f} 盈亏比={im['profit_factor']:.2f} 降成本笔={im['n_with_cr']}")
        print(f"      归属: {i_extra['ladder_attribution']}  FSM贡献: {i_extra['fsm_contribution']}")
        variants[tag] = {"metrics": im, "extra": i_extra}

    return symbol, {
        "n_bars": n, "bh": bh, "first": closes[0], "last": closes[-1],
        "E": em, "variants": variants,
        "maneuver_ratio": MANEUVER_RATIO,
        "sig_elapsed": round(sig_elapsed, 1),
    }


def main() -> None:
    order = ["OKLO"]
    only = os.environ.get("BT_SYMBOLS")
    if only:
        order = [s.strip().upper() for s in only.split(",")]

    results: dict = {}
    t_all = time.time()
    for sym in order:
        _, out = process_symbol(sym)
        results[sym] = out
        _write_report(results)
        (DATA_DIR / "fugue_version_i_results.json").write_text(
            json.dumps(results, indent=2, ensure_ascii=False, default=str))
    print(f"\n总耗时 {time.time() - t_all:.1f}s（{len(order)} 标的）")
    print("报告：fugue_version_i_results.md / .json")


# ════════════════════════════════════════════════════════════
# 报告
# ════════════════════════════════════════════════════════════

_VAR_TAGS = (("I_bar0", "含bar(遵指令)"), ("I_seg2", "线段起"), ("I_move3", "缠师走势级口径"))


def _write_report(results: dict) -> None:
    L: list[str] = []
    L.append("# 版本 I（完整版）— 全仓 + 每级独立 BSP + 多 FSM 多重赋格降成本\n")
    L.append(
        "> 取消 6 阉割：多 FSM（每级独立 cost_basis）/ 每中枢层真实 confirmed type1/2/3 BSP"
        "（经 `per_level_bsp` 适配层：走势级 `confirmed_bsp_level1`、递归层 "
        "`confirmed_bsp_for_level`，**非 move-settle 近似**）/ 含 bar 级（可配置下限）/ "
        "完整跑高层 / 区分机动仓占比 vs 级别驱动单笔量 / 删除伪 267 课号引用。\n")
    L.append(
        f"> 配置：`MANEUVER_RATIO={MANEUVER_RATIO}`（原文'例如 1/10'机动仓占比）；单笔短差量"
        "= `_level_trade_fraction`（级别驱动，非固定 0.3）。认识论 **L2**；力度口径=价格"
        "振幅 fallback（非 MACD，因 447K 上 orch-MACD 为 O(N²) 不可行）。\n")
    L.append(
        "> ⚠ **缠师原文冲突（不隐藏）**：第53/35/31课主张降成本最小级别 ≥5分钟（≈线段/"
        "走势级），反对 bar 级短差。下方三 floor 变体让数据经验裁决：`I_bar0`=含 bar（遵"
        "指令#3），`I_move3`=缠师走势级口径。若 `I_bar0` < `I_move3` → 经验印证缠师原文。\n")
    L.append("## E vs I（多 floor 变体）对照\n")
    L.append("| 标的 | bars | BH% | 策略 | 复利% | 超额 | 夏普 | MDD | 盈亏比 | 笔 | 降成本笔 |")
    L.append("|------|------|-----|------|-------|------|------|-----|--------|-----|----------|")
    for s in results:
        r = results[s]; e = r["E"]; bh = r["bh"]
        L.append(
            f"| {s} | {r['n_bars']:,} | {bh:+.1f} | E基线 | {e['total_compound']:+.1f} | "
            f"{e['total_compound']-bh:+.1f} | {e['sharpe']:+.2f} | {e['max_dd']:+.1f} | "
            f"{e['profit_factor']:.2f} | {e['n']} | — |")
        for tag, desc in _VAR_TAGS:
            v = r["variants"].get(tag)
            if not v:
                continue
            ii = v["metrics"]
            L.append(
                f"| {s} | | | {tag}({desc}) | {ii['total_compound']:+.1f} | "
                f"{ii['total_compound']-bh:+.1f} | {ii['sharpe']:+.2f} | {ii['max_dd']:+.1f} | "
                f"{ii['profit_factor']:.2f} | {ii['n']} | {ii['n_with_cr']} |")
    L.append("")
    L.append("## 级别归属分布（实测，揭示高层稀疏有效域）— I_bar0\n")
    L.append("| 标的 | 归属(级别→笔数) | 平均持仓bar | 2买标记 |")
    L.append("|------|----------------|------------|--------|")
    for s in results:
        ex = results[s]["variants"]["I_bar0"]["extra"]
        L.append(f"| {s} | {ex['ladder_attribution']} | {ex['ladder_avg_held_bars']} | "
                 f"{ex['addon_2buy_marks']} |")
    L.append("")
    L.append("## 每级 FSM 独立贡献度（多重赋格各声部）— I_bar0\n")
    L.append("> `recovered_cash`=该级短差累计回收现金；`n_short_diffs`=完成短差次数；"
             "`slice_net_gain`=该级机动 slice 终值−slice 初始投入（短差净增益）。bar 级若巨负"
             " → 经验印证缠师'太小级别短差无意义'。\n")
    L.append("| 标的 | 级别 | 回收现金 | 短差次数 | slice净增益 |")
    L.append("|------|------|---------|---------|-----------|")
    for s in results:
        contrib = results[s]["variants"]["I_bar0"]["extra"]["fsm_contribution"]
        for lvl, d in contrib.items():
            L.append(f"| {s} | {lvl} | {d['recovered_cash']:+.0f} | {d['n_short_diffs']} | "
                     f"{d['slice_net_gain']:+.0f} |")
    L.append("")
    L.append("## 按年收益（有 dates）— E vs I_move3（缠师口径）\n")
    for s in results:
        e = results[s]["E"]
        ii = results[s]["variants"]["I_move3"]["metrics"]
        if ii.get("by_year"):
            L.append(f"### {s}\n")
            L.append("| 年 | E笔 | E收益% | E胜率 | I笔 | I收益% | I胜率 |")
            L.append("|----|-----|--------|-------|-----|--------|-------|")
            yrs = sorted(set(ii["by_year"]) | set(e.get("by_year", {})))
            for y in yrs:
                ey = e.get("by_year", {}).get(y, {}); iy = ii["by_year"].get(y, {})
                L.append(
                    f"| {y} | {ey.get('n',0)} | {ey.get('return_pct',0):+.1f} | "
                    f"{ey.get('win_rate',0):.0f}% | {iy.get('n',0)} | "
                    f"{iy.get('return_pct',0):+.1f} | {iy.get('win_rate',0):.0f}% |")
            L.append("")
    OUTPUT_MD.write_text("\n".join(L))


if __name__ == "__main__":
    main()
