# Codex 代码审查上下文：否定/扬弃关系在内容级拓扑中的覆盖缺口

## 审查任务

审查模式：review
审查焦点：谱系系统对否定（negation）和扬弃（Aufhebung）关系的代码层支持程度——
当前 content_enrichment_migration.py + concept_extractor.py 组合是否正确提取并生成否定类关系。

---

## 领域上下文：什么是谱系中的否定/扬弃

谱系（Genealogy）不是普通知识图谱。每个谱系条目（settled/*.md）记录一次概念发现事件，
并且事件之间存在否定动态：

1. **否定（negates）**：谱系节点 A 否定谱系节点 B，B 的效力终止
2. **扬弃（Aufhebung）**：节点 A 否定+保留+提升节点 B（089号谱系的定义）
3. **修正（modifies）**：节点 A 修正 B 的某方面

这些动态在谱系 Markdown 文件中有两种表达位置：

### 位置1：YAML frontmatter
```yaml
negates: ["073"]       # 本谱系节点否定了073号
negated_by: ["073a"]   # 本谱系节点被073a号否定
negation_source: "Gemini challenge 审计"
negation_form: "aufhebung"
```

真实例子（073a-control-data-flow-decoupling.md frontmatter）：
```yaml
negates: ["073"]
negated_by: []
```

真实例子（062-heterogeneity-as-sinthome.md frontmatter）：
```yaml
negated_by: ["064", "093"]  # 064号部分否定 + 093号扬弃
```

### 位置2：正文中的否定声明段落

多种段落标题格式被实际使用：
- `## 否定 073 的理由`（073a文件正文）
- `## 5. 完整否定记录`（254号正文——含14条否定记录的表格）
- `## 否定了什么`（020a文件正文）
- `### 否定1：...` / `### 否定2：...`（067文件正文）
- 正文中的 `否定记录` 表格（264号）
- `negation_source:` 和 `negation_form:` 作为正文字段（如089号文件）

---

## 被审查的代码文件

### 文件1：scripts/concept_extractor.py

关键数据结构（没有 Negation 类型）：
```python
@dataclass(frozen=True)
class Modification:
    target_id: str      # 被修正的谱系 id
    target_desc: str
    modification: str

@dataclass(frozen=True)
class ContentAnalysis:
    genealogy_id: str
    sections: tuple[Section, ...]
    concepts: tuple[ConceptDefinition, ...]
    references: tuple[str, ...]
    new_concepts: tuple[NewConcept, ...]
    modifications: tuple[Modification, ...]   # 只有 modifications，没有 negations
    conclusion_summary: str
```

提取函数 `_extract_modifications`：
```python
def _extract_modifications(body: str) -> tuple[Modification, ...]:
    """Extract modifications from ### 对现有概念的修正 section."""
    section_text = _find_section_by_keywords(body, [
        "对现有概念的修正",
        "对现有概念的定位修正",
    ])
    # 只处理这两个 section heading，其他位置完全不处理
    ...
```

没有任何函数处理：
- frontmatter 中的 `negates:` 字段
- frontmatter 中的 `negated_by:` 字段
- 正文中的否定段落（"## 否定 073 的理由" 等）
- 正文中的否定记录表格

### 文件2：scripts/content_enrichment_migration.py

生成的关系类型（第159-237行）：
```python
# records 关系：rewrite → original
# defines 关系：original → concept_id（每个概念）
# modifies 关系：original → target_block（每个 Modification）
# references 关系：original → referenced_block（每个 NNN号引用）
```

完全不生成：
- `negates` 关系
- `negated_by` 关系
- `supersedes` 关系

### 文件3：scripts/block_topology.py

RELATION_TYPES 已声明（第37-46行）：
```python
RELATION_TYPES = frozenset({
    "depends_on", "negates", "related", "tensions_with",
    "supersedes", "residue_of", "reopens",
    "freezes", "splits", "severs",
    "records",
    "negated_by",
    "defines",
    "modifies",
    "references",
})
```

`negates` 关系有完整的 schema 支持（273号谱系实现）：
```python
def make_relation(from_id, to_id, relation, order, created_by, **extra):
    # 273号: negates 边默认 validity="active"
    if relation == "negates":
        rec["validity"] = extra.pop("validity", "active")
        rec["invalidated_by"] = extra.pop("invalidated_by", None)
        rec["invalidated_at"] = extra.pop("invalidated_at", None)
```

schema 支持完整，但没有任何代码调用 make_relation(..., relation="negates", ...)
来自 content enrichment 路径。

### 文件4：scripts/concept_topology_check.py

三个检测函数：
1. `detect_duplicate_concepts()` — 检查 defines 关系的冲突
2. `detect_concept_conflicts()` — 检查 modifies 的 stale reference
3. `detect_reference_dependency_mismatch()` — 检查 references vs depends_on

完全没有：
- 检查 negates 关系的一致性（active negates 边指向的节点还被其他块 defines？）
- 检查 negated_by 是否有对应的 negates 反向边
- 检查扬弃链的完整性（089号 aufhebung 在拓扑中的表达）

---

## 实际谱系否定数据样本

以下是 settled/*.md 中实际存在的 negates 关系（frontmatter 层）：

| 否定者 | 被否定者 | 否定形式 |
|--------|---------|---------|
| 073a | 073 | 否定平台约束前提 |
| 064 | 062 | 部分否定（NegationObject定义修正） |
| 068 | 065, 066 | 否定连续流形假设 |
| 087 | 086 | 否定保守全C方案 |
| 088 | 032 | 否定 divine-madness 自我限制 |
| 095 | 073b | 否定 Trampoline 近似 |
| 173 | 088 | 否定权限死锁重设计方案 |

这些否定关系当前**完全不进入 block-topology 的 relations.jsonl**——
因为 content_enrichment_migration.py 只写 defines/modifies/references/records。

---

## 审查要求

请对以下三个层面进行代码审查：

### 层面1：提取层缺口（concept_extractor.py）

1. `ContentAnalysis` 数据类是否应该增加 `negations` 字段？
2. 应该从哪些位置提取否定关系？
   - frontmatter `negates:` 列表？
   - frontmatter `negated_by:` 列表？
   - 正文中的否定段落？
   - 正文中的否定记录表格？
3. 提取 frontmatter 时，`_strip_frontmatter` 会剥离 frontmatter——
   但 frontmatter 中的 negates/negated_by 字段在剥离前是否应该被解析？

### 层面2：迁移层缺口（content_enrichment_migration.py）

1. `enrich_single_file` 是否应该在生成 modifies 关系的同时生成 negates 关系？
2. negates 关系的 `order` 应该是 1 还是 2？
   （block_topology.py 的规则：order 1 触发 rewrite，order 2 只记录）
3. negated_by 关系是否应该生成，或者由查询时通过 negates 反向推导？

### 层面3：拓扑检测缺口（concept_topology_check.py）

1. 是否应该新增 `detect_negation_consistency()` 函数？
   - 检查：negates(A→B) 存在时，negated_by(B→A) 是否也存在（反向一致性）
   - 检查：active negates 边指向的节点 B，如果 B 仍然被其他新块 defines，是否触发警告
2. `detect_concept_conflicts` 当前只处理 modifies 的 stale reference——
   negates 导致的 stale reference（被否定节点还被引用）是否应该也检测？

---

## 边界条件提示

- frontmatter 解析需要在 `_strip_frontmatter` 之前执行（frontmatter 内容被剥离后不可访问）
- negates 字段在 frontmatter 中是列表格式（YAML），需要解析
- 部分文件的 negates 字段包含注释（如 `["073a号（depth_budget 基因废除）"]`——字符串而非纯 id）
- 273号谱系已定义 negates 边的 validity 语义，迁移时需要正确设置 validity="active"
