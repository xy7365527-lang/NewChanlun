---
proposal_id: PROP-001
title: 谱系 DAG 格式标准化
scope:
  - .chanlun/genealogy/settled/
  - .chanlun/genealogy/pending/
genealogy_ref:
  - settled/012-genealogy-is-discovery-engine.md
  - settled/014-distributed-instruction-architecture.md
motivation: |
  当前谱系记录以独立 markdown 文件存在，谱系间的依赖关系（前置/关联/推导链）
  散落在各文件的 frontmatter 中。随着谱系数量增长，需要一种标准化的 DAG
  （有向无环图）格式来显式表达谱系间的拓扑关系，使依赖链可机器解析、可验证。
changes: |
  1. 在 `.chanlun/genealogy/` 下新增 `dag.yaml`，记录所有谱系节点及其边
  2. 每条谱系的 frontmatter 中 `depends_on` 和 `relates_to` 字段格式统一为谱系编号数组
  3. 提供验证脚本检查 DAG 无环性和引用完整性
approval_condition: |
  - genealogist 工位确认格式与现有谱系兼容
  - quality-guard 确认不破坏现有谱系读取流程
status: draft
---

## 详细设计

### dag.yaml 结构

```yaml
nodes:
  - id: "001"
    title: "退化线段"
    status: settled
  - id: "012"
    title: "谱系是发现引擎"
    status: settled

edges:
  - from: "001"
    to: "002"
    relation: triggered  # 001 的处理触发了 002 的发现
  - from: "012"
    to: "014"
    relation: derived    # 014 是 012 的推论
```

### 边类型

| relation | 含义 |
|----------|------|
| `depends_on` | 逻辑前置依赖 |
| `triggered` | 处理过程中触发发现 |
| `derived` | 逻辑推论关系 |
| `tensions_with` | 张力关系（非依赖） |
