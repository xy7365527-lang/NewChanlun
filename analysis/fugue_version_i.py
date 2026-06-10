"""版本 I（完整版）— 满仓 + 每级独立 BSP + 共享仓位多声部并发短差降成本。

═══════════════════════════════════════════════════════════════════════
本次重写：取消 6 个阉割（用户指令）
═══════════════════════════════════════════════════════════════════════
1. **共享仓位多声部并发短差**（用户 2026-06-09 裁定，否定旧 slice 模型）：入场一次满仓
   100%（total_shares = INITIAL_CAPITAL/price，唯一一份共享仓位）；entry 层以下所有降成本
   级别**全部作用于同一仓位**，每级别独立维护自己的短差生命周期（多槽 `_SharedFugue.active`），
   每个短差闭合时 profit 直接写入**共享 cost_basis**。**已废弃的 slice 模型**（核心仓全仓
   不做短差 + 机动仓 MANEUVER_RATIO 切 disjoint slice 分给各级别独立 FSM）缺陷：短差只作用
   于 ~10% 机动仓的各 slice，主仓 90% 成本从不下降，且 slice 限额掩盖了降成本的真实负 alpha。
   新模型让短差以满仓为基准 → 降成本的有效域边界（bar 级噪音的毁灭性）被诚实暴露（见回测）。
   注：通用 `CostReductionFSM` 是单槽单循环，无法承载多级别并发短差，故 Version I 在本文件内
   实现共享账本 `_SharedFugue`（复用 `ShortDiffCycle` + profit 公式），零污染全库 FSM 语义。
2. **每层独立 BSP**（经 per_level_bsp 适配层，任务卡强制）：每个 **中枢承载层**（走势 L1 +
   递归 L≥2）都产生真实 confirmed type1/2/3 买卖点——走势级用 `confirmed_bsp_level1`
   （透传引擎 `snap.bsp_snapshot`），递归层用 `confirmed_bsp_for_level`（per_level_bsp 内部
   组合 `buysellpoints_from_level` + `divergences_from_moves_v1`，525号组件来源无关性 +
   well-formedness 索引证明）。**取消 "move settle + 背驰" 近似，不在本文件重复实现适配器**。
3. **不排除 bar 级**：bar/笔/线段级以最小级别下限参数 `MIN_FLOOR_LADDER` 控制（默认 0
   = 含 bar），不再硬编码"bar 级不作降成本触发"的主观排除。⚠ 见下方"缠师原文冲突"。
4. **认真处理高层**：递归层完整跑 BSP，不因稀疏退化。稀疏性作为回测结果（归属分布 +
   每级 FSM 贡献度）自然呈现。
5. **操作规模从走势结构来——按子级别数均分总仓位**（用户 2026-06-09 裁定的简单方案）：
   入场满仓后，entry 层以下 N_sub 个子级别**每个操作 1/N_sub 满仓**（`_SharedFugue.level_frac`，
   `_open` 处算，Σ frac=100%）。这把单笔规模从旧 `_level_trade_fraction` 的 base=0.25/step=0.10
   魔数（无原文数值依据的 L0 引申）改为有明确语义的归一化分配。⚠ **均分不防爆仓**（L2 实测，
   诚实声明）：降成本阶段是虚拟价差收集层（股数守恒、不减 total_shares），盈亏独立叠加于满仓
   敞口且无下限，bar 级 PH settle 噪音信号下累计净负击穿本金 → I_bar0 仍爆仓（印证缠师"太小
   级别短差有害"，floor≥2 才正收益域）。爆仓根因在信号层 + 守恒律建模，非仓位规模层。理想方案
   （按各级别中枢振幅占价格比例定规模）待 L2 数据确立结构比例后接入。`MANEUVER_RATIO` 在共享
   均分模型下已无作用（slice 切分已废，满仓共享 + 均分取代），保留为接口兼容参数。
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
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

from newchan.a_macd import OnlineMacdState  # noqa: E402
from newchan.events import MoveSettleV1, SegmentSettleV1  # noqa: E402
from newchan.orchestrator.recursive import RecursiveOrchestrator  # noqa: E402
# 共享仓位多声部架构（用户 2026-06-09 裁定）：复用 ShortDiffCycle（短差循环 +
# profit 公式，承自 267号），但**不用** CostReductionFSM——通用 FSM 是单槽单循环
# （active_short_diff: ShortDiffCycle | None），无法承载"多级别并发短差作用于同一仓位"。
# Version I 的共享账本（_SharedFugue）在本文件内实现，零污染全库 FSM 的单槽语义。
from newchan.trading.cost_reduction_fsm import ShortDiffCycle  # noqa: E402
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

# ── 可配置参数 ──
# 操作规模从走势结构来（用户 2026-06-09 裁定的简单方案）：入场满仓后，entry 层以下 N_sub
# 个子级别**按级别数均分总仓位**——每级操作 1/N_sub 满仓（见 `_open` 计算 `level_frac`，
# Σ frac=100%）。这是有明确语义的归一化分配，替代旧 base/step 魔数。⚠ 均分不防 bar 级爆仓
# （根因在信号层 + 守恒律建模，见 `_open` 注释与报告）。理想方案（按各级别中枢振幅占价格比例
# 定操作规模——原文 chan99/0033"量基本只和级别有关"）待 L2 数据确立结构比例后接入。
#
# MANEUVER_RATIO 在共享均分模型下**已无作用**：slice 切分已废（满仓共享 + 均分取代），
# 保留为接口/报告兼容参数。原文"例如 1/10"（blog/031 26行）是机动仓占总仓比例，在 slice
# 版有意义；共享版满仓暴露不留机动仓，故该参数退化为纯标记。
MANEUVER_RATIO = float(os.environ.get("BT_MANEUVER_RATIO", "0.1"))
# 降成本最小级别下限（ladder）。默认 0 = 含 bar（遵指令#3）。
# ⚠ 缠师原文（第53/35/31课）主张 ≥ 线段/5分钟级（≈ladder 2~3）。设 env BT_FLOOR_LADDER=3
#   可恢复缠师口径，对照 bar 级贡献度判断噪音。
MIN_FLOOR_LADDER = int(os.environ.get("BT_FLOOR_LADDER", "0"))

PERSIST_MED_WINDOW = 50


# P5 震荡腿（O 模式）账本键偏移：osc 腿与主腿在 _SharedFugue.active 中是独立槽，
# 键 = ladder + OSC_KEY_OFFSET（整数键，与 stopped/frozen/completed 归因兼容）。
OSC_KEY_OFFSET = 100


def ladder_name(ladder: int) -> str:
    """ladder → 人类可读级别名（用于报告与贡献度归属）。"""
    if ladder >= OSC_KEY_OFFSET:
        return f"osc:{ladder_name(ladder - OSC_KEY_OFFSET)}"
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


def _scan_bsp_events(bsps, seen: set) -> tuple[bool, bool, bool, bool, tuple]:
    """扫描 BSP 列表，按 (kind, side, seg_idx, confirmed) 去重，返回布尔信号 + 新事件流。

    返回 (buy1, sell1, sell_any, buy_any, events)：

      buy1  = 新 confirmed type1 买点（底背驰 → 进场/归属）
      sell1 = 新 confirmed type1 卖点（顶背驰 → entry 层主出场）
      sell_any / buy_any = 任意 type 新 confirmed 卖/买点（次级别高抛/低吸触发）
      events = 本次新增事件元组流，每个事件 =
               (kind, side, seg_idx, confirmed, center_seg_start, center_zd,
                center_zg, price)

    去重键含 confirmed（卖飞修复）：同一 BSP 的 candidate（左侧形成）与 confirmed
    （右侧确认）是**两个生命周期事件**，分别入流——P3 预回补消费 candidate type3，
    P2 硬回补消费 confirmed type3（区间套反向应用：type3 = 纤维死亡边界）。

    **布尔流 bit-exact 证明**：布尔仅由新 confirmed 事件置位。confirmed BSP 的
    新键 (k,s,i,True) 首现 bar ≡ 旧键 (k,s,i) 在 confirmed-only 扫描下的首现 bar
    （旧扫描在 add 前跳过非 confirmed → 旧 seen 只含 confirmed 键）。candidate
    事件不参与布尔 → 不消费 events 的旧变体逐位不变。
    """
    b1 = s1 = sa = ba = False
    events: list = []
    for bp in bsps:
        key = (bp.kind, bp.side, bp.seg_idx, bp.confirmed)
        if key in seen:
            continue
        seen.add(key)
        events.append((bp.kind, bp.side, bp.seg_idx, bp.confirmed,
                       bp.center_seg_start, bp.center_zd, bp.center_zg, bp.price))
        if not bp.confirmed:
            continue
        if bp.side == "buy":
            ba = True
            if bp.kind == "type1":
                b1 = True
        else:
            sa = True
            if bp.kind == "type1":
                s1 = True
    return b1, s1, sa, ba, tuple(events)


class _LevelBspTracker:
    """单个递归层 (level_id≥2) 的 BSP 跟踪器：per_level_bsp 全量 + 去重检测新增。

    per_level_bsp.confirmed_bsp_for_level 返回全量 BSP 列表；本类按
    (kind,side,seg_idx,confirmed) 去重（candidate/confirmed 分别入流），布尔只报
    本 bar **新出现**的 confirmed 买卖点（bit-exact，见 _scan_bsp_events），事件流
    附带 kind/center 结构化字段（卖飞修复：区间套反向应用配对消费）。
    只在该层 move/zhongshu 变化时调用（move_events/zhongshu_events 非空）。
    """

    __slots__ = ("_level_id", "_seen")

    def __init__(self, level_id: int) -> None:
        self._level_id = level_id
        self._seen: set = set()

    def step(
        self, prev_level_moves: list, level_zhongshus: list, level_moves: list,
    ) -> tuple[bool, bool, bool, bool, tuple]:
        """重算 BSP，返回本 bar 新增的 (buy1, sell1, sell_any, buy_any, events)。"""
        bsps = confirmed_bsp_for_level(
            prev_level_moves, level_zhongshus, level_moves, self._level_id)
        return _scan_bsp_events(bsps, self._seen)


class _TrendBspTracker:
    """走势级（引擎 level=1）BSP 跟踪器：confirmed_bsp_level1 透传 + 去重检测新增。"""

    __slots__ = ("_seen",)

    def __init__(self) -> None:
        self._seen: set = set()

    def step(self, bsp_snapshot) -> tuple[bool, bool, bool, bool, tuple]:
        return _scan_bsp_events(confirmed_bsp_level1(bsp_snapshot), self._seen)


class _BiZhongshuBspTracker:
    """segment 级（笔中枢，525号）BSP 跟踪器：全量重算 + 去重检测新增。

    把 ladder 2（segment）从 PH proxy 升级为真实中枢承载层：笔 → 笔中枢 →
    笔级别走势 → type1/2/3。`confirmed_bsp_bi_zhongshu` 每次全量重算（笔引擎是 O(N²)，
    与既有递归层 tracker 同构），本类按 (kind,side,seg_idx,confirmed) 去重，布尔只报
    本 bar **新出现**的 confirmed 买卖点（bit-exact），事件流附带结构化字段。
    仅在 strokes 增长（新 confirmed 笔）时调用，避免无谓重算。
    """

    __slots__ = ("_seen",)

    def __init__(self) -> None:
        self._seen: set = set()

    def step(self, strokes: list) -> tuple[bool, bool, bool, bool, tuple]:
        return _scan_bsp_events(confirmed_bsp_bi_zhongshu(strokes), self._seen)


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
    # ── 结构化 BSP 事件流（级别隔离，用户 2026-06-10 裁定：不混流）──
    # bsp_events[ladder] = 该 ladder 本 bar 新事件流（每级别独立，配对逻辑在
    # 各 ladder 内独立执行、不跨 ladder）；每个事件 =
    # (kind, side, seg_idx, confirmed, center_seg_start, center_zd, center_zg, price)
    # candidate/confirmed 分别入流，事件携带中枢锚点（center_seg_start + zd/zg）。
    # 默认 ()：不消费 events 的旧路径（含旧构造方，如 m1_i_rust_engine）bit-exact；
    # 事件磁带恒为 MAX_LADDER 长元组（空 bar 用共享单例 NO_LADDER_EVENTS）。
    bsp_events: tuple = ()
    # ── 背驰事件流（有机赋格 §5.1，organic_signals 产出；级别隔离同 bsp_events）──
    # div_events[ladder] = 该 ladder 本 bar 新背驰事件流，每个事件 =
    # (kind, direction, side, seg_idx, force_a, force_c, price)：
    #   kind ∈ {"trend","consolidation"}（引擎原生透传——盘整背驰首次可见，E10 完成）；
    #   direction ∈ {"up","down"}（背驰所在 move 方向；引擎 top→up / bottom→down）；
    #   side：up→"sell"（向上段力度衰竭=卖出语义）/ down→"buy"；
    #   seg_idx = 背驰段锚（div.seg_c_end，去重键成分）；force_a/force_c 透传；
    #   price = 背驰段端点价（buy→段 low / sell→段 high，与 type1 BSP price 同构）。
    # 默认 ()：不消费该字段的全部现有路径（含 run_version_i 的 P1-P7）逐位不变。
    div_events: tuple = ()
    # up_move_settled[ladder] = 该 ladder 本 bar 是否有新向上 move settle
    # （FatigueMonitor 清空衰竭证据用，§5.3：创新动力 = 衰竭被市场否定）。默认 ()。
    up_move_settled: tuple = ()


# 事件磁带空 bar 共享单例（可按 ladder 索引，零每-bar 分配）。
NO_LADDER_EVENTS: tuple = ((),) * MAX_LADDER
# 背驰磁带 / move-settle 磁带空 bar 共享单例（organic_signals 消费）。
NO_LADDER_DIVS: tuple = ((),) * MAX_LADDER
NO_UP_SETTLED: tuple = (False,) * MAX_LADDER


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
        ev_by_ladder: dict[int, tuple] = {}   # ladder → 本 bar 新事件流（级别隔离，不混流）
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
            # 笔中枢级 BSP（ladder 2）：仅在 strokes 增长时重算
            seg_buy1, seg_sell1, seg_sell_any, seg_buy_any, seg_events = \
                bi_zhongshu_bsp.step(strokes)
            if seg_events:
                ev_by_ladder[LADDER_SEG] = seg_events

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
        l1_buy1, l1_sell1, l1_sell_any, l1_buy_any, trend_events = \
            trend_bsp.step(snap.bsp_snapshot)
        if trend_events:
            ev_by_ladder[LADDER_MOVE] = trend_events
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
            b1, s1, sa, ba, lvl_events = st.step(lower, rs.zhongshus, rs.moves)
            buy1[ladder] = b1; sell1[ladder] = s1
            sell_any[ladder] = sa; buy_any[ladder] = ba
            if lvl_events:
                ev_by_ladder[ladder] = lvl_events

        if ev_by_ladder:
            rows = [()] * MAX_LADDER
            for lad, evs in ev_by_ladder.items():
                rows[lad] = evs
            bsp_events = tuple(rows)
        else:
            bsp_events = NO_LADDER_EVENTS
        i_signals.append(BarSignalI(
            close=c, buy1=tuple(buy1), sell1=tuple(sell1),
            sell_any=tuple(sell_any), buy_any=tuple(buy_any),
            max_ladder=max_ladder, type2_buy=type2_buy,
            bsp_events=bsp_events))

        if i - last_progress >= 50_000:
            print(f"    [{i / n * 100:5.1f}%] signal bar {i:,}/{n:,}")
            last_progress = i

    return e_signals, i_signals


# ════════════════════════════════════════════════════════════
# 共享仓位多声部并发短差（用户 2026-06-09 裁定：否定 slice 模型）
# ════════════════════════════════════════════════════════════
#
# 旧 slice 模型（已废）：核心仓全仓不做短差 + 机动仓 MANEUVER_RATIO 切成 disjoint slice
#   分给各级别独立 FSM。缺陷：短差只作用于 ~10% 机动仓的各 slice，主仓 90% 成本从不下降，
#   偏离缠论降成本本意（降的是主仓成本，非某个小 slice 的成本）。
#
# 新共享模型（本次）：所有级别的短差作用于**同一个满仓仓位**：
#   1. 入场一次满仓 100%（total_shares = INITIAL_CAPITAL / price，共享）。
#   2. 多级别并发：每个 ladder 独立维护自己的短差生命周期（开/未开/待回补），
#      用 dict[ladder → (ShortDiffCycle, was_earning)] 多槽承载（取代单槽 active_short_diff）。
#   3. 每个短差闭合时 profit 直接写入**共享 cost_basis**：cost_basis -= profit / total_shares。
#      cost_basis ≤ 0 → 全仓进入挣股数阶段（earning）。
#
# 两阶段守恒律（承自 cost_reduction_fsm 267号，复用 ShortDiffCycle.profit）：
#   - 降成本阶段（cost_basis>0）：股数守恒——价差 profit 落袋（cumulative_recovered）+ 降
#     共享 cost_basis，total_shares 不变（满仓上的虚拟高抛低吸价差收集器，多级别并发独立）。
#   - 挣股数阶段（cost_basis≤0）：金额守恒——卖 V 买 V，total_shares 净增，cost_basis 锁 0。
#   守恒律由短差 **open 时的阶段**决定（was_earning 标记），不由 close 时的全局阶段决定——
#   否则降成本阶段开的短差在 earning 阶段 close 会凭空增股（状态不一致 bug）。

_FLAT, _ARMED, _LONG = 0, 1, 2

STOP_FRAC = 0.02  # 硬止损阈值 2%（用户指令）

# P4 振幅过滤阈值 θ：卖价距中枢上沿 ZG 的相对深度下限（中枢中心定理保证的最小
# 回归空间）。默认 1%——根因诊断（v2_selloff_root_cause）：<1% 的回调是次级别
# 买点信号无法现实捕捉的噪音级回调（θ=1% 下趋势太强占卖飞 58.95%）。
THETA_AMP = float(os.environ.get("BT_THETA_AMP", "0.01"))


@dataclass(frozen=True)
class PairingConfig:
    """区间套反向应用的短差配对规则（P1-P5 消融轴）。

    理论框架：底空间 = 本级别中枢 [ZD,ZG] × lifetime；纤维 = lifetime 内的次级别
    走势完成点；短差合法性 = 两腿在同一纤维（中枢存活）；正期望 = 中枢中心定理的
    回归保证（定义性质）；type3 = 纤维死亡边界 → 立即回补，不是低吸信号。

    根因（v2_selloff_root_cause + 配对诊断）：旧 FSM 盲等"下一个任何买点"
    （buy_any），不区分 kind、不锚定中枢——type1卖→type1买胜率 78.9%，
    type1卖→type3买卖飞率 67.9%（type3 买点是中枢终结讣告，不是低吸信号）。
    """

    open_kinds: tuple = ("type1",)   # 开腿卖点 kind 白名单（P4+ 加 type2；()=无离开段腿）
    hard_type3: bool = False         # P2+：confirmed type3 买 → 硬回补 + 冻结该级别
    pre_type3: bool = False          # P3+：candidate type3 买 → 预回补（左侧逃逸）
    center_gate: bool = False        # P4+：中枢存活 + 盘背位置（c>ZG）+ 振幅≥θ
    theta_amp: float = THETA_AMP     # P4+ 振幅阈值（(c−ZG)/c 下限）
    same_center_close: bool = False  # P4+：type1 买正常回补须同锚中枢（同一纤维）
    osc_mode: bool = False           # P5+：域腿（中枢震荡腿：锚 ZG 高抛 / 锚 ZD 低吸）
    # P6+（用户 2026-06-10 裁定：结构域配对）：域腿买回也用次级别走势定位——
    # 买回 = 次级别买点（buy_any[ladder−1]）×价格 ≤ 锚中枢 ZD（两腿同构：次级别
    # 走势完成点 × 锚域边界）。False（P5）= 纯价格触线 c ≤ 锚 ZD。
    osc_buy_sub: bool = False


PAIRING_VARIANTS: dict[str, PairingConfig | None] = {
    "I_seg2": None,  # 基线（现状：sell_any 开 / buy_any 平，盲配对）
    "P1": PairingConfig(),
    "P2": PairingConfig(hard_type3=True),
    "P3": PairingConfig(hard_type3=True, pre_type3=True),
    "P4": PairingConfig(hard_type3=True, pre_type3=True,
                        open_kinds=("type1", "type2"), center_gate=True,
                        same_center_close=True),
    "P5": PairingConfig(hard_type3=True, pre_type3=True,
                        open_kinds=("type1", "type2"), center_gate=True,
                        same_center_close=True, osc_mode=True),
    # P6 = 纯结构域配对（用户范式）：每级别短差只在自己的结构域（锚中枢
    # [ZD,ZG] × lifetime）内操作，两腿都用次级别走势定位；无离开段腿。
    "P6": PairingConfig(open_kinds=(), hard_type3=True, pre_type3=True,
                        osc_mode=True, osc_buy_sub=True),
    # P7 = P6 域腿 + P4 离开段腿（两类腿并发，各自独立槽）。
    "P7": PairingConfig(hard_type3=True, pre_type3=True,
                        open_kinds=("type1", "type2"), center_gate=True,
                        same_center_close=True, osc_mode=True, osc_buy_sub=True),
}


@dataclass
class _SharedFugue:
    """共享仓位 + 多声部并发短差账本（用户裁定的正确多重赋格架构）。

    所有级别短差作用于同一满仓仓位（total_shares / cost_basis 共享，唯一一份）；
    每个 ladder 在 `active` 中独立持有自己的开放短差（多槽，取代通用 FSM 的单槽）。
    """
    entry_price: float
    total_shares: float
    cost_basis: float
    level_frac: float = 0.0                      # 每个降成本级别均分的总仓比例（1/N_sub）
    cumulative_recovered: float = 0.0
    earning: bool = False                       # cost_basis≤0 后全局挣股数阶段
    # ladder → (ShortDiffCycle, was_earning, anchor)；anchor=None（旧路径）或
    # (center_seg_start, center_zg, sell_kind)——开腿卖点所属中枢锚（区间套反向应用：
    # 底空间 = 本级别中枢 [ZD,ZG] × lifetime，短差合法性 = 两腿在同一纤维）。
    active: dict = field(default_factory=dict)
    completed: list = field(default_factory=list)  # [(ladder, 已闭合 ShortDiffCycle), ...]
    stopped: set = field(default_factory=set)   # stop_mode B 冻结的 ladder
    # ── 逐笔短差诊断 trace（opt-in；trace=None 时零开销零数值影响，bit-exact）──
    trace: list | None = None                   # 完整短差记录列表（run_version_i 注入）
    _diff_open: dict = field(default_factory=dict)  # ladder → (sell_bar, sell_price)（仅 trace 时填）

    def open_diff(self, ladder: int, frac: float, sell_price: float, bar: int = -1,
                  anchor: tuple | None = None) -> None:
        """某级别高抛：该级别无开放短差时开启一个短差（基于共享满仓）。

        降成本阶段=虚拟（股数守恒，不动 total_shares）；挣股数阶段=真实减仓。
        `bar` 仅用于 trace（默认 -1，无影响）。`anchor` = 开腿卖点的中枢锚
        （配对模式注入；旧路径 None，数值零影响）。
        """
        if ladder in self.active or ladder in self.stopped:
            return
        shares = self.total_shares * frac
        if shares <= 0:
            return
        cyc = ShortDiffCycle(level=f"L{ladder}", shares=shares, sell_price=sell_price)
        self.active[ladder] = (cyc, self.earning, anchor)
        if self.earning:
            self.total_shares -= shares  # 挣股数阶段真实减仓
        if self.trace is not None:
            self._diff_open[ladder] = (bar, sell_price)

    def close_diff(self, ladder: int, buy_price: float, bar: int = -1) -> None:
        """某级别低吸：闭合该级别短差，profit 写入共享 cost_basis。

        守恒律由 open 时的阶段（was_earning）决定，保证 open/close 同律。
        `bar` 仅用于 trace（默认 -1，无影响）。
        """
        rec = self.active.get(ladder)
        if rec is None or buy_price <= 0:
            return
        cyc, was_earning, _anchor = rec
        cb_before = self.cost_basis
        shares_before = self.total_shares
        closed = ShortDiffCycle(
            level=cyc.level, shares=cyc.shares, sell_price=cyc.sell_price,
            buy_price=buy_price, is_open=False)
        if was_earning:
            # 金额守恒：卖出金额回补，total_shares 净增（买价>卖价则减股），cost_basis 锁 0
            self.total_shares += cyc.shares * cyc.sell_price / buy_price
        else:
            # 股数守恒：价差 profit 落袋 + 降共享 cost_basis
            profit = closed.profit
            self.cumulative_recovered += profit
            if self.total_shares > 0:
                self.cost_basis -= profit / self.total_shares
            if self.cost_basis <= 0:
                self.cost_basis = 0.0
                self.earning = True
        del self.active[ladder]
        self.completed.append((ladder, closed))
        if self.trace is not None:
            sell_bar, _sp = self._diff_open.pop(ladder, (-1, cyc.sell_price))
            self.trace.append({
                "ladder": ladder,
                "sell_bar": sell_bar,
                "sell_price": cyc.sell_price,
                "buy_bar": bar,
                "buy_price": buy_price,
                "shares": cyc.shares,
                "diff": (cyc.sell_price - buy_price),      # 正=低吸成功 负=卖飞
                "profit": closed.profit,                    # diff × shares
                "was_earning": was_earning,                 # True=挣股数阶段开的短差
                "shares_delta": self.total_shares - shares_before,  # 挣股数阶段净增股
                "cost_basis_before": cb_before,
                "cost_basis_after": self.cost_basis,
            })


def run_version_i(
    i_signals: list[BarSignalI],
    *,
    floor_ladder: int = MIN_FLOOR_LADDER,
    maneuver_ratio: float = MANEUVER_RATIO,
    stop_mode: str = "none",
    diag: list | None = None,
    pairing: PairingConfig | None = None,
) -> tuple[list[CompletedTrade], dict]:
    """共享仓位多声部并发短差（用户 2026-06-09 裁定，否定 slice 模型）。

    存在论：入场一次满仓 100%（total_shares = INITIAL_CAPITAL/price，共享）；entry 层以下
    所有 ladder（≥ floor_ladder）各自独立维护短差生命周期，**全部作用于同一满仓**，
    每个短差闭合时 profit 直接写入共享 cost_basis（见 `_SharedFugue`）。`maneuver_ratio`
    在新模型下不再切 slice——保留参数仅为接口兼容（单笔短差量 = 满仓 × `level_frac`，
    entry 层以下子级别数均分 1/N_sub）。清仓总市值 = 满仓市值 + 降成本期间落袋现金，
    分母 INITIAL_CAPITAL。

    ═══════════════════════════════════════════════════════════════
    硬止损（用户指令，2% 阈值）—— `stop_mode` ∈ {"none","A","B"}
    ═══════════════════════════════════════════════════════════════
    `stop_mode="none"`（默认）：无止损。
    - **止损 A**：仓位 `c < entry_price×(1−2%)` → 全仓清出（cap 持仓下行/MDD）。
    - **止损 B**：A + 每级别开放短差逆向亏 2%（`c > sell_price×(1+2%)`）→ 回补该短差并
      冻结该级别降成本（`pos.stopped`），不影响主仓与其他级别（多声部独立止损）。
    认识论 L2（真实数据）。

    ═══════════════════════════════════════════════════════════════
    `pairing`（区间套反向应用的短差配对修复，P1-P5 消融）
    ═══════════════════════════════════════════════════════════════
    `pairing=None`（默认）：旧行为逐位不变（sell_any 开 / buy_any 平，盲配对）。
    `pairing=PairingConfig(...)`：_LONG 分支的短差层改为配对规则（消费
    `sig.bsp_events` 结构化事件流，进出场/ARM/主出场逻辑不变）：

    - 开腿：confirmed 卖点且 kind ∈ open_kinds（P4+ 另加：中枢存活 + 盘背位置
      c>ZG + 振幅 (c−ZG)/c ≥ θ）。锚 = (center_seg_start, ZG, kind)。
    - 闭腿优先级：(1) confirmed type1 买（P4+ 须同锚中枢）→ 正常回补；
      (2) candidate type3 买 → 预回补（P3+）；(3) confirmed type3 买 → 硬回补
      + 冻结该级别至新存活中枢出现（P2+）。type3 = 纤维死亡边界，不是低吸。
    - P5：每级别另开 O 模式震荡腿（中枢上沿 ZG×次级别卖点高抛 / 下沿 ZD 低吸，
      中枢死亡强制回补）。⚠ 盘整背驰的完整判定需次级别力度比较（引擎零改动
      约束下不可得），此处用"次级别卖点 × 价格≥ZG"作其必要条件代理（诚实声明）。

    配对要求事件流仅中枢承载层（ladder≥FIRST_BSP_LADDER）可得 → pairing 模式下
    floor_ladder < FIRST_BSP_LADDER 是定义错误（bar/bi 无 kind/中枢概念），直接抛错。
    """
    if pairing is not None and floor_ladder < FIRST_BSP_LADDER:
        raise ValueError(
            f"pairing 模式要求 floor_ladder ≥ {FIRST_BSP_LADDER}（中枢承载层）；"
            f"bar/bi 级无 kind/中枢概念，配对无定义。floor_ladder={floor_ladder}")
    if pairing is not None and not any(s.bsp_events for s in i_signals):
        raise ValueError(
            "pairing 模式要求事件磁带（BarSignalI.bsp_events 全空——该磁带由不产"
            "事件的信号层构造，如 m1_i_rust_engine.compute_i_signals_rust）")
    n = len(i_signals)
    state = _FLAT
    entry_bar = -1; entry_price = 0.0; entry_ladder = -1; arm_bar = -1
    arm_ladder = LADDER_MOVE
    pos: _SharedFugue | None = None      # 共享仓位（满仓 + 多声部并发短差）
    active_levels: list[int] = []        # entry 层以下的并发降成本级别
    SUB_EXPIRY = _ef.SUB_EXPIRY
    n_addon = 0
    n_core_stops = 0            # 仓位 2% 止损触发次数（A/B）
    n_voice_stops = 0          # 级别短差 2% 止损触发次数（B）
    _stop_on = stop_mode in ("A", "B")
    _voice_stop_on = stop_mode == "B"

    # ── 配对模式状态（市场级中枢生命周期账本，跨 trade 持续；pairing=None 时闲置）──
    last_center: dict[int, tuple] = {}   # ladder → (center_seg_start, zd, zg) 最新存活中枢
    dead_centers: dict[int, set] = {}    # ladder → {center_seg_start}（confirmed type3 = 纤维死亡）
    frozen: dict[int, int] = {}          # ladder → 死亡中枢键（硬回补后冻结至新存活中枢出现）
    n_close_normal = n_close_pre = n_close_hard = 0
    n_open_gate_rejects = 0              # P4+ 中枢门拒绝的开腿卖点数
    n_osc_open = n_osc_zd_close = 0      # P5 震荡腿开腿 / ZD 正常低吸数

    trades: list[CompletedTrade] = []
    ladder_attribution: dict[int, int] = {}
    ladder_held_bars: dict[int, int] = {}
    # 每级别累计贡献度（跨所有交易）：按 ladder 归因已闭合短差
    fsm_recovered: dict[int, float] = {}  # 该级别短差落袋 profit 累计（含负）
    fsm_diffs: dict[int, int] = {}        # 该级别完成短差次数
    fsm_short_pnl: dict[int, float] = {}  # 该级别价差收益累计（= profit 累计）

    def _open(bar_idx: int, price: float, el: int) -> None:
        nonlocal state, entry_bar, entry_price, entry_ladder, pos, active_levels
        entry_bar = bar_idx; entry_price = price; entry_ladder = el
        # 降成本级别 = entry 层以下所有 ladder（≥ floor_ladder），各级别并发作用于同一仓位。
        active_levels = list(range(floor_ladder, el))
        # 操作规模从走势结构来（简单方案，用户 2026-06-09 裁定）：按子级别数均分总仓位——
        # N_sub 个子级别每个操作 1/N_sub 满仓（Σ frac=100%）。这把单笔规模从旧
        # _level_trade_fraction 的 base=0.25/step=0.10 魔数（无原文数值依据的 L0 形式化引申）
        # 改为有明确语义的归一化分配。⚠ 均分**不防爆仓**（L2 实测）：降成本阶段是虚拟价差
        # 收集层（股数守恒、不减 total_shares），其盈亏独立叠加于满仓敞口且无下限；bar 级
        # PH settle 在波动中高抛低吸经常做反 → 累计净负击穿本金 → I_bar0 仍爆仓（印证缠师
        # "太小级别短差有害"，floor≥2 才正收益域）。爆仓根因在信号层（bar 级净负）+ 守恒律
        # 建模（虚拟层无下限），非仓位规模层——均分改不了，只让规模分配语义严格。
        n_sub = len(active_levels)
        level_frac = (1.0 / n_sub) if n_sub > 0 else 0.0
        # 满仓建仓（共享仓位，不切 slice）。diag 开启时注入 trace 列表（逐笔短差记录）。
        pos = _SharedFugue(
            entry_price=price, total_shares=INITIAL_CAPITAL / price, cost_basis=price,
            level_frac=level_frac, trace=([] if diag is not None else None))
        state = _LONG

    def _close(bar_idx: int, price: float, reason: str) -> None:
        nonlocal state, entry_bar, entry_price, entry_ladder, pos, active_levels
        if pos is None or entry_price <= 0 or pos.total_shares <= 0:
            state = _FLAT; pos = None; active_levels = []; return
        # 清仓前回补所有级别未闭短差（按各短差 open 阶段守恒律）
        for ladder in list(pos.active.keys()):
            pos.close_diff(ladder, price, bar_idx)
        # 清仓总市值 = 满仓市值 + 降成本期间落袋现金（挣股数阶段已并入 total_shares）
        total_value = pos.total_shares * price + pos.cumulative_recovered
        pnl_pct = (total_value - INITIAL_CAPITAL) / INITIAL_CAPITAL * 100
        held = bar_idx - entry_bar
        trades.append(CompletedTrade(
            entry_bar=entry_bar, entry_price=entry_price, exit_bar=bar_idx,
            exit_price=price, pnl_pct=round(pnl_pct, 4), exit_reason=reason,
            n_short_diffs=len(pos.completed),
            cost_basis_at_exit=round(pos.cost_basis, 6)))
        # ── 逐笔短差诊断：把本笔交易的完整短差 trace + 成本轨迹归档到 diag ──
        if diag is not None:
            diag.append({
                "entry_bar": entry_bar, "entry_price": entry_price,
                "exit_bar": bar_idx, "exit_price": price, "exit_reason": reason,
                "entry_ladder": entry_ladder, "entry_ladder_name": ladder_name(entry_ladder),
                "pnl_pct": round(pnl_pct, 4),
                "cost_basis_entry": entry_price,
                "cost_basis_exit": pos.cost_basis,
                "total_shares_entry": INITIAL_CAPITAL / entry_price,
                "total_shares_exit": pos.total_shares,
                "reached_earning": pos.earning,
                "n_diffs": len(pos.trace),
                "diffs": list(pos.trace),
            })
        # 分级贡献度归因（诊断）：按 ladder 聚合已闭合短差
        for ladder, cyc in pos.completed:
            fsm_recovered[ladder] = fsm_recovered.get(ladder, 0.0) + cyc.profit
            fsm_diffs[ladder] = fsm_diffs.get(ladder, 0) + 1
            fsm_short_pnl[ladder] = fsm_short_pnl.get(ladder, 0.0) + cyc.profit
        ladder_attribution[entry_ladder] = ladder_attribution.get(entry_ladder, 0) + 1
        ladder_held_bars[entry_ladder] = ladder_held_bars.get(entry_ladder, 0) + held
        state = _FLAT; entry_price = 0.0; entry_ladder = -1; pos = None; active_levels = []

    for i in range(n):
        sig = i_signals[i]
        c = sig.close

        # ── 配对模式：中枢生命周期账本（每 bar 维护，独立于持仓状态——中枢的
        # 生死是市场性质，不是仓位性质）。事件流级别隔离：每 ladder 独立维护
        # 自己的中枢上下文，不跨 ladder。──
        if pairing is not None:
            evrows = sig.bsp_events or NO_LADDER_EVENTS
            if evrows is not NO_LADDER_EVENTS:
                for lad in range(FIRST_BSP_LADDER, MAX_LADDER):
                    for ev in evrows[lad]:
                        kind, side, confirmed, cs = ev[0], ev[1], ev[3], ev[4]
                        if cs is None:
                            continue
                        dead = dead_centers.setdefault(lad, set())
                        if confirmed and kind == "type3":
                            dead.add(cs)             # 纤维死亡边界（买/卖侧均终结中枢）
                            if pairing.hard_type3 and side == "buy":
                                frozen[lad] = cs     # 向上离开确认 → 冻结该级别短差
                        elif cs not in dead:
                            last_center[lad] = (cs, ev[5], ev[6])
                            if lad in frozen and frozen[lad] != cs:
                                del frozen[lad]      # 新存活中枢出现 → 解冻

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
            # 硬止损 A/B：仓位跌破 entry×(1−2%) → 全仓清出（先于一切，cap 持仓回撤/MDD）
            if _stop_on and c < entry_price * (1.0 - STOP_FRAC):
                n_core_stops += 1
                _close(i, c, "stop_core_2pct")
            # 主出场：entry 层 confirmed type1 卖点（顶背驰）→ 全仓清出
            elif sig.sell1[entry_ladder]:
                _close(i, c, f"exit_{ladder_name(entry_ladder)}_type1sell")
            elif pairing is None:
                # 多重赋格降成本（基线，逐位不变）：各级别（entry 层以下）并发短差，
                # 全部作用于同一仓位 pos，触发 = 本级别 sell_any(高抛) / buy_any(低吸)。
                for ladder in active_levels:
                    if ladder in pos.stopped:
                        continue  # 止损 B 已冻结的级别（停止该级别降成本，不影响主仓/他级别）
                    rec = pos.active.get(ladder)
                    has_open = rec is not None
                    # 止损 B：开放短差逆向亏 2%（卖出后价格反向上涨 > sell×(1+2%)）→
                    # 回补该短差 + 冻结该级别降成本（每级别独立止损）。
                    if (_voice_stop_on and has_open
                            and c > rec[0].sell_price * (1.0 + STOP_FRAC)):
                        pos.close_diff(ladder, c, i)
                        pos.stopped.add(ladder)
                        n_voice_stops += 1
                        continue
                    if sig.sell_any[ladder] and not has_open:
                        pos.open_diff(ladder, pos.level_frac, c, i)
                    elif sig.buy_any[ladder] and has_open:
                        pos.close_diff(ladder, c, i)
                if sig.type2_buy:
                    n_addon += 1
            else:
                # ── 区间套反向应用的短差配对（P1-P7）──
                # 底空间 = 本级别中枢 [ZD,ZG]×lifetime；两腿须在同一纤维（中枢存活）；
                # type3 = 纤维死亡边界 → 回补信号，不是低吸。
                # 级别隔离（用户 2026-06-10 裁定）：每个 ladder 只消费自己的事件流
                # （evrows[ladder]）+ 自己的中枢上下文（last_center/dead/frozen[ladder]）
                # + 次级别（ladder−1）定位信号；配对在各 ladder 内独立完成，不跨 ladder。
                evrows = sig.bsp_events or NO_LADDER_EVENTS
                for ladder in active_levels:
                    evs = evrows[ladder]
                    rec = pos.active.get(ladder)
                    if rec is not None and evs:
                        anchor = rec[2]  # (center_seg_start, zg, sell_kind) | None
                        # 闭腿优先级（事件索引：0=kind 1=side 3=confirmed 4=center）
                        normal = any(
                            e[3] and e[0] == "type1" and e[1] == "buy"
                            and (not pairing.same_center_close or anchor is None
                                 or e[4] == anchor[0])
                            for e in evs)
                        pre = pairing.pre_type3 and any(
                            (not e[3]) and e[0] == "type3" and e[1] == "buy"
                            for e in evs)
                        hard = pairing.hard_type3 and any(
                            e[3] and e[0] == "type3" and e[1] == "buy" for e in evs)
                        if normal:
                            pos.close_diff(ladder, c, i); n_close_normal += 1
                        elif pre:
                            pos.close_diff(ladder, c, i); n_close_pre += 1
                        elif hard:
                            pos.close_diff(ladder, c, i); n_close_hard += 1
                    # 开腿（平仓 bar 可重开：配对模式下开/平由不同事件驱动，
                    # 同 bar 先平后开是事件序合法的）
                    if (pairing.open_kinds and evs
                            and pos.active.get(ladder) is None
                            and ladder not in frozen):
                        for e in evs:
                            if not (e[3] and e[1] == "sell"
                                    and e[0] in pairing.open_kinds):
                                continue
                            if pairing.center_gate:
                                cs, zg = e[4], e[6]
                                if (cs is None
                                        or cs in dead_centers.get(ladder, ())
                                        or c <= zg
                                        or (c - zg) / c < pairing.theta_amp):
                                    n_open_gate_rejects += 1
                                    continue
                            pos.open_diff(ladder, pos.level_frac, c, i,
                                          anchor=(e[4], e[6], e[0]))
                            break
                    # ── P5+ 域腿（中枢震荡腿，独立槽）：每级别短差只在自己的结构域
                    # （锚中枢 [ZD,ZG] × lifetime）内操作，操作点用次级别走势定位。──
                    if pairing.osc_mode:
                        okey = ladder + OSC_KEY_OFFSET
                        orec = pos.active.get(okey)
                        lc = last_center.get(ladder)
                        center_alive = (lc is not None and lc[0]
                                        not in dead_centers.get(ladder, ()))
                        if orec is not None:
                            # osc 锚 = (center_seg_start, 锚中枢 ZD, "osc")——回补边界
                            # 必须用开腿时锚定中枢的 ZD（两腿同纤维），不是最新中枢的 ZD。
                            # 同中枢强制 = 几何（价格 ≤ 锚 ZD）×生命周期（锚未死亡）：
                            # 次级别定位信号（PH proxy/buy_any）不携带 center 引用，
                            # 几何+lifetime 是"同一纤维"的严格可实现形式。
                            o_anchor = orec[2]
                            o_dead = (o_anchor is not None and o_anchor[0]
                                      in dead_centers.get(ladder, ()))
                            if o_dead:
                                pos.close_diff(okey, c, i)  # 中枢死亡 → 强制回补
                            elif (o_anchor is not None and c <= o_anchor[1]
                                  and (not pairing.osc_buy_sub
                                       or sig.buy_any[ladder - 1])):
                                # 低吸：≤锚 ZD；P6+ 另要求次级别买点（次级别向下
                                # 走势完成点定位，与开腿的次级别卖点定位同构）
                                pos.close_diff(okey, c, i)
                                n_osc_zd_close += 1
                        elif (center_alive and ladder not in frozen
                              and ladder >= 1 and c >= lc[2]
                              and sig.sell_any[ladder - 1]):
                            # 上沿 ZG × 次级别卖点（盘整背驰必要条件代理）→ 高抛
                            pos.open_diff(okey, pos.level_frac, c, i,
                                          anchor=(lc[0], lc[1], "osc"))
                            n_osc_open += 1
                if sig.type2_buy:
                    n_addon += 1

    if state == _LONG and pos is not None and pos.total_shares > 0:
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
    extra = {
        "ladder_attribution": {ladder_name(k): v for k, v in ladder_attribution.items()},
        "ladder_avg_held_bars": {ladder_name(k): v for k, v in avg_held.items()},
        "fsm_contribution": fsm_contrib,
        "addon_2buy_marks": n_addon,
        "maneuver_ratio": maneuver_ratio,
        "min_floor_ladder": floor_ladder,
        "stop_mode": stop_mode,
        "n_core_stops": n_core_stops,
        "n_voice_stops": n_voice_stops,
    }
    if pairing is not None:
        extra["pairing"] = {
            "open_kinds": list(pairing.open_kinds),
            "hard_type3": pairing.hard_type3,
            "pre_type3": pairing.pre_type3,
            "center_gate": pairing.center_gate,
            "theta_amp": pairing.theta_amp,
            "same_center_close": pairing.same_center_close,
            "osc_mode": pairing.osc_mode,
            "osc_buy_sub": pairing.osc_buy_sub,
            "n_close_normal": n_close_normal,
            "n_close_pre_type3": n_close_pre,
            "n_close_hard_type3": n_close_hard,
            "n_open_gate_rejects": n_open_gate_rejects,
            "n_osc_open": n_osc_open,
            "n_osc_zd_close": n_osc_zd_close,
        }
    return trades, extra


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
        "> 共享仓位多声部（满仓 + entry 层以下各级别并发短差作用于同一 cost_basis）/ "
        "每中枢层真实 confirmed type1/2/3 BSP"
        "（经 `per_level_bsp` 适配层：走势级 `confirmed_bsp_level1`、递归层 "
        "`confirmed_bsp_for_level`，**非 move-settle 近似**）/ 含 bar 级（可配置下限）/ "
        "完整跑高层 / 区分机动仓占比 vs 级别驱动单笔量 / 删除伪 267 课号引用。\n")
    L.append(
        f"> 配置：`MANEUVER_RATIO={MANEUVER_RATIO}`（原文'例如 1/10'机动仓占比）；单笔短差量"
        "= 满仓 × `level_frac`（entry 层以下子级别数均分 1/N_sub）。认识论 **L2**；力度口径=价格"
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
