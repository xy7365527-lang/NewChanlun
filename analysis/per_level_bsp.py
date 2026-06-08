"""Per-level confirmed BSP 适配层（递归层 ≥2 真实买卖点）。

替换 `fugue_version_i.py` 中"move settle + 背驰"的近似买卖点判定。
引擎已为 level=1 产出 confirmed BSP（`snap.bsp_snapshot`），但递归层 ≥2 不直接
产每层 BSP——引擎只产中枢/走势 settle。本模块用引擎的纯函数入口
`buysellpoints_from_level` + `divergences_from_moves_v1` 组合出递归层的
confirmed BSP，所需的输入类型差异由两个适配器消除：

  1. `_LevelZhongshuView`：LevelZhongshu → 暴露 v1 Zhongshu 接口
     （seg_start=comp_start, seg_end=comp_end, break_seg=break_comp）。
  2. `_MoveAsSegmentView`：前一级别 Move → 暴露 Segment 接口
     （i0=first_seg_s0, i1=last_seg_s1, high/low/direction 直接透传）。

索引一致性依据（well-formedness 证明）
---------------------------------------------
引擎递归链中：
- `adapt_moves(prev_level_moves, lvl)` 给每个组件 component_idx = 列表位置 i。
- `zhongshu_from_components` 把 comp_start/comp_end/break_comp 记为 component_idx，
  即 **前一级别 moves 全列表位置**。
- `moves_from_level_zhongshus` 把 level-N Move.seg_start/seg_end 记为同一组件位置，
  zs_start/zs_end 记为 LevelZhongshu 全列表位置。

因此 `buysellpoints_from_level`/`divergences_from_moves_v1` 内部所有按索引访问
（`segments[move.seg_start]`、`zhongshus[div.center_idx]`、`zhongshus[move.zs_start]`）
当且仅当传入：
- `segments` = 前一级别 moves 全列表（顺序不变，逐元素适配为 Segment 接口）；
- `zhongshus` = 本级别 LevelZhongshu 全列表（顺序不变，逐元素适配为 Zhongshu 接口）；
- `moves` = 本级别 Move 全列表
时索引自洽。本模块严格按此契约组装，不重排不过滤。

认识论等级与有效域边界
----------------------
- df_macd=None：力度退化为 `振幅 × 组件跨度`（fallback，见 a_divergence_v1._compute_force）。
  递归层的 i0/i1 = 前级别组件索引（非 raw bar），故力度的"持续长度"是组件计数，
  不是 bar 计数——这是 521号边界（高层力度为 PH 纯拓扑代理，不能算 MACD），
  不是矛盾。本模块不伪造 raw-bar MACD，诚实保留拓扑代理力度。
- 本模块产出 confirmed BSP 的 confirmed 字段语义：Type1 面积比 ≤ TYPE1_CONFIRM_RATIO、
  Type2 不创新极值、Type3 回试后延续段——均由 buysellpoints_from_level 内部决定，
  本模块不改判据。

谱系：521号（candidate+MACD闸 ≡ BSP type1 confirmed 同构）/ 525号（组件来源无关）/
project_divergence_locator_entry_exit。
"""

from __future__ import annotations

from dataclasses import dataclass

from newchan.a_buysellpoint_v1 import BuySellPoint, buysellpoints_from_level
from newchan.a_divergence import Divergence
from newchan.a_divergence_v1 import divergences_from_moves_v1
from newchan.a_move_v1 import Move, moves_from_zhongshus
from newchan.a_stroke import Stroke
from newchan.a_zhongshu_level import LevelZhongshu
from newchan.a_zhongshu_v1 import zhongshu_from_strokes

__all__ = [
    "level_bsp_inputs",
    "confirmed_bsp_for_level",
    "confirmed_bsp_level1",
    "confirmed_bsp_bi_zhongshu",
    "BI_ZHONGSHU_LEVEL_ID",
]

# 笔中枢级 BSP 的 level_id 构造标签（525号点3 / a_buysellpoint_v1 docstring 点3：
# level_id 是构造标签，非有效性声明）。笔中枢是最低中枢层（线段中枢/走势级之下），
# 与 candidate_settle_backtest_qqq.py 的 LEVEL_ID=1（"笔级别构造标签"）一致。
# 不与递归引擎 level=1（线段中枢/走势级）冲突：level_id 不进 BSP 身份键去重，
# 笔中枢 BSP 与走势级 BSP 由各自独立的 tracker 持有。
BI_ZHONGSHU_LEVEL_ID = 1


# ════════════════════════════════════════════════════════════
# 适配器视图（frozen，不修改引擎数据类）
# ════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class _MoveAsSegmentView:
    """前一级别 Move → v1 Segment 接口（buysellpoints/divergences 的 segments 元素）。

    引擎按索引消费 segments[idx].{i0,i1,high,low,direction}。
    Move 暴露 first_seg_s0/last_seg_s1（前端定位用前级别组件索引），
    映射 i0=first_seg_s0, i1=last_seg_s1（任务卡逐字）。
    high/low/direction 直接透传 Move 同名字段。
    """

    _move: Move

    @property
    def i0(self) -> int:
        return self._move.first_seg_s0

    @property
    def i1(self) -> int:
        return self._move.last_seg_s1

    @property
    def high(self) -> float:
        return self._move.high

    @property
    def low(self) -> float:
        return self._move.low

    @property
    def direction(self) -> str:
        return self._move.direction


@dataclass(frozen=True, slots=True)
class _LevelZhongshuView:
    """LevelZhongshu → v1 Zhongshu 接口（buysellpoints/divergences 的 zhongshus 元素）。

    引擎消费 zhongshus[idx].{seg_start,seg_end,break_seg,zd,zg,settled,break_direction}。
    LevelZhongshu 对应字段为 comp_start/comp_end/break_comp（任务卡逐字：
    seg_start=comp_start, break_seg=break_comp；seg_end=comp_end 同构补全）。
    zd/zg/settled/break_direction 直接透传同名字段。
    """

    _lzs: LevelZhongshu

    @property
    def seg_start(self) -> int:
        return self._lzs.comp_start

    @property
    def seg_end(self) -> int:
        return self._lzs.comp_end

    @property
    def break_seg(self) -> int:
        return self._lzs.break_comp

    @property
    def zd(self) -> float:
        return self._lzs.zd

    @property
    def zg(self) -> float:
        return self._lzs.zg

    @property
    def settled(self) -> bool:
        return self._lzs.settled

    @property
    def break_direction(self) -> str:
        return self._lzs.break_direction


# ════════════════════════════════════════════════════════════
# 输入组装（纯函数，不重排不过滤——保证索引自洽）
# ════════════════════════════════════════════════════════════


def level_bsp_inputs(
    prev_level_moves: list[Move],
    level_zhongshus: list[LevelZhongshu],
) -> tuple[list[_MoveAsSegmentView], list[_LevelZhongshuView]]:
    """把递归层 ≥2 的 (前级别 moves, 本级别 LevelZhongshu) 适配成引擎可消费的
    (segments, zhongshus)。

    严格逐元素适配，**保持原列表顺序与长度**——这是索引自洽的前提
    （见模块 docstring well-formedness 证明）。

    Parameters
    ----------
    prev_level_moves : list[Move]
        前一级别（level N-1）的全部 moves（含末尾 unsettled）。作为本级别
        中枢/走势的组件序列，引擎以 component_idx = 列表位置消费。
    level_zhongshus : list[LevelZhongshu]
        本级别（level N）的全部 LevelZhongshu（含末尾 unsettled）。

    Returns
    -------
    (segments_view, zhongshus_view)
        分别可作为 buysellpoints_from_level / divergences_from_moves_v1 的
        segments / zhongshus 参数。
    """
    if prev_level_moves is None or level_zhongshus is None:
        raise ValueError("prev_level_moves 和 level_zhongshus 不能为 None")
    segments_view = [_MoveAsSegmentView(_move=m) for m in prev_level_moves]
    zhongshus_view = [_LevelZhongshuView(_lzs=z) for z in level_zhongshus]
    return segments_view, zhongshus_view


def confirmed_bsp_for_level(
    prev_level_moves: list[Move],
    level_zhongshus: list[LevelZhongshu],
    level_moves: list[Move],
    level_id: int,
) -> list[BuySellPoint]:
    """产出递归层 N≥2 的 confirmed 买卖点（type1/2/3，买/卖）。

    组合引擎纯函数：先 divergences_from_moves_v1（df_macd=None → 拓扑代理力度），
    再 buysellpoints_from_level。

    Parameters
    ----------
    prev_level_moves : list[Move]
        前一级别（N-1）全部 moves（组件序列）。
    level_zhongshus : list[LevelZhongshu]
        本级别（N）全部 LevelZhongshu。
    level_moves : list[Move]
        本级别（N）全部 Move。
    level_id : int
        递归级别 N（≥2）。

    Returns
    -------
    list[BuySellPoint]
        本级别的 confirmed/candidate 买卖点（按 seg_idx 排序）。
        调用方按需筛 `bp.confirmed and bp.kind == "type1"` 等。

    Raises
    ------
    ValueError
        level_id < 2（level=1 请直接用 snap.bsp_snapshot，见 confirmed_bsp_level1）。
    """
    if level_id < 2:
        raise ValueError(
            f"confirmed_bsp_for_level 仅用于递归层 ≥2，level_id={level_id}；"
            "level=1 请用 confirmed_bsp_level1（引擎已产 bsp_snapshot）")

    segments_view, zhongshus_view = level_bsp_inputs(
        prev_level_moves, level_zhongshus)

    divergences: list[Divergence] = divergences_from_moves_v1(
        segments_view,
        zhongshus_view,
        level_moves,
        level_id,
        df_macd=None,
        merged_to_raw=None,
    )

    return buysellpoints_from_level(
        segments_view,
        zhongshus_view,
        level_moves,
        divergences,
        level_id,
    )


def confirmed_bsp_level1(bsp_snapshot) -> list[BuySellPoint]:
    """走势级（level=1）直接透传引擎已产的 confirmed BSP。

    引擎在 level=1 已运行完整 BuySellPointEngine，confirmed BSP 在
    `snap.bsp_snapshot.buysellpoints` 中——不需要本模块重算。

    Parameters
    ----------
    bsp_snapshot : BuySellPointSnapshot
        snap.bsp_snapshot。

    Returns
    -------
    list[BuySellPoint]
        snap.bsp_snapshot.buysellpoints 的列表副本（不可变透传）。
    """
    if bsp_snapshot is None:
        raise ValueError("bsp_snapshot 不能为 None")
    return list(bsp_snapshot.buysellpoints)


# ════════════════════════════════════════════════════════════
# 笔中枢级 confirmed BSP（segment 级 = 笔级别走势，中枢承载层下放）
# ════════════════════════════════════════════════════════════
#
# 525号笔中枢退化基底路径：三笔重叠 → 笔中枢 → 笔级别走势(盘整/趋势) → 笔级别背驰
# → type1/2/3 买卖点。这给 segment 级（ladder 2，原仅 PH proxy）一个真实中枢承载层，
# 使其能产生 confirmed type1/2/3 买卖点，从而作为 entry_level / 降成本级别。
#
# 组件来源无关性（525号点1 / a_buysellpoint_v1 docstring 点1）：
#   buysellpoints_from_level / divergences_from_moves_v1 只要求 segments[idx] 满足最小
#   区间接口 {direction, high, low, i0, i1}。**Stroke 原生满足全部五字段**（a_stroke.Stroke），
#   故同一组引擎纯函数不加修改地服务笔中枢路径——无需 _MoveAsSegmentView 适配器
#   （线段中枢路径需要它是因为 Move 没有 i0/i1，笔本身就是端点级单位）。
#
# 索引自洽（well-formedness）：
#   zhongshu_from_strokes 内部 `confirmed = [s for s in strokes if s.confirmed]` 重新编号，
#   Zhongshu.{seg_start,seg_end,break_seg} 索引进 confirmed 列表位置。本函数先把 strokes
#   过滤为 confirmed 列表，再把**同一个 confirmed 列表**作为 segments 传给下游纯函数——
#   中枢编号依据的列表 ≡ BSP 索引的 segments，索引绝对自洽（不依赖"仅末笔未确认"不变量）。


def confirmed_bsp_bi_zhongshu(
    strokes: list[Stroke],
    level_id: int = BI_ZHONGSHU_LEVEL_ID,
) -> list[BuySellPoint]:
    """从笔序列产出笔中枢级 confirmed/candidate 买卖点（type1/2/3，买/卖）。

    管线（525号笔中枢路径，与 candidate_settle_backtest_qqq.py:142-146 同构）：

        confirmed 笔 → zhongshu_from_strokes(笔中枢)
                     → moves_from_zhongshus(笔级别走势：盘整/趋势分类)
                     → divergences_from_moves_v1(df_macd=None → 振幅力度，笔级别背驰)
                     → buysellpoints_from_level(type1/2/3)

    力度口径（521号边界）：df_macd=None → 力度 = 价格振幅 × 笔跨度（i1−i0），
    非 MACD。诚实保留拓扑/振幅代理力度，不伪造 raw-bar MACD。

    Parameters
    ----------
    strokes : list[Stroke]
        笔序列（可含末尾 unconfirmed 笔；本函数内部过滤 confirmed）。
        Stroke 原生暴露 {direction, high, low, i0, i1}，直接作为 segments。
    level_id : int
        笔中枢级构造标签（默认 BI_ZHONGSHU_LEVEL_ID=1）。非有效性声明（525号点3）。

    Returns
    -------
    list[BuySellPoint]
        按 seg_idx 排序的笔中枢级买卖点。调用方按 `bp.confirmed and bp.kind == "type1"`
        等筛选。笔不足 3 根 confirmed 时返回空列表（zhongshu_from_strokes 短路）。
    """
    if strokes is None:
        raise ValueError("strokes 不能为 None")

    confirmed = [s for s in strokes if s.confirmed]
    if len(confirmed) < 3:
        return []

    zhongshus = zhongshu_from_strokes(confirmed)
    moves = moves_from_zhongshus(zhongshus, num_segments=len(confirmed))
    divergences: list[Divergence] = divergences_from_moves_v1(
        confirmed, zhongshus, moves, level_id, df_macd=None, merged_to_raw=None,
    )
    return buysellpoints_from_level(confirmed, zhongshus, moves, divergences, level_id)
