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
status: implemented
review_date: "2026-02-21"
reviewer: "workstation-B (genealogist + quality-guard)"
review_verdict: "核心设计合理，3处必须修正后方可实施"
amendments_required:
  - id: A1
    severity: blocking
    description: "字段命名不一致：changes 中写 `relates_to`，但现有全部谱系文件使用 `related`。必须统一为 `related`。"
  - id: A2
    severity: blocking
    description: "边类型缺失：dag.yaml 的 4 种边类型缺少 `negates` 和 `negated_by`。068号谱系已使用 negates: [065, 066]，否定关系是体系核心概念（005a 禁止定理），不可遗漏。"
  - id: A3
    severity: blocking
    description: "`tensions_with` 不等价于 `related`。现有 `related` 是通用关联（如 070 related: [048, 059] 并非张力），需保留 `related` 作为通用关联边，`tensions_with` 作为特定张力边。"
recommendation: |
  建议 dag.yaml 边类型扩展为 7 种：
  depends_on, triggered, derived, tensions_with, related, negates, negated_by。
  dag.yaml 应从 frontmatter 生成（frontmatter 为单一真相源），而非独立维护。
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
