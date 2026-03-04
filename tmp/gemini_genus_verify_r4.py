#!/usr/bin/env python
"""Gemini verify round 4: addressing the core dilemma."""
import sys, os
sys.path.insert(0, os.getcwd())
sys.path.insert(0, os.path.join(os.getcwd(), "src"))

from newchan.gemini.modes import GeminiChallenger

SUBJECT = (
    "第四轮：你精确命中了核心二难困境。三个否定全部确认。\n\n"
    "确认1: 死锁时立场不变更 -> 不产生 StanceDiff -> 滑动窗口中死锁边滑出 -> 死锁被误判为衰减。\n"
    "        滑动窗口方案废弃。\n\n"
    "确认2: StanceDiff 没有因果链信息。X 不变 Y 变了不蕴含 X 导致 Y。\n"
    "        因果边从 StanceDiff 推导不可能。废弃因果边方案。\n\n"
    "确认3: 滑动窗口 = 遗忘 = 允许转移话题消解 lack。直接摧毁核心原则。废弃。\n\n"
    "核心二难困境（你发现的）：\n"
    "  死锁需要记忆 -> 需要累积图\n"
    "  累积图中环路不消失 -> genus 单调不减 -> 收敛条件永远不满足\n"
    "  这两个需求互斥。\n\n"
    "但是——这个二难困境是否有第三条路？\n\n"
    "提案：放弃用图拓扑做收敛判断。\n\n"
    "回到编排者的原始直觉：拓扑操作判断的对象是对话过程留下的结构痕迹。\n"
    "但什么是结构痕迹？不一定是图上的环路。\n\n"
    "重新定义：\n"
    "  结构痕迹 = StanceDiff 序列本身。\n"
    "  收敛 = StanceDiff 序列收敛到不动点。\n\n"
    "具体：\n"
    "  - 每轮产生一个 StanceDiff（已有基础设施，不需要构造图）\n"
    "  - diff_t = compute_stance_diff(stance_{t-1}, stance_t)\n"
    "  - 收敛 = 连续 N 轮的 StanceDiff 都是空的（changed={}, removed={}, added={}）\n"
    "  - N = 参与主体数 + 1 = 3（理由：每个主体至少一次机会产生变更但未产生）\n\n"
    "  - 空 diff 意味着：所有 stance key 的值都没变，没有新 key 也没有消失的 key\n"
    "  - 连续 3 轮空 diff 意味着：经历完整 Gemini->Codex->Gemini 交替后立场不变\n\n"
    "但这就是编排者最初拒绝的方案——连续两轮 conceded/reasons/unresolved 不变。\n"
    "区别在于：编排者拒绝的是 conceded 列表的字符串比较。\n"
    "现在的方案是 StanceDiff 的结构化比较——这是严格的代数对象等价判断，不是字符串匹配。\n\n"
    "不动点条件下的 residue：\n"
    "  - 收敛时的 stance 就是最终立场\n"
    "  - 从整个 stance 序列中通过 derive_concession_trace() 推导让步轨迹\n"
    "  - concession 来自差分序列的累积——哪些 stance key 在整个对话中消失了或改变了\n"
    "  - unresolved = 最终 stance 中 verdict=fail 或 conditional 的议题\n"
    "  - residue 不需要环路来定义——residue = 让步轨迹 + 未解决议题\n\n"
    "这个方案的优势：\n"
    "  1. 不需要构造图——直接使用已有的 StanceDiff 基础设施\n"
    "  2. 不需要因果边——不存在因果推导问题\n"
    "  3. 不需要滑动窗口——差分序列是完整历史\n"
    "  4. 不需要 simple_cycles——没有复杂度问题\n"
    "  5. 收敛条件是代数不动点——数学上严格\n"
    "  6. lack 的登记来自让步轨迹 + 未解决议题——不依赖图拓扑\n\n"
    "这个方案的问题：\n"
    "  - 编排者的直觉是对的——拓扑判据比字符串匹配更强\n"
    "  - 但你已证明在有向图上做正确的拓扑判据面临二难困境\n"
    "  - StanceDiff 不动点是否退化为编排者最初拒绝的东西？\n"
    "  - 如果不退化，它多了什么？\n\n"
    "请严格质询。我特别想知道：\n"
    "1. StanceDiff 不动点方案是否数学上严格\n"
    "2. 它是否退化为编排者最初拒绝的字符串比较\n"
    "3. 如果你接受此方案，你在四轮中坚持的哪些立场发生了变化？\n"
    "4. 如果你拒绝此方案，替代方案是什么？"
)

CONTEXT = (
    "三轮质询历史：\n"
    "Round 1: SCC 替换 WCC 数学错误 -> fail, 0 concessions\n"
    "Round 2: 无向指标噪声 + diameter活锁 + 累积图不消失 + 因果缺失 -> fail, 0 concessions\n"
    "Round 3: 死锁vs滑动窗口互斥 + 因果不可推导 + 滑动窗口=遗忘=允许话题转移 -> fail, 0 concessions\n\n"
    "被废弃的方案：\n"
    "- beta_1 = E - V + C_SCC\n"
    "- 两层指标（无向 + 有向）\n"
    "- diameter + 1 stability_window\n"
    "- 累积图\n"
    "- 滑动窗口\n"
    "- 因果边从 StanceDiff 推导\n\n"
    "坚持的原则：\n"
    "- 否定方向不可抹掉\n"
    "- 收敛不是消除矛盾而是显式登记\n"
    "- lack 不能被无声消解\n"
    "- 无轮数上限\n\n"
    "已有基础设施（不需要新建）：\n"
    "- StanceDeclaration, StanceDiff, ConcessionTrace (consensus_trigger.py)\n"
    "- compute_stance_diff() (consensus_trigger.py)\n"
    "- derive_concession_trace() (consensus_trigger.py)\n"
    "- parse_stance_declaration() (stance_parser.py)\n"
    "- InquiryCycleResult (consensus_trigger.py)\n"
    "- trigger_ceremony() -> write_consensus_ceremony() (consensus_trigger.py)"
)

challenger = GeminiChallenger()
result = challenger.verify(SUBJECT, CONTEXT)
print(result.response)
print("\n---MODEL---")
print(result.model)
