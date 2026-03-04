#!/usr/bin/env python
"""Gemini verify: genus convergence design."""
import sys, os
sys.path.insert(0, os.getcwd())
sys.path.insert(0, os.path.join(os.getcwd(), "src"))

from newchan.gemini.modes import GeminiChallenger

SUBJECT = (
    "多轮质询管道的拓扑收敛判据设计——用第一 Betti 数（亏格）判断质询循环何时停止\n\n"
    "方案核心：\n"
    "1. 质询过程的拓扑由\"立场图\"（stance graph）表示\n"
    "   - 节点 = stance key（具体议题）\n"
    "   - 边 = 同一轮内共存的 stance 对之间的张力关系（有方向——否定方向不可抹掉）\n"
    "   - genus = 该图的第一 Betti 数 beta_1 = E - V + C（C = 强连通分量数）\n\n"
    "2. 收敛条件：genus 在连续 3 轮（stability_window=3）内保持不变\n"
    "   - 3 轮保证至少经历一次完整的 Gemini->Codex->Gemini 交替\n"
    "   - 无轮数上限——收敛由拓扑判据决定，不是预设阈值\n\n"
    "3. genus 减少 = 危险信号\n"
    "   - 矛盾在被人为消解，lack 在丢失\n"
    "   - 拦截机制：将消失的环路注入下一轮 context，强制双方回应\n\n"
    "4. residue 定义\n"
    "   - residue = 收敛时拓扑中剩余的洞（不可消去的亏格）\n"
    "   - genus > 0 时，每个独立环路映射为 unresolved 项（tension 区块的内容）\n"
    "   - 收敛不是消除矛盾，是矛盾的显式登记\n\n"
    "5. 选择强连通分量而非弱连通分量\n"
    "   - 否定方向是有意义的（谱系生成方向、否定方向）\n"
    "   - 弱连通 = 抹掉方向后看\n"
    "   - 强连通 = 沿否定方向看\n"
    "   - 后者更严格——收敛更难达到\n\n"
    "请严格质询此设计的数学严格性、概念一致性、和可能的致命缺陷。"
)

CONTEXT = (
    "体系背景：\n"
    "- 187号谱系诊断：计算/对话架构分离是伪分离——对话过程完全缺席\n"
    "- Gemini 原创判断：亏格必须由异质主体碰撞产生。计算过程再完备，产生的是 error 不是 lack\n"
    "- 拓扑操作是确定性的，但判断的对象是对话过程留下的结构痕迹，不是对话内容本身\n"
    "- stance 差分架构已有：StanceDeclaration（每轮立场）、StanceDiff（轮间差分）、ConcessionTrace（让步轨迹）\n"
    "- consensus_ceremony 已有三区块原子写入：consensus + residue + tension\n\n"
    "潜在问题方向（请质询但不限于这些）：\n"
    "1. beta_1 = E - V + C 对有向图的适用性——这是无向图拓扑不变量，对有向图是否退化？\n"
    "2. 立场图的构造规则——stance key 之间的边是什么？同一轮共存就连边太弱了吗？\n"
    "3. 强连通分量对 DAG 结构的含义——如果立场图是 DAG（无环），genus 永远是 E-V+V=E，这有意义吗？\n"
    "4. stability_window=3 的数学意义是什么？不是经验好数字，是有拓扑学理由吗？\n"
    "5. 从 genus 减少推出 lack 丢失——这个推导链的每一步都严格吗？\n"
    "6. 环路 = unresolved 项的映射——如果一个环路包含已让步的 stance key 怎么办？"
)

challenger = GeminiChallenger()
result = challenger.verify(SUBJECT, CONTEXT)
print(result.response)
print("\n---MODEL---")
print(result.model)
