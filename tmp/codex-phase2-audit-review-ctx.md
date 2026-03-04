# Phase 2 Audit Code Review Context

## 任务说明

对三个脚本进行代码层审查（review 模式）。这些脚本是谱系知识图谱的内容提取和拓扑检测核心。
数据规模：307 个谱系区块，1140 个概念，2592 条关系，active_cycle_rank=1404。

## 审查维度定义

- **F (Fatal)**: 会破坏数据完整性或产出错误结果
- **I (Important)**: 设计缺陷或精度问题
- **S (Suggestion)**: 可改进但不阻塞

## 重点议题（来自 Phase 2 审计讨论）

1. inherits 关系实现路径：从 missing_dependency 检测到 inherits 写入的完整链路
2. 71 duplicates 中 potential_birth 的分类准确性（代码中 2 个 defines = potential_birth，≥3 = confirmed_duplicate）
3. cycle_rank=1404 不变根因：defines 过滤不影响 SCC 结构的原因

---

## 文件1：scripts/concept_extractor.py

### 关键常量

```python
_FRONTMATTER_METADATA_FIELDS = frozenset({
    "状态", "类型", "前置", "关联", "域", "溯源", "来源",
    "结算方式", "结算依据", "结算时间", "创建时间", "已执行",
    "status", "type",
    "negation_source", "negation_form", "negation_model",
})

_VALUE_ASSIGNMENT_RE = re.compile(
    r'^(?:'
    r'\d{4}[-/]\d{1,2}[-/]\d{1,2}'
    r'|`[^`]+`'
    r'|\[.*?\]\(.*?\)'
    r'|(?:true|false|null|none|yes|no)$'
    r'|\d{3}[a-z]?(?:\s*[、,]\s*\d{3}[a-z]?)*$'
    r'|[\w./\\-]+\.(?:py|md|yaml|json|sh|txt)'
    r'|https?://'
    r')',
    re.IGNORECASE
)
```

### _extract_concepts() 三层过滤（第 262-272 行）

```python
def _extract_concepts(body: str) -> tuple[ConceptDefinition, ...]:
    for i, line in enumerate(body.split('\n'), 1):
        match = _CONCEPT_DEF_RE.search(line)
        if match:
            term = match.group(1).strip()
            definition = match.group(2).strip()
            # 层1：frontmatter 元数据字段
            if term in _FRONTMATTER_METADATA_FIELDS:
                continue
            # 层2：NNN号格式的 term
            if re.match(r'^\d{3}[a-z]?号$', term):
                continue
            # 层3：值型赋值
            if _VALUE_ASSIGNMENT_RE.match(definition):
                continue
            # 其他过滤
            if '---' in definition or '|' in definition[:5]:
                continue
            # 去重
            if term not in seen_terms:
                seen_terms.add(term)
                concepts.append(ConceptDefinition(...))
```

### NNN号引用提取（第 283-288 行）

```python
_GENEALOGY_REF_RE = re.compile(r'(\d{3}[a-z]?)号')

def _extract_references(body: str) -> tuple[str, ...]:
    refs = set()
    for match in _GENEALOGY_REF_RE.finditer(body):
        refs.add(match.group(1))
    return tuple(sorted(refs))
```

---

## 文件2：scripts/concept_topology_check.py

### potential_birth / confirmed_duplicate 分类（第 150-151 行）

```python
# Folding status: potential_birth when 2nd defines appears
folding_status = "potential_birth" if len(defs) == 2 else "confirmed_duplicate"
```

注：分类基于 `len(defs)`（defines 关系数量），而不是 `definition_count`（唯一定义文本数量）。

### _should_check_conflict 逻辑（第 40-56 行）

```python
def _should_check_conflict(rel: dict) -> bool:
    rtype = rel.get("relation")
    if rtype == "revises":
        return True
    if rtype == "modifies":
        return True  # old data = unknown → conservative inclusion
    if rtype == "refines":
        kind = rel.get("modification_kind", "unknown")
        return kind == "unknown"
    return False
```

### TOPOLOGICAL_RELATIONS 集合（第 456-460 行）

```python
TOPOLOGICAL_RELATIONS = frozenset({
    "depends_on", "negates", "related", "tensions_with", "supersedes",
    "residue_of", "reopens", "freezes", "splits", "severs",
    "negated_by", "modifies", "refines", "revises", "references",
})
```

注：`defines` 和 `records` 不在此集合中。`annotates` 不在此集合中。

### cycle_rank 计算（第 617-634 行）

```python
for scc in sccs:
    if len(scc) < 2:
        # Check self-loop
        for n in scc:
            for dst in fwd.get(n, ()):
                if dst == n:
                    cycle_rank += 1
        continue
    e_count = 0
    for n in scc:
        for dst in fwd.get(n, ()):
            if dst in scc:
                e_count += 1
    v_count = len(scc)
    cycle_rank += e_count - v_count + 1
```

### detect_duplicate_concepts 中 modifies 连通组件逻辑（第 98-143 行）

```python
modifies_adj: dict[str, set[str]] = defaultdict(set)
for rel in relations:
    if _is_evolution_relation(rel):  # 匹配 modifies/refines/revises
        modifies_adj[rel.get("from", "")].add(rel.get("to", ""))
        modifies_adj[rel.get("to", "")].add(rel.get("from", ""))

# 双向化 → 无向连通
first_comp = _get_component(block_ids[0])
all_connected_by_modifies = all(bid in first_comp for bid in block_ids[1:])
```

---

## 文件3：scripts/content_enrichment_migration.py

### _build_existing_keys 的 defensive check（第 45-55 行）

```python
def _build_existing_keys(base: Path) -> set[tuple]:
    try:
        existing = read_all_relations(base)
    except FileNotFoundError:
        return set()
    keys = set()
    for r in existing:
        if r.get("from") and r.get("to") and r.get("relation"):
            keys.add(_relation_dedup_key(r))
    return keys
```

### 关系去重键（第 40-42 行）

```python
def _relation_dedup_key(rel: dict) -> tuple:
    return (rel["from"], rel["to"], rel["relation"], rel.get("order", 0))
```

### missing_dependency → inherits 的代码路径

当前 missing_dependency 由 `detect_reference_dependency_mismatch()` 检测，返回：

```python
{
    "type": "missing_dependency",
    "severity": "warn",
    "block": block_id,
    "target": m,
    "signal": "potential_depends_on",
}
```

注：`signal` 字段是 `potential_depends_on`，而非 `inherits`。
`inherits` 关系类型**不在** `RELATION_TYPES` 中，也不在任何写入路径上。
`content_enrichment_migration.py` 中没有从 missing_dependency 转换为 inherits 的代码。

### 幂等性机制（第 161-168 行）

```python
def _write_rel(rel: dict) -> None:
    nonlocal relations_written
    if existing_keys is not None:
        key = _relation_dedup_key(rel)
        if key in existing_keys:
            return
        existing_keys.add(key)
    append_relation(rel, base)
    relations_written += 1
```

注意：`existing_keys.add(key)` 在写入之前执行，但在 `append_relation` 之后（如果 append_relation 抛出异常，key 已被加入集合但关系未写入）。

### 合并 defines 逻辑（第 183-190 行）

```python
merged_defines: dict[str, tuple[str, str]] = {}
for nc in ca.new_concepts:
    merged_defines[nc.term] = (nc.term, nc.definition)
for c in ca.concepts:
    existing = merged_defines.get(c.term)
    if existing is None or (not existing[1] and c.definition):
        merged_defines[c.term] = (c.term, c.definition)
```

注：new_concepts 优先，inline concepts 仅在定义为空时覆盖。

---

## 审计背景数据

- `definitions_count` = 1140（三轮校准后）
- `relations_written` = 2592
- `missing_dependency` = 613（正文引用但不在 depends_on）
- `structural_only` = 160（depends_on 但正文无引用）
- `potential_birth` = 45，`confirmed_duplicate` = 26
- `active_cycle_rank` = 1404（defines 过滤前后不变）
- `active_β₀` = 2（主图 316 节点 + 孤立岛 3 节点）

---

## 审查问题清单

请逐一分析以下问题：

### Q1（concept_extractor.py）
`_FRONTMATTER_METADATA_FIELDS` 是否遗漏了常见 frontmatter 字段（如 `negation_model`, `negates`, `negated_by`, `depends_on`, `title`, `date`）？遗漏会导致什么假阳性？

### Q2（concept_extractor.py）
`_VALUE_ASSIGNMENT_RE` 中的 ID list 模式：`\d{3}[a-z]?(?:\s*[、,]\s*\d{3}[a-z]?)*$`
该模式末尾的 `$` 是否足够严格？能否匹配 "041, 089（补充说明）" 这类带后缀的行？

### Q3（concept_extractor.py）
NNN号过滤正则 `r'^\d{3}[a-z]?号$'` 匹配的是 term 字段（粗体内的术语），而不是 definition 字段。这是否是正确的设计意图？如果谱系区块中有 **041号**：（某个定义），这个 term 会被过滤吗？

### Q4（concept_extractor.py）
`_extract_concepts()` 中的三层过滤顺序：先 frontmatter 字段，再 NNN号，再值型赋值。有没有过滤顺序导致的遗漏或误杀？

### Q5（concept_topology_check.py）
`folding_status = "potential_birth" if len(defs) == 2 else "confirmed_duplicate"`
这里 `len(defs)` 是 defines 关系数量，而 `definition_count` 是唯一定义文本数量。当同一术语有 3 个 defines 但 2 个定义文本相同时（2 unique texts），代码会说 "confirmed_duplicate"，而实际上可能只有 2 个真实来源。这个分类阈值是否合理？

### Q6（concept_topology_check.py）
`_should_check_conflict(rel)` 中 `refines` 仅当 `modification_kind == "unknown"` 时检查。
审计数据：11 refines = 3 refine + 8 unknown + 0 revises。8 unknown → 进入冲突检测。
但 3 refine（kind="refine"）被排除。这 3 个已知精化操作如果实际上改变了语义，会产生漏检。是否有保护？

### Q7（concept_topology_check.py）
`TOPOLOGICAL_RELATIONS` 不含 `defines`，因此 defines 边不进入 SCC 计算。
但 `references` 在其中。enrichment 大量写入 references 边 → cycle_rank 上升从 654→1404。
如果引入 `inherits`：
(a) inherits 是否应该加入 TOPOLOGICAL_RELATIONS？
(b) 若加入，对 cycle_rank 的预期影响是什么（inherits 边方向 A→B 与 references 不同吗？）

### Q8（content_enrichment_migration.py）
`_build_existing_keys` 中只捕获 `FileNotFoundError`。如果 relations.jsonl 存在但内容损坏（JSON decode error），会传播未捕获异常导致整个 enrichment 失败。这是否是 defensive check 不够的情况？

### Q9（content_enrichment_migration.py）
enrichment 的幂等性依赖 `existing_keys` 在内存中积累。如果进程中途崩溃（如在第 200 个文件），下次重跑时从头构建 `existing_keys`，已写入的关系会被跳过（正确）。但 `rewrite_block` 会重复创建（`write_block` 是否幂等？）。请检查 block 创建的幂等性。

### Q10（content_enrichment_migration.py）
当前没有从 613 条 missing_dependency 转换为 inherits 关系的代码路径。Phase 2 审计讨论中提到这是议题1的核心。请评估：
(a) 实现 inherits 写入需要在哪个函数中添加什么代码？
(b) `inherits` 需要先加入 block_topology.py 的 RELATION_TYPES 才能通过 make_relation 验证。
(c) inherits 是否应该有 order 字段（order=3 代表推断关系？）

### Q11（整体）
`_write_rel()` 在更新 `existing_keys.add(key)` 后才调用 `append_relation()`。如果 `append_relation` 失败，key 已经被标记为"已写入"——下次重跑时这个关系永远不会被写入。这是 TOCTOU 类型的一致性问题。是否需要修复？
