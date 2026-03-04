"""
折叠内在性检验 第三轮 — Agent B: 缠论结构分析
黄金日线 (GC=F) 2000-01-01 ~ 2026-03-01

严格按缠论知识库定义执行：
1. K线包含处理
2. 顶底分型识别
3. 笔构建（新笔定义：顶底分型间至少3根K线）
4. 线段构建（三笔重叠法）
5. 中枢标记
6. 走势结构标记（趋势/盘整）
"""

import json
import datetime
from dataclasses import dataclass, field, asdict
from typing import List, Optional, Tuple

import yfinance as yf
import pandas as pd


# ============================================================
# 数据类
# ============================================================

@dataclass
class Bar:
    """经过包含处理后的K线"""
    date: str          # YYYY-MM-DD
    high: float
    low: float
    raw_count: int = 1  # 合并了几根原始K线


@dataclass
class Fractal:
    """分型"""
    kind: str          # "top" or "bottom"
    date: str          # 分型所在日期（中间K线日期）
    price: float       # 顶分型取high，底分型取low
    bar_index: int     # 在merged_bars中的索引


@dataclass
class Stroke:
    """笔"""
    direction: str     # "up" or "down"
    start_date: str
    end_date: str
    start_price: float
    end_price: float
    start_fractal_idx: int
    end_fractal_idx: int
    bar_count: int = 0


@dataclass
class Segment:
    """线段"""
    direction: str     # "up" or "down"
    start_date: str
    end_date: str
    start_price: float
    end_price: float
    stroke_count: int = 0


@dataclass
class Zhongshu:
    """中枢"""
    id: int
    start_date: str
    end_date: str
    zg: float          # 中枢区间高点 min(g1, g2)
    zd: float          # 中枢区间低点 max(d1, d2)
    duration_days: int = 0
    seg_indices: List[int] = field(default_factory=list)


@dataclass
class Movement:
    """走势"""
    id: int
    kind: str          # "uptrend" / "downtrend" / "consolidation"
    start_date: str
    end_date: str
    duration_days: int = 0
    zhongshu_count: int = 0


# ============================================================
# Step 1: 获取数据
# ============================================================

def fetch_gold_data() -> pd.DataFrame:
    """获取黄金日线数据"""
    print("正在下载黄金日线数据 (GC=F)...")
    df = yf.download("GC=F", start="2000-01-01", end="2026-03-01", progress=False)
    # 处理可能的MultiIndex列
    if isinstance(df.columns, pd.MultiIndex):
        df.columns = df.columns.get_level_values(0)
    df = df.dropna(subset=["High", "Low"])
    print(f"获取到 {len(df)} 根K线，日期范围: {df.index[0].strftime('%Y-%m-%d')} ~ {df.index[-1].strftime('%Y-%m-%d')}")
    return df


# ============================================================
# Step 2: K线包含处理
# ============================================================

def has_inclusion(b1: Bar, b2: Bar) -> bool:
    """判断两根K线是否存在包含关系"""
    return (b1.high >= b2.high and b1.low <= b2.low) or \
           (b2.high >= b1.high and b2.low <= b1.low)


def merge_bars(b1: Bar, b2: Bar, direction_up: bool) -> Bar:
    """合并两根有包含关系的K线"""
    if direction_up:
        return Bar(
            date=b2.date,
            high=max(b1.high, b2.high),
            low=max(b1.low, b2.low),
            raw_count=b1.raw_count + b2.raw_count
        )
    else:
        return Bar(
            date=b2.date,
            high=min(b1.high, b2.high),
            low=min(b1.low, b2.low),
            raw_count=b1.raw_count + b2.raw_count
        )


def process_inclusion(df: pd.DataFrame) -> List[Bar]:
    """对原始K线做包含处理，返回合并后的K线序列"""
    bars: List[Bar] = []
    for i in range(len(df)):
        row = df.iloc[i]
        new_bar = Bar(
            date=df.index[i].strftime("%Y-%m-%d"),
            high=float(row["High"]),
            low=float(row["Low"])
        )

        if len(bars) < 2:
            bars.append(new_bar)
            continue

        # 检查当前bar与最后一根bar是否有包含关系
        last = bars[-1]
        if has_inclusion(last, new_bar):
            # 确定合并方向：看last与前一根bar的关系
            prev = bars[-2]
            direction_up = last.high > prev.high  # 简化判断
            merged = merge_bars(last, new_bar, direction_up)
            bars[-1] = merged
        else:
            bars.append(new_bar)

    print(f"包含处理: {len(df)} 根原始K线 → {len(bars)} 根合并K线")
    return bars


# ============================================================
# Step 3: 分型识别
# ============================================================

def find_fractals(bars: List[Bar]) -> List[Fractal]:
    """在合并后的K线序列上识别顶底分型"""
    fractals: List[Fractal] = []

    for i in range(1, len(bars) - 1):
        prev_bar = bars[i - 1]
        curr_bar = bars[i]
        next_bar = bars[i + 1]

        # 顶分型：中间K线高点最高且低点最高
        if curr_bar.high > prev_bar.high and curr_bar.high > next_bar.high and \
           curr_bar.low > prev_bar.low and curr_bar.low > next_bar.low:
            fractals.append(Fractal(
                kind="top",
                date=curr_bar.date,
                price=curr_bar.high,
                bar_index=i
            ))

        # 底分型：中间K线低点最低且高点最低
        elif curr_bar.low < prev_bar.low and curr_bar.low < next_bar.low and \
             curr_bar.high < prev_bar.high and curr_bar.high < next_bar.high:
            fractals.append(Fractal(
                kind="bottom",
                date=curr_bar.date,
                price=curr_bar.low,
                bar_index=i
            ))

    print(f"分型识别: 找到 {len(fractals)} 个分型 (顶: {sum(1 for f in fractals if f.kind == 'top')}, 底: {sum(1 for f in fractals if f.kind == 'bottom')})")
    return fractals


# ============================================================
# Step 4: 笔构建
# ============================================================

def build_strokes(fractals: List[Fractal], bars: List[Bar]) -> List[Stroke]:
    """
    从分型序列构建笔——标准贪心算法。

    新笔定义：顶分型与底分型的bar_index差 >= 4（保证中间至少3根独立K线）。

    算法：
    1. 从第一个分型开始，作为当前锚点
    2. 向后扫描，寻找第一个满足条件的异类分型形成笔
    3. 如果遇到同类分型且更极端，更新锚点
    4. 形成笔后，笔的终点成为下一笔的起点锚点
    """
    if not fractals:
        return []

    MIN_BAR_GAP = 4  # 新笔条件：bar_index差 >= 4

    strokes: List[Stroke] = []
    anchor = fractals[0]  # 当前锚点分型
    anchor_idx = 0

    i = 1
    while i < len(fractals):
        curr = fractals[i]

        if curr.kind == anchor.kind:
            # 同类分型：更新锚点为更极端的
            if curr.kind == "top" and curr.price >= anchor.price:
                anchor = curr
                anchor_idx = i
            elif curr.kind == "bottom" and curr.price <= anchor.price:
                anchor = curr
                anchor_idx = i
            i += 1
            continue

        # 异类分型：检查笔成立条件
        bar_gap = abs(curr.bar_index - anchor.bar_index)

        if bar_gap < MIN_BAR_GAP:
            i += 1
            continue

        # 方向和价格约束
        if anchor.kind == "bottom" and curr.kind == "top":
            if curr.price <= anchor.price:
                i += 1
                continue
            direction = "up"
        else:
            if curr.price >= anchor.price:
                i += 1
                continue
            direction = "down"

        # 在 anchor 和 curr 之间，检查是否有更极端的同类分型
        # 比如锚点是bottom，我们找top，但中间可能有更低的bottom
        better_anchor = anchor
        better_anchor_idx = anchor_idx
        found_better = False
        for k in range(anchor_idx + 1, i):
            mid = fractals[k]
            if mid.kind == anchor.kind:
                if anchor.kind == "bottom" and mid.price < better_anchor.price:
                    better_anchor = mid
                    better_anchor_idx = k
                    found_better = True
                elif anchor.kind == "top" and mid.price > better_anchor.price:
                    better_anchor = mid
                    better_anchor_idx = k
                    found_better = True

        if found_better:
            # 重新从更好的锚点开始
            anchor = better_anchor
            anchor_idx = better_anchor_idx
            # 不推进i，重新检查当前curr与新锚点
            bar_gap = abs(curr.bar_index - anchor.bar_index)
            if bar_gap < MIN_BAR_GAP:
                i += 1
                continue
            if anchor.kind == "bottom" and curr.price <= anchor.price:
                i += 1
                continue
            if anchor.kind == "top" and curr.price >= anchor.price:
                i += 1
                continue

        strokes.append(Stroke(
            direction=direction,
            start_date=anchor.date,
            end_date=curr.date,
            start_price=anchor.price,
            end_price=curr.price,
            start_fractal_idx=anchor_idx,
            end_fractal_idx=i,
            bar_count=bar_gap
        ))

        # 笔的终点成为下一笔的锚点
        anchor = curr
        anchor_idx = i
        i += 1

    print(f"笔构建: {len(strokes)} 笔 (上升: {sum(1 for s in strokes if s.direction == 'up')}, 下降: {sum(1 for s in strokes if s.direction == 'down')})")
    return strokes


# ============================================================
# Step 5: 线段构建（三笔重叠法 — 口径A）
# ============================================================

def _stroke_range(s: Stroke) -> Tuple[float, float]:
    """笔的价格区间 [low, high]"""
    return (min(s.start_price, s.end_price), max(s.start_price, s.end_price))


def _overlap(r1: Tuple[float, float], r2: Tuple[float, float]) -> Optional[Tuple[float, float]]:
    """两个区间的重叠部分，无重叠返回None"""
    lo = max(r1[0], r2[0])
    hi = min(r1[1], r2[1])
    if lo < hi:
        return (lo, hi)
    return None


def _three_stroke_overlap(s1: Stroke, s2: Stroke, s3: Stroke) -> bool:
    """前三笔是否有重叠区域"""
    r1 = _stroke_range(s1)
    r2 = _stroke_range(s2)
    r3 = _stroke_range(s3)
    o12 = _overlap(r1, r2)
    if o12 is None:
        return False
    o123 = _overlap(o12, r3)
    return o123 is not None


def build_segments(strokes: List[Stroke]) -> List[Segment]:
    """
    口径A线段构建：至少三笔，前三笔有重叠，贪心推进。

    改进算法：
    1. 从每个可能的起始位置扫描，找到连续3笔有重叠的位置
    2. 从该位置开始贪心延伸——每次向后看2笔(保持奇数)，
       检查最后三笔是否仍有重叠
    3. 线段方向由第一笔决定
    4. 下一条线段从当前线段最后一笔的终点开始寻找
    """
    if len(strokes) < 3:
        return []

    segments: List[Segment] = []

    i = 0
    while i <= len(strokes) - 3:
        s1, s2, s3 = strokes[i], strokes[i + 1], strokes[i + 2]

        if _three_stroke_overlap(s1, s2, s3):
            direction = s1.direction
            seg_start = i
            seg_end = i + 2

            # 贪心延伸：每次+1笔，检查最新的连续三笔是否有重叠
            j = i + 3
            while j < len(strokes):
                # 检查最后三笔 (j-2, j-1, j) 是否有重叠
                if _three_stroke_overlap(strokes[j - 2], strokes[j - 1], strokes[j]):
                    seg_end = j
                    j += 1
                else:
                    break

            # 确保线段笔数为奇数（线段从顶到底或底到顶）
            stroke_count = seg_end - seg_start + 1
            if stroke_count % 2 == 0:
                seg_end -= 1
                stroke_count -= 1

            if stroke_count >= 3:
                seg_strokes = strokes[seg_start:seg_end + 1]

                # 线段起止价格：起点=第一笔起价，终点=最后一笔终价
                sp = seg_strokes[0].start_price
                ep = seg_strokes[-1].end_price

                segments.append(Segment(
                    direction=direction,
                    start_date=strokes[seg_start].start_date,
                    end_date=strokes[seg_end].end_date,
                    start_price=sp,
                    end_price=ep,
                    stroke_count=stroke_count
                ))

            # 下一条线段从当前线段结束后的下一笔开始
            # 但要考虑重叠：新线段的第一笔可以是当前线段最后一笔
            i = seg_end
        else:
            i += 1

    print(f"线段构建: {len(segments)} 条线段 (上升: {sum(1 for s in segments if s.direction == 'up')}, 下降: {sum(1 for s in segments if s.direction == 'down')})")
    return segments


# ============================================================
# Step 6: 中枢标记
# ============================================================

def build_zhongshus(segments: List[Segment]) -> List[Zhongshu]:
    """
    从线段序列构建中枢。
    中枢定义：至少三段有重叠区域，取公共重叠区间。
    使用贪心法：连续扫描线段，维护当前重叠区间。
    """
    if len(segments) < 3:
        return []

    zhongshus: List[Zhongshu] = []
    zs_id = 1

    i = 0
    while i <= len(segments) - 3:
        # 取三段的价格区间
        ranges = []
        for k in range(3):
            seg = segments[i + k]
            lo = min(seg.start_price, seg.end_price)
            hi = max(seg.start_price, seg.end_price)
            ranges.append((lo, hi))

        # 计算三段公共重叠
        overlap_lo = max(r[0] for r in ranges)
        overlap_hi = min(r[1] for r in ranges)

        if overlap_lo < overlap_hi:
            # 有公共重叠，形成中枢
            zd = overlap_lo
            zg = overlap_hi
            zs_start = i
            zs_end = i + 2

            # 贪心延伸：后续线段如果与 [zd, zg] 有重叠，就纳入中枢
            j = i + 3
            while j < len(segments):
                seg = segments[j]
                seg_lo = min(seg.start_price, seg.end_price)
                seg_hi = max(seg.start_price, seg.end_price)
                if seg_lo < zg and seg_hi > zd:
                    # 有重叠，延伸中枢
                    zs_end = j
                    j += 1
                else:
                    # 离开中枢
                    break

            start_date = segments[zs_start].start_date
            end_date = segments[zs_end].end_date

            # 计算持续天数
            try:
                d1 = datetime.datetime.strptime(start_date, "%Y-%m-%d")
                d2 = datetime.datetime.strptime(end_date, "%Y-%m-%d")
                duration = (d2 - d1).days
            except ValueError:
                duration = 0

            zhongshus.append(Zhongshu(
                id=zs_id,
                start_date=start_date,
                end_date=end_date,
                zg=round(zg, 2),
                zd=round(zd, 2),
                duration_days=duration,
                seg_indices=list(range(zs_start, zs_end + 1))
            ))
            zs_id += 1
            i = zs_end + 1
        else:
            i += 1

    print(f"中枢标记: {len(zhongshus)} 个中枢")
    return zhongshus


# ============================================================
# Step 7: 走势结构标记
# ============================================================

def build_movements(zhongshus: List[Zhongshu], segments: List[Segment]) -> List[Movement]:
    """
    标记走势类型：
    - 趋势：同方向至少两个中枢（上升=中枢依次抬高，下降=中枢依次降低）
    - 盘整：只有一个中枢

    改进：连续尝试配对中枢为趋势，未配对的中枢标记为盘整。
    """
    if not zhongshus:
        return []

    movements: List[Movement] = []
    mv_id = 1
    used = set()  # 已被趋势使用的中枢索引

    # 第一遍：寻找趋势（贪心匹配连续中枢）
    i = 0
    while i < len(zhongshus):
        if i in used:
            i += 1
            continue

        # 尝试从i开始构建趋势
        trend_indices = [i]
        trend_direction = None

        j = i + 1
        while j < len(zhongshus):
            prev_zs = zhongshus[trend_indices[-1]]
            curr_zs = zhongshus[j]

            # 上升：后中枢ZD > 前中枢ZG
            if curr_zs.zd > prev_zs.zg:
                if trend_direction is None:
                    trend_direction = "up"
                if trend_direction == "up":
                    trend_indices.append(j)
                    j += 1
                    continue
                else:
                    break
            # 下降：后中枢ZG < 前中枢ZD
            elif curr_zs.zg < prev_zs.zd:
                if trend_direction is None:
                    trend_direction = "down"
                if trend_direction == "down":
                    trend_indices.append(j)
                    j += 1
                    continue
                else:
                    break
            else:
                break

        if len(trend_indices) >= 2:
            # 形成趋势
            kind = "uptrend" if trend_direction == "up" else "downtrend"
            first_zs = zhongshus[trend_indices[0]]
            last_zs = zhongshus[trend_indices[-1]]
            start_date = first_zs.start_date
            end_date = last_zs.end_date
            try:
                d1 = datetime.datetime.strptime(start_date, "%Y-%m-%d")
                d2 = datetime.datetime.strptime(end_date, "%Y-%m-%d")
                duration = (d2 - d1).days
            except ValueError:
                duration = 0

            movements.append(Movement(
                id=mv_id,
                kind=kind,
                start_date=start_date,
                end_date=end_date,
                duration_days=duration,
                zhongshu_count=len(trend_indices)
            ))
            mv_id += 1
            for idx in trend_indices:
                used.add(idx)
            i = trend_indices[-1] + 1
        else:
            i += 1

    # 第二遍：未被趋势使用的中枢标记为盘整
    for k in range(len(zhongshus)):
        if k not in used:
            zs = zhongshus[k]
            try:
                d1 = datetime.datetime.strptime(zs.start_date, "%Y-%m-%d")
                d2 = datetime.datetime.strptime(zs.end_date, "%Y-%m-%d")
                duration = (d2 - d1).days
            except ValueError:
                duration = 0

            movements.append(Movement(
                id=mv_id,
                kind="consolidation",
                start_date=zs.start_date,
                end_date=zs.end_date,
                duration_days=duration,
                zhongshu_count=1
            ))
            mv_id += 1

    # 按起始日期排序
    movements.sort(key=lambda m: m.start_date)
    # 重新编号
    for idx, mv in enumerate(movements):
        mv.id = idx + 1

    print(f"走势标记: {len(movements)} 段走势 (上升趋势: {sum(1 for m in movements if m.kind == 'uptrend')}, 下降趋势: {sum(1 for m in movements if m.kind == 'downtrend')}, 盘整: {sum(1 for m in movements if m.kind == 'consolidation')})")
    return movements


# ============================================================
# 输出
# ============================================================

def generate_report(
    df: pd.DataFrame,
    bars: List[Bar],
    fractals: List[Fractal],
    strokes: List[Stroke],
    segments: List[Segment],
    zhongshus: List[Zhongshu],
    movements: List[Movement]
) -> str:
    """生成Markdown分析报告"""
    lines = [
        "# 折叠内在性检验 第三轮 — Agent B 缠论结构分析报告",
        "",
        "## 数据概要",
        "",
        f"- 标的: 黄金期货 (GC=F)",
        f"- 时间范围: {df.index[0].strftime('%Y-%m-%d')} ~ {df.index[-1].strftime('%Y-%m-%d')}",
        f"- 原始K线数: {len(df)}",
        f"- 包含处理后K线数: {len(bars)}",
        f"- 分型数: {len(fractals)} (顶: {sum(1 for f in fractals if f.kind == 'top')}, 底: {sum(1 for f in fractals if f.kind == 'bottom')})",
        f"- 笔数: {len(strokes)} (上升: {sum(1 for s in strokes if s.direction == 'up')}, 下降: {sum(1 for s in strokes if s.direction == 'down')})",
        f"- 线段数: {len(segments)}",
        f"- 中枢数: {len(zhongshus)}",
        f"- 走势数: {len(movements)}",
        "",
        "## 方法说明",
        "",
        "### 包含处理",
        "相邻K线存在包含关系时合并。合并方向：看前一根无包含K线——若当前高点>前K线高点则向上合并(取max high, max low)，否则向下合并(取min high, min low)。",
        "",
        "### 分型识别",
        "- 顶分型：中间K线高点和低点都是三根中最高",
        "- 底分型：中间K线低点和高点都是三根中最低",
        "",
        "### 笔构建",
        "使用新笔定义：顶底分型间(不含分型K线)至少3根K线。顶底交替，上升笔顶高于底，下降笔底低于顶。",
        "",
        "### 线段构建",
        "口径A（三笔重叠法）：连续三笔价格区间有公共重叠则形成线段，贪心向后延伸。",
        "",
        "### 中枢构建",
        "至少三条线段有公共重叠区间。中枢区间 = 公共重叠的[ZD, ZG]。贪心延伸：后续线段与[ZD,ZG]有重叠则纳入。",
        "",
        "### 走势标记",
        "- 上升趋势：>=2个中枢，后中枢ZD > 前中枢ZG（中枢依次抬高）",
        "- 下降趋势：>=2个中枢，后中枢ZG < 前中枢ZD（中枢依次降低）",
        "- 盘整：单个中枢",
        "",
    ]

    # 中枢表
    lines.append("## 表一：中枢列表")
    lines.append("")
    lines.append("| 中枢编号 | 起始日期 | 结束日期 | 持续天数 | 区间高点 | 区间低点 |")
    lines.append("|----------|----------|----------|----------|----------|----------|")
    for zs in zhongshus:
        lines.append(f"| {zs.id} | {zs.start_date} | {zs.end_date} | {zs.duration_days} | {zs.zg} | {zs.zd} |")
    lines.append("")

    # 走势表
    lines.append("## 表二：走势列表")
    lines.append("")
    kind_map = {"uptrend": "上升趋势", "downtrend": "下降趋势", "consolidation": "盘整"}
    lines.append("| 走势编号 | 类型 | 起始日期 | 结束日期 | 持续天数 | 中枢数 |")
    lines.append("|----------|------|----------|----------|----------|--------|")
    for mv in movements:
        lines.append(f"| {mv.id} | {kind_map.get(mv.kind, mv.kind)} | {mv.start_date} | {mv.end_date} | {mv.duration_days} | {mv.zhongshu_count} |")
    lines.append("")

    # 统计摘要
    lines.append("## 统计摘要")
    lines.append("")

    if zhongshus:
        avg_duration = sum(z.duration_days for z in zhongshus) / len(zhongshus)
        avg_range = sum(z.zg - z.zd for z in zhongshus) / len(zhongshus)
        lines.append(f"- 平均中枢持续天数: {avg_duration:.1f}")
        lines.append(f"- 平均中枢区间宽度: {avg_range:.2f}")
        lines.append(f"- 最长中枢: {max(z.duration_days for z in zhongshus)} 天")
        lines.append(f"- 最短中枢: {min(z.duration_days for z in zhongshus)} 天")
        lines.append("")

    if movements:
        uptrends = [m for m in movements if m.kind == "uptrend"]
        downtrends = [m for m in movements if m.kind == "downtrend"]
        consolidations = [m for m in movements if m.kind == "consolidation"]
        lines.append(f"- 上升趋势: {len(uptrends)} 段")
        if uptrends:
            lines.append(f"  - 平均持续: {sum(m.duration_days for m in uptrends) / len(uptrends):.0f} 天")
        lines.append(f"- 下降趋势: {len(downtrends)} 段")
        if downtrends:
            lines.append(f"  - 平均持续: {sum(m.duration_days for m in downtrends) / len(downtrends):.0f} 天")
        lines.append(f"- 盘整: {len(consolidations)} 段")
        if consolidations:
            lines.append(f"  - 平均持续: {sum(m.duration_days for m in consolidations) / len(consolidations):.0f} 天")
    lines.append("")

    return "\n".join(lines)


def generate_json(
    zhongshus: List[Zhongshu],
    movements: List[Movement],
    segments: List[Segment],
    strokes: List[Stroke],
    bars: List[Bar],
    raw_count: int
) -> dict:
    """生成结构化JSON"""
    return {
        "meta": {
            "ticker": "GC=F",
            "timeframe": "daily",
            "date_range": [bars[0].date if bars else "", bars[-1].date if bars else ""],
            "raw_bars": raw_count,
            "merged_bars": len(bars),
            "strokes": len(strokes),
            "segments": len(segments),
            "zhongshus": len(zhongshus),
            "movements": len(movements)
        },
        "zhongshus": [
            {
                "id": zs.id,
                "start_date": zs.start_date,
                "end_date": zs.end_date,
                "zg": zs.zg,
                "zd": zs.zd,
                "duration_days": zs.duration_days,
                "seg_count": len(zs.seg_indices)
            }
            for zs in zhongshus
        ],
        "movements": [
            {
                "id": mv.id,
                "kind": mv.kind,
                "start_date": mv.start_date,
                "end_date": mv.end_date,
                "duration_days": mv.duration_days,
                "zhongshu_count": mv.zhongshu_count
            }
            for mv in movements
        ],
        "segments": [
            {
                "direction": seg.direction,
                "start_date": seg.start_date,
                "end_date": seg.end_date,
                "start_price": round(seg.start_price, 2),
                "end_price": round(seg.end_price, 2),
                "stroke_count": seg.stroke_count
            }
            for seg in segments
        ]
    }


# ============================================================
# Main
# ============================================================

def main():
    print("=" * 60)
    print("折叠内在性检验 R3 — Agent B: 缠论结构分析")
    print("=" * 60)

    # 1. 获取数据
    df = fetch_gold_data()

    # 2. 包含处理
    bars = process_inclusion(df)

    # 3. 分型识别
    fractals = find_fractals(bars)

    # 4. 笔构建
    strokes = build_strokes(fractals, bars)

    # 5. 线段构建
    segments = build_segments(strokes)

    # 6. 中枢标记
    zhongshus = build_zhongshus(segments)

    # 7. 走势标记
    movements = build_movements(zhongshus, segments)

    # 输出
    report = generate_report(df, bars, fractals, strokes, segments, zhongshus, movements)
    output_dir = "C:/Users/hanju/NewChanlun/tmp/fold-r3-experiment"

    with open(f"{output_dir}/agent_b_report.md", "w", encoding="utf-8") as f:
        f.write(report)
    print(f"\n报告已保存: {output_dir}/agent_b_report.md")

    result_json = generate_json(zhongshus, movements, segments, strokes, bars, len(df))
    with open(f"{output_dir}/agent_b_results.json", "w", encoding="utf-8") as f:
        json.dump(result_json, f, ensure_ascii=False, indent=2)
    print(f"JSON已保存: {output_dir}/agent_b_results.json")

    print("\n" + "=" * 60)
    print("分析完成。")
    print("=" * 60)


if __name__ == "__main__":
    main()
