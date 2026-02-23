# 145号下游推论修复报告

## 145-1：dag_query.py CLI 化 DAG 访问

**状态**：resolved

**验证结果**：
- `scripts/dag_query.py` 存在，包含 6 个子命令：`--exists`, `--node`, `--edges`, `--edge-exists`, `--stats`, `--recent`
- `scripts/dag_add_node.py` 存在，支持 `--id`, `--title`, `--type`, `--file`, `--depends_on`, `--related`, `--tensions_with`, `--negates` 参数
- 两个脚本覆盖了 dag.yaml 的所有读写操作，Lead 无需直接 Read dag.yaml

**标注理由**：工具已实现且功能完整。145号谱系声明的行为约束（"Lead 对 dag.yaml 的所有操作应通过 CLI"）是规则层声明，不需要额外代码。

---

## 145-2：Gemini ctx 模板 ≤ 8KB 约束

**状态**：resolved

**修改文件**：
1. `.claude/agents/gemini-challenger.md` — 在"质询前：构建上下文"章节后增加"产出大小约束"子章节
2. `.claude/skills/orchestrator-proxy/SKILL.md` — 在构建规则中增加第 6 条：产出大小 ≤ 8KB

**约束内容**：Gemini 直接回复（写入谱系或汇报给团队的内容）不超过 8KB。超出部分写入 `tmp/` 附件文件，正文仅保留摘要 + 附件路径引用。

**标注理由**：8KB ≈ 2000 token，是单次质询/决策结果嵌入 Claude agent 上下文的合理上限。约束已写入两个规范文件，覆盖 challenge/verify 和 decide 两种模式。

---

## 145-3：格式B排除理由推导链

**状态**：resolved

**验证结果**：`.claude/rules/no-unnecessary-escalation.md` 第 32-41 行已包含完整的格式B排除理由推导链要求：

```
145号修复（Gemini Round 2 收敛）：格式B必须附带排除理由推导链。
目的不是阻止格式B使用，而是使分类决策可观测——为 Stop-Guard 一级筛查和异质审计提供数据。
排除分类仅接受：resolved（已完成）、long_term（附具体重新激活条件）、blocked（附具体阻塞项）。
"误报"不是合法的排除分类——必须给出为什么该工位不满足可执行条件的推导。
```

**标注理由**：在上一轮（v147-swarm 或之前）已经实现。格式B定义中明确要求排除理由推导链，且限定了合法的排除分类（resolved/long_term/blocked）。

---

## 影响声明

- 修改文件：`.claude/agents/gemini-challenger.md`（增加产出大小约束章节）
- 修改文件：`.claude/skills/orchestrator-proxy/SKILL.md`（构建规则增加第6条）
- 修改文件：`.chanlun/downstream-action-overrides.yaml`（145-1/2/3 标注 resolved）
- 新建文件：`tmp/downstream-145-report.md`（本报告）
