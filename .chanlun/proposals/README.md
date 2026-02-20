# 结构修改提案模式

本目录存放对项目结构、定义、谱系的变更提案。所有非平凡的结构修改必须先以提案形式提出，经审查批准后再实施。

## 流程

1. **创建提案**：在本目录创建 `PROP-NNN-<slug>.md`
2. **审查**：相关工位/编排者审查提案内容
3. **批准**：满足 `approval_condition` 后将 status 改为 `approved`
4. **实施**：按提案内容执行变更，完成后将 status 改为 `implemented`

## 提案模板

```yaml
---
proposal_id: PROP-NNN
title: <简明标题>
scope:
  - <影响的文件/定义/谱系路径>
genealogy_ref:
  - <相关谱系编号，如 "settled/012-genealogy-is-discovery-engine.md">
motivation: |
  为什么需要这个变更。
changes: |
  具体变更描述。
approval_condition: |
  批准条件：谁批准、什么条件下批准。
status: draft  # draft | pending_review | approved | rejected | implemented
---
```

## 字段说明

| 字段 | 必填 | 说明 |
|------|------|------|
| proposal_id | 是 | 格式 `PROP-NNN`，递增编号 |
| title | 是 | 简明描述变更内容 |
| scope | 是 | 影响范围：文件路径、定义名、谱系编号 |
| genealogy_ref | 否 | 相关谱系引用（无则留空） |
| motivation | 是 | 变更动机 |
| changes | 是 | 具体变更内容 |
| approval_condition | 是 | 批准条件 |
| status | 是 | 当前状态 |

## 编号规则

- 从 `PROP-001` 开始递增
- 文件名格式：`PROP-NNN-<短横线分隔的slug>.md`
