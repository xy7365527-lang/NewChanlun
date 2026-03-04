#!/usr/bin/env python
"""Gemini verify round 2: response to genus formula criticism."""
import sys, os
sys.path.insert(0, os.getcwd())
sys.path.insert(0, os.path.join(os.getcwd(), "src"))

from newchan.gemini.modes import GeminiChallenger

SUBJECT = (
    "第二轮质询：回应你对 beta_1 = E - V + C_SCC 的数学否定，给出修正方案\n\n"
    "你在第一轮的否定完全正确。具体确认：\n\n"
    "1. 确认：beta_1 = E - V + C 仅在 C=WCC 时是拓扑不变量（Circuit Rank）。\n"
    "   用 SCC 替换 WCC 后公式不再有拓扑意义。A->B->C 算出 beta_1=2 是荒谬的。\n\n"
    "2. 确认：在累积图中 genus 单调不减，减少拦截机制对累积图无意义。\n"
    "   在快照图中边消失使 C_SCC 增加，beta_1 可能不减反增——拦截方向错误。\n\n"
    "3. 确认：stability_window=3 是操作经验，不是拓扑理由。\n\n"
    "修正方案（两层）：\n\n"
    "层1: 无向 Circuit Rank 作为基础指标\n"
    "  - 使用 beta_1 = E - V + C_WCC（抹掉方向后的标准 Circuit Rank）\n"
    "  - 数学上正确地计算无向意义上的独立环数\n"
    "  - 这给出矛盾结构的基础拓扑复杂度\n\n"
    "层2: 有向环路直接计数作为方向敏感指标\n"
    "  - 用 networkx simple_cycles() 提取所有有向简单环路\n"
    "  - 有向环路 A->B->C->A 意味着三个立场形成循环否定——这才是有意义的方向敏感洞\n"
    "  - 有向环路数 = SCC 中包含 >= 2 节点的分量内部的环路数\n"
    "  - 这保留了编排者的直觉：否定方向不可抹掉\n\n"
    "收敛条件修正：\n"
    "  - 无向 Circuit Rank 稳定 AND 有向环路集合稳定 -> 收敛\n"
    "  - 有向环路消失 -> 方向敏感的 lack 丢失拦截（比无向 genus 减少更精确）\n"
    "  - 有向环路新增 -> 新矛盾环路正在形成，继续\n\n"
    "关于 stability_window 的修正：\n"
    "  你说得对，3 不是拓扑理由。修正：\n"
    "  - stability_window 由图的直径决定：diameter(G) + 1\n"
    "  - 直径 = 图中最长最短路径。直径 + 1 轮的稳定意味着信息有足够时间在整个图上传播\n"
    "  - 下限 clamp 为 3（操作性约束：至少一次完整交替）\n\n"
    "残存问题：\n"
    "  - simple_cycles() 对大图的计算复杂度是指数级的——立场图会大到不可计算吗？\n"
    "  - 立场图的边构造规则：什么构成有向边？只是共存 stance key 的共存张力？\n"
    "    还是应该从 StanceDiff 的 changed/removed 来推导有向否定关系？\n\n"
    "请再次严格质询修正方案。尤其是：\n"
    "1. 两层指标是否冗余——一层够吗？\n"
    "2. diameter + 1 作为 stability_window 是否数学上严格？\n"
    "3. 立场图的边应该怎么构造才最严格？\n"
    "4. 有向环路消失的检测在累积图 vs 快照图上的行为？"
)

CONTEXT = (
    "第一轮 Gemini 质询结果：\n"
    "verdict: fail\n"
    "stances:\n"
    "  betti_number_scc_formula: contradictory\n"
    "  genus_decrease_interception: contradictory\n"
    "  stability_window_topology: reject\n"
    "  directed_graph_homology: needs_work\n\n"
    "关键否定：\n"
    "- A->B->C（3节点, 2边, 3 SCC）-> beta_1 = 2 - 3 + 3 = 2，一条线段算出2个洞\n"
    "- 累积图中 genus 单调不减，减少拦截无意义\n"
    "- stability_window=3 是操作经验不是拓扑理由\n\n"
    "体系约束：\n"
    "- 否定方向不可抹掉（编排者决断）\n"
    "- 收敛不是消除矛盾，是矛盾的显式登记\n"
    "- lack 不能被无声消解\n"
    "- 需要运行时硬约束（不是 advisory）"
)

challenger = GeminiChallenger()
result = challenger.verify(SUBJECT, CONTEXT)
print(result.response)
print("\n---MODEL---")
print(result.model)
