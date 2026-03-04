#!/usr/bin/env python
"""Gemini challenge v2: 用 gemini-3-pro-preview thinking 模式重跑四议题。

gemini-3.1-pro-preview 当前 503（高需求），
gemini-3-pro-preview 可用且支持 thinking_budget=8192。
"""
import sys, os
sys.path.insert(0, os.getcwd())
sys.path.insert(0, os.path.join(os.getcwd(), "src"))

from pathlib import Path
from newchan.gemini.modes import GeminiChallenger

CONTEXT = Path("tmp/phase2-audit-discussion-ctx.md").read_text(encoding="utf-8")

SUBJECTS = {
    "inherits": """议题1：inherits 关系类型引入

编排者声称 613 条 missing_dependency 是"概念沉降图"，提议引入第三种概念连接类型 inherits（基底性依赖）。

质询点：
1. 这是否是对 missing_dependency 的过度解读？613 条中有多少可能只是松散词汇复用而非真正的概念继承？概念词匹配的精度能接受吗？
2. inherits 不纳入 VALIDITY_CAPABLE_RELATIONS 的理由是"概念沉降不可逆"。反例：如果被继承的概念本身被否定了呢？继承链上的所有节点是否应该被标记为"基底动摇"？
3. inherits 应不应该纳入 TOPOLOGICAL_RELATIONS（影响图不变量计算）？如果纳入，cycle_rank 预期增量多少？如果不纳入，图不变量是否遗漏了最大的关系盲区？
4. inherits 边在 Phase 3 的 directed flag complex 中如何处理？与 depends_on/references 环路性质不同——inherits 是单向沉降，不产生传递三角形。""",

    "duplicates": """议题2：71 duplicates 的分析

三轮精度校准后 duplicates 从 110 降到 71：
- 第1轮：赋值过滤器（过滤日期/路径/代码/URL） → 97
- 第2轮：frontmatter 元数据字段过滤 → 71
- 剩余 71 个全是真实概念复用：推论(8)、四分法分类(8)、结论(7)、模式识别(6)

质询点：
1. "推论"、"结论"等一般性词汇应该过滤还是保留？过滤=丢失概念演化信号，保留=duplicates 永远不为零。你怎么判断？
2. potential_birth/confirmed_duplicate 的 2/3 定义数阈值有没有理论依据？为什么不是 2/4 或 3/5？
3. 26 个 confirmed_duplicate 的威胁等级——它们真的是概念冲突（同一概念的不兼容定义）还是概念在不同语境下的合法扩展？需要什么级别的验证？""",

    "cycle_rank": """议题3：cycle_rank=1404 与 Phase 3

enrichment 前 cycle_rank=654，enrichment 后 1404（+115%）。三轮校准后仍为 1404——因为过滤影响的是 defines（不在 TOPOLOGICAL_RELATIONS 中），SCC 结构由 references/depends_on/negates 等拓扑边决定。

质询点：
1. cycle_rank 从 654 到 1404 的跳变意味着什么？references 边创造了大量新 SCC 环路——这些环路是真实的循环依赖还是引用关系的自然结构？
2. Phase 3 的 β₁^Δ 是否需要区分边类型构建不同权重的单纯形？depends_on（结构性）vs references（指示性）vs inherits（基底性）在 flag complex 中应该有不同的贡献吗？
3. 如果 inherits 加入 TOPOLOGICAL_RELATIONS，cycle_rank 预期从 1404 增加到多少？613 条新边中有多少会参与已有 SCC？""",

    "domain": """议题4：domain 规范草案

Phase 3 折叠/稳定态检测需要 domain 字段。当前 relations.jsonl 中 domain 使用量为 0。

候选 domain 枚举：
- chanlun_single — 缠论单标的分析
- chanlun_multi — 四矩阵资本流转
- architecture — 系统架构/方法论
- metatheory — 谱系学/元理论
- consensus — 共识仪式/诊断

质询点：
1. 5 个候选 domain 的边界够清晰吗？architecture 和 metatheory 会不会重叠？例如"蜂群的并行原则"是 architecture 还是 metatheory？
2. "概念密度 + 社区检测"推断旧谱系的 domain 可靠吗？社区检测算法（Louvain/Label Propagation）在有向图上的行为与无向图不同，预期产生多少个社区？
3. domain 字段应该是区块属性（每个 genealogy block 一个 domain）还是关系属性（每条 edge 一个 domain）？折叠的定义是"同一对象在不同域中的范畴身份"——如果 domain 在 block 上，如何检测跨域关系？"""
}

def main():
    # 直接指定 gemini-3-pro-preview（3.1 当前 503）
    gc = GeminiChallenger(model="gemini-3-pro-preview")
    results = {}
    models_used = {}
    for key, subject in SUBJECTS.items():
        print(f"[{key}] Calling Gemini challenge (gemini-3-pro-preview thinking)...", flush=True)
        try:
            r = gc.challenge(subject=subject, context=CONTEXT)
            results[key] = r.response
            models_used[key] = r.model
            print(f"[{key}] Done. Model: {r.model}, len={len(r.response)}")
        except Exception as e:
            results[key] = f"ERROR: {e}"
            models_used[key] = "error"
            print(f"[{key}] Failed: {e}")

    # Write combined report
    out = Path("tmp/gemini-phase2-audit-challenge-v2.md")
    lines = ["# Gemini Phase 2 审计质询报告（gemini-3-pro-preview thinking）\n\n"]
    for key, resp in results.items():
        model = models_used.get(key, "unknown")
        lines.append(f"## 议题：{key}（model: {model}）\n\n{resp}\n\n---\n\n")
    out.write_text("".join(lines), encoding="utf-8")
    print(f"\nReport written to {out}")

if __name__ == "__main__":
    main()
