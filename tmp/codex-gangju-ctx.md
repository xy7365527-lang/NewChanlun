# 纲举目张代码层审查上下文

## 审查目标

验证总方针§目的实现连通性——从 Gemini/Codex 回复文本到 consensus/residue/tension 三区块写入磁盘，管道是否完整且可执行。

---

## 系统架构概述

### 已知状态

| 层 | 实例数 |
|---|---|
| Event layer（event 类型区块）| 182个 |
| Relation layer（relations.jsonl）| 958条 |
| Reflexive layer（rewrite 区块）| 0个 |
| Consensus 区块 | 0个 |
| Residue 区块 | 0个 |
| Tension 区块 | 0个 |

系统当前：质询代码写了，但从未在生产环境执行。

---

## 关键文件摘录

### 1. scripts/block_topology.py（核心 API）

```python
BLOCK_TYPES = frozenset({
    "event", "consensus", "residue", "tension", "rewrite",
})

SOURCES = frozenset({
    "cc", "gemini", "codex", "migration", "serena",
})

RELATION_TYPES = frozenset({
    "depends_on", "negates", "related", "tensions_with",
    "supersedes", "residue_of", "reopens",
    "freezes", "splits", "severs",
    "records",
    "negated_by",
})

def write_block_with_relations(block_type, source, content, refs=None,
                                relations=None, git_ref="", base=DEFAULT_BASE) -> dict:
    """Atomic write: block + relations + rewrite (if order 1 relations exist).
    Strategy: prepare all files in a temp directory, then move to official location.
    Returns the primary block dict.
    """

def read_all_relations(base=DEFAULT_BASE) -> list[dict]:
    """Read all relations from JSONL."""

def query_relations(base=DEFAULT_BASE, block_id=None, relation=None, order=None) -> list[dict]:
    """Query relations with optional filters."""

def list_blocks(base=DEFAULT_BASE, block_type=None) -> list[dict]:
    """List all blocks, optionally filtered by type."""
```

### 2. scripts/consensus_trigger.py（立场差分架构）

```python
@dataclass(frozen=True, slots=True)
class StanceDeclaration:
    verdict: Literal["pass", "fail", "conditional"]
    stances: dict[str, str]  # 具体点 → 持有的立场
    round_number: int = 0
    self_reported_concessions: list[str] = field(default_factory=list)

@dataclass(frozen=True, slots=True)
class InquiryCycleResult:
    scenario: Literal["gemini_verify", "plan_review"]
    trigger_block_id: str
    conclusion: str
    gemini_conceded: list[str]
    codex_conceded: list[str]
    concession_reasons: dict[str, str]
    unresolved: list[str]
    source: str = "cc"
    concession_trace: ConcessionTrace | None = None
    stance_diffs: list[StanceDiff] = field(default_factory=list)

def extract_from_gemini_verify(stance_sequence, trigger_block_id, negation_stands,
                                conclusion, unresolved=None) -> InquiryCycleResult:
    """从 Gemini verify 质询的立场序列中推导共识仪式所需数据。"""

def extract_from_plan_review(stance_sequence, trigger_block_id, conclusion,
                              unresolved=None) -> InquiryCycleResult:
    """从 plan-review 多轮对审的立场序列中推导共识仪式所需数据。"""

def trigger_ceremony(cycle_result: InquiryCycleResult, base=DEFAULT_BASE) -> dict[str, dict]:
    """从质询循环结果触发共识仪式三区块写入。
    §21: 三区块原子性地同时写入。
    Returns {"consensus": ..., "residue": ..., "tension": ...}
    """
    return write_consensus_ceremony(
        trigger_block_id=cycle_result.trigger_block_id,
        conclusion=cycle_result.conclusion,
        gemini_conceded=cycle_result.gemini_conceded,
        codex_conceded=cycle_result.codex_conceded,
        concession_reasons=cycle_result.concession_reasons,
        unresolved=cycle_result.unresolved,
        source=cycle_result.source,
        base=base,
    )

def detect_convergence_from_review_file(review_file: Path) -> bool:
    """检测 review-results 文件是否表明质询循环已收敛。
    收敛信号：plan-review 的"确认满意"/"共识达成"/"方案定稿"/"[plan-reviewed]"
    或 gemini verify 的 YAML frontmatter result: pass/fail/escalate
    """

def find_trigger_block_for_review(review_file: Path, base=DEFAULT_BASE) -> str | None:
    """从 review-results 文件中提取关联的 trigger block ID。
    从 YAML frontmatter 的 trigger: 或 target: 字段提取。
    """
```

### 3. scripts/consensus_ceremony.py（三区块原子写入）

```python
def write_consensus_ceremony(trigger_block_id, conclusion, gemini_conceded,
                               codex_conceded, concession_reasons, unresolved,
                               source="cc", base=DEFAULT_BASE) -> dict[str, dict]:
    """Atomic write of the three-block consensus ceremony.
    Produces 5 blocks total (3 primary + 2 rewrite) and 4 relations
    (2 order-1 + 2 order-2).
    Returns dict with keys "consensus", "residue", "tension".

    具体产出：
    - consensus block: refs=[trigger_block_id], content.conclusion
    - residue block: refs=[consensus.id], content.{gemini_conceded, codex_conceded, reasons}
    - tension block: refs=[consensus.id], content.unresolved
    - rewrite_residue: refs=[residue.id], records residue→consensus 关系
    - rewrite_tension: refs=[tension.id], records tension→consensus 关系
    - 关系：residue_of(order=1), tensions_with(order=1), records(order=2)×2
    """
```

### 4. scripts/stance_parser.py（输出协议注入 + 解析）

```python
def parse_stance_declaration(text: str, round_number: int = 0) -> StanceDeclaration | None:
    """从回复文本中提取结构化立场声明。
    解析 ---stance-declaration--- ... ---end-stance--- YAML 块。
    如果文本中不包含立场声明块，返回 None。
    """

def parse_stance_sequence(texts: Sequence[str], start_round: int = 1) -> list[StanceDeclaration]:
    """从多轮回复文本中依次提取立场声明序列。"""

STANCE_OUTPUT_PROTOCOL_GEMINI = "..."  # 注入 Gemini prompt 的输出协议文本
STANCE_OUTPUT_PROTOCOL_CODEX = "..."   # 注入 Codex prompt 的输出协议文本
```

### 5. scripts/ceremony_scan.py（ceremony 扫描器）

```python
def get_frozen_nodes(root):
    """从 block-topology 读取 frozen 节点集合（178号-2 迁移）。
    直接读 .chanlun/block-topology/relations.jsonl"""

def detect_genealogy_anomalies(root):
    """检测谱系编号异常：重复编号、文件名编号与内部 id 不一致。
    包含 block-topology completeness check：每个 settled 文件应有 meta.json 对应映射"""
    # 注意：dag.yaml 完整性检查标注在注释里，但函数体只检查 settled 文件
    # 不再读旧 dag.yaml
```

---

## 诊断问题

请对以下四个问题进行代码层审查：

### Q1：trigger_ceremony() 调用链完整性

从 Gemini/Codex 回复文本到 consensus/residue/tension 三区块写入磁盘的完整调用链是：

```
Gemini/Codex 回复文本
  → stance_parser.parse_stance_declaration()  [提取立场声明]
  → stance_parser.parse_stance_sequence()     [多轮序列]
  → consensus_trigger.extract_from_*()        [推导 InquiryCycleResult]
  → consensus_trigger.trigger_ceremony()      [触发仪式]
  → consensus_ceremony.write_consensus_ceremony()  [原子写入磁盘]
```

问题：这个调用链在代码层是否自洽？是否有断点？
已知问题：`trigger_ceremony()` 从未被任何生产代码调用——没有任何脚本、hook、或 CLI 入口触发它。

### Q2：ceremony_scan 从 dag.yaml 迁移到 block-topology

ceremony_scan.py 中：
- `get_frozen_nodes()` 已经迁移到读 `.chanlun/block-topology/relations.jsonl`（第55-141行）
- `detect_genealogy_anomalies()` 已增加 block-topology completeness check（第471-494行），但仍保留旧的 frontmatter 和编号一致性检查
- 旧的 dag.yaml 完整性检查已不存在于代码中

问题：ceremony_scan 是否还依赖 dag.yaml？如果迁移已完成，是否有遗漏？

### Q3：共识仪式第一次执行的最短路径

已知前置条件：
1. 需要一个真实的 InquiryCycleResult（来自真实质询循环）
2. 需要一个有效的 trigger_block_id（182个现有 event 区块中的某个）
3. Gemini/Codex CLI 已就绪（API key 已配置）
4. stance_parser 的输出协议已注入（但尚未在生产 prompt 中使用）

最短路径是：手动构建一个 InquiryCycleResult，调用 trigger_ceremony()？
还是：先让一个真实质询循环跑通，再触发仪式？

技术障碍在哪里？

### Q4：Reflexive layer 实现方案

当前 Reflexive layer 为 0 rewrite 区块。

write_block_with_relations() 在检测到 order=1 关系时，会自动生成 rewrite 区块（第197-219行）。
write_consensus_ceremony() 内部也生成了 2 个 rewrite 区块（rewrite_residue + rewrite_tension）。

Reflexive layer 的激活条件是：有任何 order=1 关系被写入。
当前 958 条关系全部是迁移时写入的 depends_on，没有 order 字段（或 order=1？）。

问题：如果现有 958 条关系没有 order 字段，query_relations(order=1) 会返回什么？
relations.jsonl 中的旧关系是否包含 order 字段？
