---
name: project-topology
description: 项目文件拓扑与指令架构。需要定位文件、理解目录结构、或查找 agent 职责时激活。包含知识仓库映射、谱系目录、分布式指令卡组。
---

### 知识仓库映射
元编排中的 `knowledge/` 在本项目中对应：
- 速查定义：`缠论知识库.md`
- 域对象 schema：`definitions.yaml`
- 规则规范：`docs/spec/`（segment_rules_v1.md, zhongshu_rules_v1.md, move_rules_v1.md 等）
- 原文参考：`docs/chanlun/text/chan99/`（编纂版，已知不完整；完整原文见三级权威链）

### 谱系目录
- `.chanlun/genealogy/pending/` — 生成态矛盾（含 `tension-` 前缀的深度张力待审）
- `.chanlun/genealogy/settled/` — 已结算记录
- 谱系三种状态：`生成态` / `已结算` / `深度张力待审`（019d）

### 分布式指令架构（谱系014）

元编排指令不再是单一 SKILL.md 单体，而是分布式指令卡组：

| 文件 | 加载者 | 内容 |
|------|--------|------|
| `SKILL.md`（核心卡） | 所有 agent | 概念分离、决策分层、质询序列、生成态/结算态 |
| `.claude/agents/meta-lead.md` | Lead | 薄路由层：收信→判断类型→转发工位 |
| `.claude/agents/genealogist.md` | 谱系工位 | 谱系写入、张力检查、回溯扫描 |
| `.claude/agents/source-auditor.md` | 源头审计 | 三级权威链、溯源标签、原文考古 |
| `.claude/agents/meta-observer.md` | 元规则观测 | 二阶反馈回路、元编排进化 |
| `.claude/agents/quality-guard.md` | 质量守卫 | 结果包检查、代码违规扫描 |
| `.claude/agents/topology-manager.md` | 拓扑管理 | 工位扩张/收缩建议 |
| `.claude/agents/claude-challenger.md` | 反向质询 | 对 Gemini 产出再质询，防止单向质询退化 |
| `references/methodology-v3.3.md` | 参考文档 | 完整方法论（不直接加载） |

**设计原则**：一个 agent 一件事。Lead 只路由不认知。复杂度通过 agent 数量扩展。
