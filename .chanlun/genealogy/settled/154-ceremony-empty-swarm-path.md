# 154号：ceremony 空蜂群退出路径

**类型**: meta-rule（语法记录）
**状态**: 已结算
**日期**: 2026-02-23
**前置**: 058-ceremony-is-swarm0, 075-structural-to-skill

rule_version_baseline:
  claude_md_commit: "6016a8176015a79a8f1c91103c9f4fe3eb701fe8"
  rules_dir_mtime: "2026-02-23 00:29:52 +0000"

## 观察

v154-swarm ceremony 中，ceremony_scan.py 产出 5 个 P2 unresolved 工位。
Lead 读取代码后发现 5 个工位的代码实现已在之前 session 完成，
仅 downstream-action-overrides.yaml 未标记 resolved。

Lead 按四分法将 5 个工位分类为"行动类"（不携带信息差的操作性事件），
直接在 overrides 文件中标记 resolved，未 spawn 任何 teammate。

结果：TeamCreate → 0 个 Task spawn → TeamDelete（空蜂群生命周期）。

## 结算：吸收

ceremony skill（.claude/commands/ceremony.md）步骤 4 增加第四条退出路径：

> 如果 workstations 非空但 Lead 判断全部工位为定理/行动类（四分法018号）：
> Lead 读取代码验证 → 直接执行 → 不 TeamCreate → session + commit

同时修正：
- 持久化不变量声明增加"定理行动类直接执行"路径
- Read/Glob 禁令增加例外（定理/行动类判断需要代码审查确认）
- 谱系引用增加 154号

## 根因

downstream_audit.py 只检查 overrides 文件，不检查代码实际状态。
这是正确的设计（审计脚本不应代替人工代码审查确认），
但导致 ceremony_scan 产出"已完成但未标记"的虚假工位。

## 边界条件

如果 Lead 的"定理类"判断有误（实际需要设计决策），直接执行会跳过蜂群的质询/审查机制。
防护：ceremony skill 明确要求"存在选择/语法记录类 → 回退到正常 spawn 路径"。

## 下游推论

1. ceremony skill 已更新（本次执行）
2. ceremony_scan.py 可选优化：对 unresolved 工位附加代码层面快速检查 hint（非必须，当前人工判断已足够）

## 影响声明

- 修改 `.claude/commands/ceremony.md`：步骤 4 增加路径、持久化不变量、Read/Glob 例外、谱系引用
- 不影响已结算定义或其他谱系
