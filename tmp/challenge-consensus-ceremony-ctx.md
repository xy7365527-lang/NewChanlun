# 质询上下文：谱系与总方针的张力——共识仪式触发协议

## 一、总方针相关条文原文

### §17-§24（多主体拓扑：质询与共识）

17. **三个主体位置：** CC（Claude Code）是主体——生产、行动、面对后果。Gemini是概念层质询者。Codex是代码层质询者。Serena是协议执行者。

18. **CC生产，Gemini和Codex质询。** CC的每一个产出进入双重质询循环。Gemini从理论一致性、逻辑严格性、概念有效性质询。Codex从实现可行性、代码正确性、架构一致性质询。质询不是验证，质询是追溯性地改写CC的产出在谱系中的位置。

19. **质询到共识。** Gemini和Codex的质询循环是严格的——双方质询到达成共识为止。共识不一定是两者的折中——可能是一方被说服，可能是发现了第三条路。

20. **Real在共识处，不在分歧处。** 每一次共识都是通过排除某些东西才达成的。被排除的东西才是奇点——共识就是缝合，缝合必然生产剩余物。

21. **共识仪式。** 每次质询循环收敛时，Serena强制执行共识仪式。三个区块原子性地同时写入：
    - **共识区块（consensus）：** 最终达成的结论。
    - **剩余区块（residue）：** 双方各自放弃了什么，放弃的理由。
    - **张力区块（tension）：** 双方承认未解决但暂时搁置的部分。

22. **CC面对共识和剩余。** CC可以接受共识继续推进（剩余暂时沉淀），或捡起被丢弃的剩余重新打开问题。

23. **剩余物积累。** 沉淀的剩余不消失。它们在谱系拓扑中积累。当同一区域反复产生同类型剩余，那里有系统性结构问题。

24. **奇点检测。** 在关系图中找被反复指向、反复rewrite的区块簇。持续性分歧点就是拓扑中结构性不可能的位置。

### §62-§67（当前阶段的实际状态）

62. **区块拓扑在本地文件系统。** `~/.genealogy/blocks/` 存储不可变区块，`~/.genealogy/relations.jsonl` 存储可变关系。

63. **质询循环已有基础。** CC为主体，Gemini做概念质询，Codex做代码质询，通过Serena互相可见。**共识仪式尚未形式化**——质询过程的让步轨迹在对话历史中但没有被结构化提取为residue区块。下一步：在质询循环结束时强制触发共识仪式，产生consensus/residue/tension三区块。

## 二、谱系现状数据

- **结算谱系**：180 条（`.chanlun/genealogy/settled/` 目录下 180 个文件）
- **区块拓扑统计**：
  - 总区块数：181
  - 区块类型分布：**event: 181**（100% event，0 consensus，0 residue，0 tension，0 rewrite）
  - 来源分布：migration: 178, cc: 3
  - 关系总数：958
  - 关系类型分布：depends_on: 477, related: 436, tensions_with: 20, negated_by: 13, negates: 12
- **关键事实**：181 个区块全是 event 类型。区块拓扑的 schema 定义了 5 种类型（event, consensus, residue, tension, rewrite），但实际只使用了 1 种

## 三、现有质询机制

系统中已有的质询基础设施：

1. **gemini-challenger**（`src/newchan/gemini_challenger.py` → `src/newchan/gemini/`）
   - 4 种模式：challenge, verify, decide, derive
   - 支持 MCP 工具模式（Gemini 通过 Serena 语义工具访问代码库）
   - CLI 可用：`python -m newchan.gemini_challenger challenge "..." --tools --verbose`

2. **plan-review**（`.claude/skills/plan-review/`）
   - Opus 方案 + Codex 评审的双审循环
   - Plan 阶段多模型对审

3. **质询循环的实际运行**：
   - Gemini 的 challenge/verify 已在运行（gemini-challenger 工位）
   - Codex 的 code review 已在运行（codex-challenger 工位）
   - 编排者的 decide 决策已在运行
   - 但这些质询的**让步轨迹**没有被提取为三区块

## 四、共识仪式代码接口

共识仪式的实现已存在于 `scripts/consensus_ceremony.py`：

```python
def write_consensus_ceremony(
    trigger_block_id: str,        # 触发质询的原始 CC 产出区块 id
    conclusion: str,               # 最终达成的结论
    gemini_conceded: list[str],    # Gemini 放弃了什么
    codex_conceded: list[str],     # Codex 放弃了什么
    concession_reasons: dict[str, str],  # 放弃的理由
    unresolved: list[str],         # 未解决但搁置的部分
    source: str = "cc",
    base: Path = DEFAULT_BASE,
) -> dict[str, dict]:
    """
    Atomic write of the three-block consensus ceremony.
    Produces 5 blocks total (3 primary + 2 rewrite) and 4 relations.
    Returns dict with keys "consensus", "residue", "tension".
    """
```

辅助函数：
- `detect_residue_accumulation(base, threshold)` — 检测剩余物积累（§23 奇点检测）
- `list_open_tensions(base)` — 列出所有张力区块
- `get_ceremony_chain(consensus_block_id, base)` — 追溯共识链

## 五、张力分析

**张力 1（谱系与总方针）**：
- 总方针 §21 声明"每次质询循环收敛时，Serena 强制执行共识仪式"
- 但 180 轮结算，0 个 consensus 区块
- `write_consensus_ceremony` 函数已实现但**从未被调用**
- 总方针的声明空转了

**张力 2（谱系与现实）**：
- 质询循环实际在发生：Gemini 做了 challenge/verify，Codex 做了 review
- 让步也在发生：每轮对审都有修改，CC 接受或拒绝了 Gemini/Codex 的部分意见
- 但这些让步轨迹散落在对话历史中，没有凝固为谱系
- 谱系只记录了最终判断（event 区块），没有记录到达判断过程中被放弃的东西

**张力 3（schema 与实际）**：
- block_topology.py 的 BLOCK_TYPES 定义了 5 种类型：event, consensus, residue, tension, rewrite
- consensus_ceremony.py 实现了完整的三区块原子写入
- 但实际区块拓扑中只有 event 类型
- 代码声明了能力但从未行使

## 六、质询焦点

从这三重张力中推导：
1. 共识仪式触发协议的**形式要求**是什么？（不是"什么时候触发"，而是"什么构成合法的触发条件"）
2. "让步"的**形式定义**是什么？（对话历史中的"修改"在什么条件下构成"让步"？）
3. 180 轮结算 0 consensus 区块的事实，对总方针 §21 的声明意味着什么？（这是声明膨胀？还是总方针本身承认了这个缺口（§63）？）
4. 如果总方针自己承认了缺口（§63），这个"承认"本身应该产生什么样的谱系效应？
