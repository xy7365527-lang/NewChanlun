#!/usr/bin/env python
"""Gemini verify round 3: response to second round criticism."""
import sys, os
sys.path.insert(0, os.getcwd())
sys.path.insert(0, os.path.join(os.getcwd(), "src"))

from newchan.gemini.modes import GeminiChallenger

SUBJECT = (
    "第三轮质询：你的四个否定全部成立。以下是逐条确认和收敛后的方案。\n\n"
    "确认1: Layer 1（无向 Circuit Rank）废弃。一层有向就够。\n"
    "确认2: diameter + 1 废弃。动态图直径追不上，非连通图直径无穷大。\n"
    "确认3: 累积图与环路消失互斥。废弃累积图方案。\n"
    "确认4: 边必须有因果语义。从 StanceDiff 的因果链构造。\n\n"
    "收敛后的方案（单层有向因果图）：\n\n"
    "图定义：\n"
    "  - 节点 = stance key（具体议题）\n"
    "  - 有向边 X -> Y = StanceDiff 中 Y 的变更是 X 存在的因果后果\n"
    "    具体：如果在 round t 中，stance X 保持不变，但 stance Y 因为 X 而发生变更，\n"
    "    则添加 X -> Y。因果关系从 StanceDiff.changed 的上下文中推导。\n"
    "  - 图是滑动窗口（不是累积也不是快照）：保留最近 W 轮的因果边\n\n"
    "有向环路计数：\n"
    "  - 直接用 simple_cycles() 在滑动窗口图上提取有向环路\n"
    "  - 有向环路 A->B->C->A = 三个立场形成循环否定 = 结构性死锁\n"
    "  - cycle_count = len(list(nx.simple_cycles(G)))\n\n"
    "收敛条件：\n"
    "  - 有向环路集合在 N 轮内保持不变（集合相等，不是数量相等）\n"
    "  - N 不是硬编码数字——N = 参与主体数 + 1（2 个主体时 N=3，3 个时 N=4）\n"
    "  - 理由：每个主体至少有一次机会回应所有现存环路，+ 1 轮确认无变化\n\n"
    "环路消失处理（滑动窗口语义）：\n"
    "  - 滑动窗口中，旧边自然滑出 = 环路可以消失\n"
    "  - 但边是否滑出取决于因果链是否仍然活跃：\n"
    "    如果 X -> Y 的因果关系在最近 W 轮中都未被引用 -> 该边退出\n"
    "    如果 X -> Y 被反复引用 -> 该边保留\n"
    "  - 因此环路消失意味着组成该环路的所有因果关系都已衰减 = 矛盾被自然解决\n"
    "  - 环路在窗口外消失 ≠ lack 丢失（是自然衰减）\n"
    "  - 环路在窗口内突然消失（某条边被刻意不引用）→ lack 丢失拦截\n\n"
    "simple_cycles 复杂度问题：\n"
    "  - 立场图不会很大——每轮 stance 通常 5-15 个 key\n"
    "  - 滑动窗口限制了图的最大规模\n"
    "  - W = 2 * N（保留足够轮次的因果信息）\n\n"
    "residue 定义（从环路推导）：\n"
    "  - 收敛时的有向环路集合 = unresolved（tension 区块内容）\n"
    "  - 每个环路的 stance keys = 一组不可解的循环否定\n"
    "  - residue = 这些环路中参与者各自放弃的立场（从 ConcessionTrace 推导）\n\n"
    "请质询：\n"
    "1. 因果边的推导——从 StanceDiff 怎么确定 X -> Y 的因果方向？\n"
    "   StanceDiff 只给了 changed/removed/added，没有因果链信息\n"
    "2. 滑动窗口 W = 2N 是否有数学理由\n"
    "3. 窗口内突然消失 vs 窗口外自然衰减的区分是否可靠\n"
    "4. 还有没有我遗漏的致命问题"
)

CONTEXT = (
    "第一轮 Gemini 否定：SCC 替换 WCC 数学错误\n"
    "第二轮 Gemini 否定：无向指标是噪声、diameter+1 活锁、累积图环路不消失、边缺因果性\n"
    "两轮 concessions: []\n\n"
    "收敛趋势：\n"
    "- 从两层指标收敛到单层有向\n"
    "- 从累积/快照二选一收敛到滑动窗口\n"
    "- 从 beta_1 公式收敛到直接环路计数\n"
    "- 从硬编码 stability_window 收敛到 参与主体数 + 1\n\n"
    "尚未让步的立场：\n"
    "- 否定方向不可抹掉\n"
    "- 收敛不是消除矛盾而是显式登记\n"
    "- lack 不能被无声消解\n"
    "- 无轮数上限"
)

challenger = GeminiChallenger()
result = challenger.verify(SUBJECT, CONTEXT)
print(result.response)
print("\n---MODEL---")
print(result.model)
