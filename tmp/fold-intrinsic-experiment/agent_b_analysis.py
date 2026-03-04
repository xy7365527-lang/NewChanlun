"""
缠论结构分析：K线包含 → 分型 → 笔 → 线段 → 中枢 → 状态转换

严格依据《缠论知识库.md》定义实现。
认识论等级：L2（真实数据验证，单标的日线 2000-2026）
"""

import csv
import json
from dataclasses import dataclass, field, asdict
from typing import List, Optional, Tuple
from datetime import datetime


# ============================================================
# 1. 数据结构
# ============================================================

@dataclass(frozen=True)
class RawBar:
    """原始K线"""
    date: str
    open: float
    high: float
    low: float
    close: float
    volume: int
    index: int  # 原始序号


@dataclass(frozen=True)
class MergedBar:
    """包含处理后的K线"""
    high: float
    low: float
    start_date: str   # 合并区间起始日期
    end_date: str      # 合并区间结束日期
    index: int         # 在合并序列中的序号
    raw_count: int     # 合并了几根原始K线


@dataclass(frozen=True)
class Fractal:
    """分型"""
    type: str          # 'top' or 'bottom'
    date: str          # 分型中间K线的日期
    price: float       # 顶分型取high，底分型取low
    high: float        # 中间K线high
    low: float         # 中间K线low
    bar_index: int     # 在合并K线序列中的索引（中间那根）


@dataclass(frozen=True)
class Stroke:
    """笔"""
    start: Fractal
    end: Fractal
    direction: str     # 'up' or 'down'
    index: int         # 笔序号


@dataclass(frozen=True)
class Segment:
    """线段"""
    start_stroke_idx: int
    end_stroke_idx: int
    direction: str     # 'up' or 'down'
    high: float
    low: float
    start_date: str
    end_date: str
    index: int


@dataclass
class Zhongshu:
    """中枢"""
    zd: float          # 中枢下沿
    zg: float          # 中枢上沿
    gg: float          # 最高点
    dd: float          # 最低点
    start_date: str
    end_date: str
    start_seg_idx: int
    end_seg_idx: int
    status: str        # 'extending' / 'expanding' / 'newborn'
    transitions: List[dict] = field(default_factory=list)
    index: int = 0


# ============================================================
# 2. K线包含处理（§2）
# ============================================================

def load_data(filepath: str) -> List[RawBar]:
    """加载CSV数据"""
    bars = []
    with open(filepath, 'r') as f:
        reader = csv.DictReader(f)
        for i, row in enumerate(reader):
            bars.append(RawBar(
                date=row['Date'],
                open=float(row['Open']),
                high=float(row['High']),
                low=float(row['Low']),
                close=float(row['Close']),
                volume=int(row['Volume']),
                index=i,
            ))
    return bars


def has_inclusion(bar1_high, bar1_low, bar2_high, bar2_low) -> bool:
    """判断两根K线是否存在包含关系（§2.1）"""
    # bar1 包含 bar2
    if bar1_high >= bar2_high and bar1_low <= bar2_low:
        return True
    # bar2 包含 bar1
    if bar2_high >= bar1_high and bar2_low <= bar1_low:
        return True
    return False


def merge_bars(raw_bars: List[RawBar]) -> List[MergedBar]:
    """
    K线包含处理（§2.2-2.4）
    - 向上包含：high=max, low=max
    - 向下包含：high=min, low=min
    - 先左后右顺序处理
    """
    if len(raw_bars) < 2:
        return [MergedBar(
            high=raw_bars[0].high, low=raw_bars[0].low,
            start_date=raw_bars[0].date, end_date=raw_bars[0].date,
            index=0, raw_count=1
        )]

    # 初始化：第一根K线直接加入
    merged: List[dict] = [{
        'high': raw_bars[0].high,
        'low': raw_bars[0].low,
        'start_date': raw_bars[0].date,
        'end_date': raw_bars[0].date,
        'raw_count': 1,
    }]

    for i in range(1, len(raw_bars)):
        cur = raw_bars[i]
        prev = merged[-1]

        if has_inclusion(prev['high'], prev['low'], cur.high, cur.low):
            # 确定合并方向：看前一根（非包含的）与当前的关系
            # 如果merged只有一根，用第一根的方向
            if len(merged) >= 2:
                ref = merged[-2]
            else:
                ref = {'high': raw_bars[0].high, 'low': raw_bars[0].low}

            if prev['high'] > ref['high']:
                # 向上包含：取高高低低（high=max, low=max）
                new_high = max(prev['high'], cur.high)
                new_low = max(prev['low'], cur.low)
            else:
                # 向下包含：取低低高高（high=min, low=min）
                new_high = min(prev['high'], cur.high)
                new_low = min(prev['low'], cur.low)

            merged[-1] = {
                'high': new_high,
                'low': new_low,
                'start_date': prev['start_date'],
                'end_date': cur.date,
                'raw_count': prev['raw_count'] + 1,
            }
        else:
            merged.append({
                'high': cur.high,
                'low': cur.low,
                'start_date': cur.date,
                'end_date': cur.date,
                'raw_count': 1,
            })

    result = []
    for idx, m in enumerate(merged):
        result.append(MergedBar(
            high=m['high'], low=m['low'],
            start_date=m['start_date'], end_date=m['end_date'],
            index=idx, raw_count=m['raw_count'],
        ))
    return result


# ============================================================
# 3. 分型识别（§3）
# ============================================================

def find_fractals(bars: List[MergedBar]) -> List[Fractal]:
    """
    在包含处理后的K线序列上识别顶底分型（§3.1-3.2）
    顶分型：中间K线高点最高且低点最高
    底分型：中间K线低点最低且高点最低
    """
    fractals = []
    for i in range(1, len(bars) - 1):
        prev_bar, cur_bar, next_bar = bars[i - 1], bars[i], bars[i + 1]

        # 顶分型
        if (cur_bar.high > prev_bar.high and cur_bar.high > next_bar.high
                and cur_bar.low > prev_bar.low and cur_bar.low > next_bar.low):
            fractals.append(Fractal(
                type='top', date=cur_bar.start_date,
                price=cur_bar.high, high=cur_bar.high, low=cur_bar.low,
                bar_index=cur_bar.index,
            ))

        # 底分型
        elif (cur_bar.low < prev_bar.low and cur_bar.low < next_bar.low
              and cur_bar.high < prev_bar.high and cur_bar.high < next_bar.high):
            fractals.append(Fractal(
                type='bottom', date=cur_bar.start_date,
                price=cur_bar.low, high=cur_bar.high, low=cur_bar.low,
                bar_index=cur_bar.index,
            ))

    return fractals


# ============================================================
# 4. 笔的构建（§4）
# ============================================================

def build_strokes(fractals: List[Fractal], merged_bars: List[MergedBar]) -> List[Stroke]:
    """
    从分型序列构建笔（§4.1-4.4）

    采用逐步推进法：
    1. 从第一个分型开始，寻找能配对的异类分型
    2. 配对条件：顶底交替、间距>=4根合并K线、顶高于底
    3. 同类分型竞争时：顶取最高、底取最低
    4. 一旦配对成功，形成一笔，从终点继续
    """
    if not fractals:
        return []

    strokes = []
    stroke_idx = 0

    # 当前锚点分型
    anchor = fractals[0]
    i = 1

    while i < len(fractals):
        cur = fractals[i]

        if cur.type == anchor.type:
            # 同类分型：更新锚点（顶取高、底取低）
            if cur.type == 'top' and cur.price > anchor.price:
                anchor = cur
            elif cur.type == 'bottom' and cur.price < anchor.price:
                anchor = cur
            i += 1
            continue

        # 异类分型：检查是否能成笔
        bar_gap = abs(cur.bar_index - anchor.bar_index)
        if bar_gap < 4:
            i += 1
            continue

        # 顶高于底检查
        if anchor.type == 'top' and cur.type == 'bottom':
            if anchor.price <= cur.price:
                # 不满足顶高于底，用当前分型替换锚点
                anchor = cur
                i += 1
                continue
        elif anchor.type == 'bottom' and cur.type == 'top':
            if cur.price <= anchor.price:
                anchor = cur
                i += 1
                continue

        # 成笔
        direction = 'up' if anchor.type == 'bottom' else 'down'
        strokes.append(Stroke(
            start=anchor, end=cur,
            direction=direction, index=stroke_idx,
        ))
        stroke_idx += 1
        anchor = cur
        i += 1

    return strokes


# ============================================================
# 5. 线段构建（§5 特征序列法）
# ============================================================

def _stroke_high(s: Stroke) -> float:
    return max(s.start.price, s.end.price)


def _stroke_low(s: Stroke) -> float:
    return min(s.start.price, s.end.price)


def _stroke_range(s: Stroke) -> Tuple[float, float]:
    """返回笔的 (low, high)"""
    return (_stroke_low(s), _stroke_high(s))


def build_segments(strokes: List[Stroke]) -> List[Segment]:
    """
    线段构建（§5.1-5.5 特征序列法）
    线段至少3笔，前三笔必须有重叠。
    使用特征序列法判断线段终结。
    """
    if len(strokes) < 3:
        return []

    segments = []
    seg_start = 0  # 当前线段起始笔索引
    seg_idx = 0

    while seg_start < len(strokes) - 2:
        # 确定线段方向：与起始笔方向一致
        seg_dir = strokes[seg_start].direction

        # 尝试延伸线段
        seg_end = _find_segment_end(strokes, seg_start, seg_dir)

        if seg_end is None:
            seg_start += 1
            continue

        # 验证前三笔有重叠
        if not _three_strokes_overlap(strokes, seg_start, seg_start + 2):
            seg_start += 1
            continue

        # 构建线段
        seg_strokes = strokes[seg_start:seg_end + 1]
        seg_high = max(_stroke_high(s) for s in seg_strokes)
        seg_low = min(_stroke_low(s) for s in seg_strokes)

        start_date = strokes[seg_start].start.date
        end_date = strokes[seg_end].end.date

        segments.append(Segment(
            start_stroke_idx=seg_start,
            end_stroke_idx=seg_end,
            direction=seg_dir,
            high=seg_high,
            low=seg_low,
            start_date=start_date,
            end_date=end_date,
            index=seg_idx,
        ))
        seg_idx += 1
        seg_start = seg_end

    return segments


def _three_strokes_overlap(
    strokes: List[Stroke], i: int, j: int,
) -> bool:
    """检查从i到j的笔是否有重叠区间"""
    if j - i < 2:
        return False
    ranges = [_stroke_range(strokes[k]) for k in range(i, j + 1)]
    overlap_low = max(r[0] for r in ranges)
    overlap_high = min(r[1] for r in ranges)
    return overlap_high > overlap_low


def _find_segment_end(
    strokes: List[Stroke], start: int, direction: str,
) -> Optional[int]:
    """
    用特征序列法找线段终结点（§5.3-5.5）
    """
    n = len(strokes)
    if start + 2 >= n:
        return None

    # 收集特征序列元素（反向笔）
    char_indices = []
    for i in range(start, n):
        s = strokes[i]
        if direction == 'up' and s.direction == 'down':
            char_indices.append(i)
        elif direction == 'down' and s.direction == 'up':
            char_indices.append(i)

    if len(char_indices) < 1:
        return None

    # 对特征序列做包含处理
    char_elements = []
    for ci in char_indices:
        s = strokes[ci]
        char_elements.append({
            'high': _stroke_high(s),
            'low': _stroke_low(s),
            'stroke_idx': ci,
        })

    std_chars = _merge_char_sequence(char_elements, direction)

    if len(std_chars) < 3:
        last_char_stroke = char_indices[-1] if char_indices else start + 2
        end = max(start + 2, last_char_stroke)
        if end < n:
            return end
        return n - 1 if n - 1 > start else None

    for i in range(1, len(std_chars) - 1):
        prev_c = std_chars[i - 1]
        cur_c = std_chars[i]
        next_c = std_chars[i + 1]

        if direction == 'up':
            # 顶分型
            if (cur_c['high'] > prev_c['high'] and cur_c['high'] > next_c['high']
                    and cur_c['low'] > prev_c['low'] and cur_c['low'] > next_c['low']):
                term_stroke = cur_c['stroke_idx']
                if term_stroke > start + 1:
                    return term_stroke - 1
        else:
            # 底分型
            if (cur_c['low'] < prev_c['low'] and cur_c['low'] < next_c['low']
                    and cur_c['high'] < prev_c['high'] and cur_c['high'] < next_c['high']):
                term_stroke = cur_c['stroke_idx']
                if term_stroke > start + 1:
                    return term_stroke - 1

    last_stroke = char_indices[-1] if char_indices else start + 2
    if last_stroke >= n:
        last_stroke = n - 1
    if last_stroke <= start:
        return None
    return last_stroke


def _merge_char_sequence(elements: List[dict], seg_direction: str) -> List[dict]:
    """
    对特征序列做包含处理（§5.4）
    特征序列元素当K线处理，方向由线段方向决定：
    - 向上线段的特征序列（向下笔）：向下处理
    - 向下线段的特征序列（向上笔）：向上处理
    """
    if len(elements) < 2:
        return list(elements)

    result = [dict(elements[0])]

    for i in range(1, len(elements)):
        cur = elements[i]
        prev = result[-1]

        if has_inclusion(prev['high'], prev['low'], cur['high'], cur['low']):
            if seg_direction == 'up':
                # 特征序列向下处理：取低低高高
                new_high = min(prev['high'], cur['high'])
                new_low = min(prev['low'], cur['low'])
            else:
                # 特征序列向上处理：取高高低低
                new_high = max(prev['high'], cur['high'])
                new_low = max(prev['low'], cur['low'])
            result[-1] = {
                'high': new_high,
                'low': new_low,
                'stroke_idx': cur['stroke_idx'],
            }
        else:
            result.append(dict(cur))

    return result


# ============================================================
# 6. 中枢识别与状态转换（§6）
# ============================================================

def build_zhongshus(segments: List[Segment]) -> List[Zhongshu]:
    """
    从线段序列构建中枢（§6.1-6.7）

    中枢定义：至少3个连续次级别走势类型有重叠区间。
    这里以线段作为次级别走势类型的代理。

    中枢区间 [ZD, ZG] 由前三段的重叠确定（§6.3-6.4）。
    后续线段进入中枢区间 → 延伸；
    后续线段与中枢区间有重叠但超出 [DD, GG] → 扩展；
    后续线段完全离开中枢区间 → 新生（旧中枢终结）。
    """
    if len(segments) < 3:
        return []

    zhongshus = []
    zs_idx = 0
    i = 0

    while i < len(segments) - 2:
        # 尝试用 segments[i], [i+1], [i+2] 构建中枢
        s1, s2, s3 = segments[i], segments[i + 1], segments[i + 2]

        # 计算三段重叠
        overlap_low = max(s1.low, s2.low, s3.low)
        overlap_high = min(s1.high, s2.high, s3.high)

        if overlap_high <= overlap_low:
            i += 1
            continue

        # 中枢成立
        zd = overlap_low
        zg = overlap_high
        gg = max(s1.high, s2.high, s3.high)
        dd = min(s1.low, s2.low, s3.low)

        zs = Zhongshu(
            zd=zd, zg=zg, gg=gg, dd=dd,
            start_date=s1.start_date,
            end_date=s3.end_date,
            start_seg_idx=i,
            end_seg_idx=i + 2,
            status='extending',
            transitions=[],
            index=zs_idx,
        )

        # 尝试用后续线段延伸/扩展中枢
        j = i + 3
        while j < len(segments):
            seg = segments[j]
            seg_low = seg.low
            seg_high = seg.high

            # 判断该线段与中枢的关系
            has_overlap_with_zs = seg_high > zs.zd and seg_low < zs.zg

            if has_overlap_with_zs:
                old_status = zs.status
                # 线段与中枢区间有重叠
                # 检查是否扩展（超出原 GG/DD）
                new_gg = max(zs.gg, seg_high)
                new_dd = min(zs.dd, seg_low)

                if new_gg > zs.gg or new_dd < zs.dd:
                    # 扩展：中枢范围扩大
                    if zs.status != 'expanding':
                        zs.transitions.append({
                            'from': old_status,
                            'to': 'expanding',
                            'date': seg.start_date,
                            'trigger_seg_idx': j,
                        })
                    zs.status = 'expanding'

                zs.gg = new_gg
                zs.dd = new_dd
                zs.end_date = seg.end_date
                zs.end_seg_idx = j
                j += 1
            else:
                # 线段完全离开中枢 → 中枢终结，新生
                break

        zhongshus.append(zs)
        zs.index = zs_idx
        zs_idx += 1

        # 下一个中枢从离开点开始
        if j < len(segments):
            i = j - 1  # 回退一段，让新中枢有机会从离开段开始
            if i <= zs.end_seg_idx:
                i = zs.end_seg_idx + 1
        else:
            break

    # 标记最后一个中枢之后的"新生"状态
    # 如果最后一个中枢被离开，后续形成新中枢，则旧中枢状态为 newborn
    for k in range(len(zhongshus) - 1):
        cur_zs = zhongshus[k]
        next_zs = zhongshus[k + 1]
        # 如果下一个中枢的区间与当前中枢不重叠 → 当前中枢已被离开
        if next_zs.zd >= cur_zs.zg or next_zs.zg <= cur_zs.zd:
            if cur_zs.status != 'newborn':
                cur_zs.transitions.append({
                    'from': cur_zs.status,
                    'to': 'newborn',
                    'date': next_zs.start_date,
                    'trigger_seg_idx': next_zs.start_seg_idx,
                })
                cur_zs.status = 'newborn'

    return zhongshus


# ============================================================
# 7. 序列化与输出
# ============================================================

def zhongshu_to_dict(zs: Zhongshu) -> dict:
    return {
        'index': zs.index,
        'zd': round(zs.zd, 4),
        'zg': round(zs.zg, 4),
        'gg': round(zs.gg, 4),
        'dd': round(zs.dd, 4),
        'start_date': zs.start_date,
        'end_date': zs.end_date,
        'start_seg_idx': zs.start_seg_idx,
        'end_seg_idx': zs.end_seg_idx,
        'status': zs.status,
        'transitions': zs.transitions,
    }


def generate_summary(
    raw_count: int,
    merged_count: int,
    fractal_count: int,
    stroke_count: int,
    segment_count: int,
    zhongshus: List[Zhongshu],
) -> str:
    """生成人类可读的中文摘要"""
    lines = [
        "# 缠论结构分析摘要",
        "",
        "## 基础数据",
        "",
        f"- 原始K线数量：{raw_count}",
        f"- 包含处理后K线数量：{merged_count}",
        f"- 分型数量：{fractal_count}",
        f"- 笔数量：{stroke_count}",
        f"- 线段数量：{segment_count}",
        f"- 中枢数量：{len(zhongshus)}",
        "",
        "## 中枢状态统计",
        "",
    ]

    status_counts = {'extending': 0, 'expanding': 0, 'newborn': 0}
    for zs in zhongshus:
        status_counts[zs.status] = status_counts.get(zs.status, 0) + 1

    lines.append(f"- 延伸中（extending）：{status_counts.get('extending', 0)}")
    lines.append(f"- 扩展中（expanding）：{status_counts.get('expanding', 0)}")
    lines.append(f"- 已新生（newborn）：{status_counts.get('newborn', 0)}")
    lines.append("")

    # 统计状态转换
    total_transitions = sum(len(zs.transitions) for zs in zhongshus)
    lines.append(f"## 状态转换总数：{total_transitions}")
    lines.append("")

    # 列出所有状态转换
    if total_transitions > 0:
        lines.append("| 中枢序号 | 转换 | 时间 | 触发线段 |")
        lines.append("|---------|------|------|---------|")
        for zs in zhongshus:
            for t in zs.transitions:
                from_cn = _status_cn(t['from'])
                to_cn = _status_cn(t['to'])
                lines.append(
                    f"| {zs.index} | {from_cn} → {to_cn} "
                    f"| {t['date']} | 线段#{t['trigger_seg_idx']} |"
                )
        lines.append("")

    # 中枢详情（前20个 + 最后5个）
    lines.append("## 中枢详情（部分）")
    lines.append("")
    lines.append("| 序号 | 区间 [ZD, ZG] | 时间范围 | 状态 | 包含线段数 |")
    lines.append("|------|--------------|---------|------|-----------|")

    display_zhongshus = zhongshus[:20]
    if len(zhongshus) > 25:
        display_zhongshus.append(None)  # separator
        display_zhongshus.extend(zhongshus[-5:])
    elif len(zhongshus) > 20:
        display_zhongshus = zhongshus

    for zs in display_zhongshus:
        if zs is None:
            lines.append("| ... | ... | ... | ... | ... |")
            continue
        seg_count = zs.end_seg_idx - zs.start_seg_idx + 1
        lines.append(
            f"| {zs.index} | [{zs.zd:.2f}, {zs.zg:.2f}] "
            f"| {zs.start_date} ~ {zs.end_date} "
            f"| {_status_cn(zs.status)} | {seg_count} |"
        )

    lines.append("")
    lines.append("## 认识论等级")
    lines.append("")
    lines.append("L2：真实数据验证（单标的日线，2000-2026，约6400根K线）。")
    lines.append("结论的有效域限于该标的该时段。")
    lines.append("")

    return "\n".join(lines)


def _status_cn(status: str) -> str:
    mapping = {
        'extending': '延伸',
        'expanding': '扩展',
        'newborn': '新生',
    }
    return mapping.get(status, status)


# ============================================================
# 8. 主流程
# ============================================================

def main():
    import os

    base_dir = os.path.dirname(os.path.abspath(__file__))
    input_path = os.path.join(base_dir, "agent_b_input.csv")

    print("加载数据...")
    raw_bars = load_data(input_path)
    print(f"  原始K线：{len(raw_bars)} 根")

    print("K线包含处理...")
    merged = merge_bars(raw_bars)
    print(f"  合并后K线：{len(merged)} 根")

    print("分型识别...")
    fractals = find_fractals(merged)
    top_count = sum(1 for f in fractals if f.type == 'top')
    bot_count = sum(1 for f in fractals if f.type == 'bottom')
    print(f"  分型：{len(fractals)} 个（顶 {top_count}，底 {bot_count}）")

    print("笔构建...")
    strokes = build_strokes(fractals, merged)
    up_count = sum(1 for s in strokes if s.direction == 'up')
    dn_count = sum(1 for s in strokes if s.direction == 'down')
    print(f"  笔：{len(strokes)} 根（上 {up_count}，下 {dn_count}）")

    print("线段构建...")
    segments = build_segments(strokes)
    print(f"  线段：{len(segments)} 段")

    print("中枢识别...")
    zhongshus = build_zhongshus(segments)
    print(f"  中枢：{len(zhongshus)} 个")

    # 输出结果
    results = {
        'metadata': {
            'raw_bars': len(raw_bars),
            'merged_bars': len(merged),
            'fractals': len(fractals),
            'strokes': len(strokes),
            'segments': len(segments),
            'zhongshus': len(zhongshus),
            'date_range': f"{raw_bars[0].date} ~ {raw_bars[-1].date}",
        },
        'zhongshus': [zhongshu_to_dict(zs) for zs in zhongshus],
    }

    results_path = os.path.join(base_dir, "agent_b_results.json")
    with open(results_path, 'w', encoding='utf-8') as f:
        json.dump(results, f, ensure_ascii=False, indent=2)
    print(f"  结果已保存：{results_path}")

    # 状态转换时间戳
    timestamps = []
    for zs in zhongshus:
        for t in zs.transitions:
            timestamps.append({
                'zhongshu_index': zs.index,
                'from_status': t['from'],
                'to_status': t['to'],
                'date': t['date'],
                'trigger_seg_idx': t['trigger_seg_idx'],
            })

    ts_path = os.path.join(base_dir, "agent_b_timestamps.json")
    with open(ts_path, 'w', encoding='utf-8') as f:
        json.dump(timestamps, f, ensure_ascii=False, indent=2)
    print(f"  时间戳已保存：{ts_path}")

    # 摘要
    summary = generate_summary(
        len(raw_bars), len(merged), len(fractals),
        len(strokes), len(segments), zhongshus,
    )
    summary_path = os.path.join(base_dir, "agent_b_summary.md")
    with open(summary_path, 'w', encoding='utf-8') as f:
        f.write(summary)
    print(f"  摘要已保存：{summary_path}")

    print("完成。")


if __name__ == '__main__':
    main()
