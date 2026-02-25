---
id: "201"
number: 201
type: meta-rule
status: 已结算
date: 2026-02-25
trigger: meta-observer-guard（016号强制）
session: context-recovery-repair
rule_version_baseline:
  claude_md_commit: "999de98"
  rules_dir_mtime: "2026-02-25"
---

# 201号：上下文恢复 + 结构修复 session 二阶观察

## 观察摘要

本 session 因上下文压缩触发恢复。ceremony_scan 检测到 P0 异常（3 个谱系缺 id 字段 + 5 个缺 block-topology 映射），
均为 v51 session 快速产出时遗漏的结构元数据。修复为行动类（四分法），直接执行。

## 规则触发/违反模式

| 规则 | 触发情况 | 判定 |
|------|---------|------|
| 016号（meta-observer guard） | Stop hook 触发二阶观察 | 正常触发 |
| 137号（强制输出格式） | 格式A 执行 | 正常 |
| no-unnecessary-escalation | 编排者声明"不需要问我"，P0 异常按行动类自主修复 | 正常 |

无新违反模式。

## 模式观察

### 自主 session 的结构债务

v51 session 在编排者缺席下快速产出 4 个谱系（197-200），但遗漏了：
- frontmatter `id` 字段（198/199/200）
- block-topology 映射（196-200）

这是"自主高速产出 → 结构元数据不完整"的模式。与 200号（僵尸任务）同构：
快速执行时非核心元数据被跳过。

**判定**：收敛信号。200号已记录此类模式。不写新规则，但标记：
ceremony_scan 的 P0 异常检测是此类债务的有效捕获机制。

## 元规则一致性

无不一致。stagnation 警告（settled 计数未变）是预期行为——本 session 是修复，不是生成。

## 自环检查

- 198号候选1（决策权委托）：本 session 再次验证——编排者"不需要问我"，蜂群自主修复。模式稳定
- 198号候选2（自我质询）：本 session 未触发
- 无新发散信号
