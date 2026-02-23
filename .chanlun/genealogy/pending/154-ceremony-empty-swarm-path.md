# 154号：ceremony 空蜂群退出路径

**类型**: meta-rule（语法记录候选）
**状态**: 生成态
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

## 发现

ceremony skill 的流程覆盖了以下退出路径：
1. workstations 为空 → 020号反转（干净终止）
2. workstations 非空、全部"待 Gemini decide"或"长期" → 路由 Gemini / 显式阻塞
3. workstations 非空、有可执行项 → spawn 蜂群

缺失的路径：
4. **workstations 非空，但 Lead 判断全部为定理/行动类（代码已完成、只需确认性标记）** → 当前行为是 Lead 直接执行，不 spawn 蜂群

路径 4 是合法的四分法判断，但 ceremony skill 中未显式描述。
这导致 TeamCreate 后立即 TeamDelete 的空蜂群反模式。

## 根因

downstream_audit.py 只检查 overrides 文件，不检查代码实际状态。
这是正确的设计（审计脚本不应代替人工代码审查确认），
但导致 ceremony_scan 产出"已完成但未标记"的虚假工位。

## 建议（语法记录候选）

ceremony skill 应增加路径 4 的显式描述：
- 如果 Lead 读取代码后判断全部工位为定理/行动类，允许 Lead 直接执行，不 spawn 蜂群
- 此时不执行 TeamCreate（避免空蜂群），直接走"定理/行动类批处理"路径
- 或：ceremony_scan.py 增加 hint 字段，标注"可能已在代码层面完成"的工位

## 边界条件

如果 Lead 的"定理类"判断有误（实际需要设计决策），直接执行会跳过蜂群的质询/审查机制。
但当前案例中 5 个工位确实只是标记确认，无设计决策。

## 下游推论

1. ceremony skill 补充路径 4 描述
2. ceremony_scan.py 可选：对 downstream_audit 产出的 unresolved 工位，附加代码层面的快速检查 hint

## 影响声明

- 不改变现有代码
- 识别 ceremony skill 的一个未覆盖场景
- 不影响已结算的定义或谱系
