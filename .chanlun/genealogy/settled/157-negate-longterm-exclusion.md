# 157号：否定 long_term 排除分类

**id**: 157
**status**: 已结算
**type**: 语法记录（编排者 INTERRUPT）
**date**: 2026-02-23
**negates**: 无（对规则文件中隐含分类的否定）
**前置**: 156-negate-ceremony-empty-swarm-path, 055-no-simple-task-exemption

## 矛盾发现

`no-unnecessary-escalation.md` 格式B排除分类中包含 `long_term（附具体重新激活条件）`。
这意味着蜂群可以将某些工位标记为"长期搁置"——即承认存在蜂群当前不处理的合法工位。

156号已结算：Lead 必须用蜂群处理所有工位，"长期/可选"不是合法排除。
但 `long_term` 分类仍残留在格式B的排除分类列表中，与156号的下游推论直接矛盾。

## 决断

编排者 INTERRUPT（与156号同源）：
> 所有工位都应该被蜂群处理，不存在"长期搁置"的合法性。

## 结算：否定

从 `no-unnecessary-escalation.md` 格式B排除分类中删除 `long_term`。
排除分类仅接受：resolved（已完成）、blocked（附具体阻塞项）。

## 边界条件

如果某工位因外部物理约束（API不可用、权限缺失等）无法推进，应分类为 `blocked` 而非 `long_term`。
blocked 必须附具体阻塞项，阻塞解除后蜂群自动处理。

## 下游推论

1. 格式B排除分类从三元（resolved/long_term/blocked）收缩为二元（resolved/blocked）
2. 任何之前被标记为 long_term 的工位应重新分类为 blocked（附具体阻塞项）或直接处理

## 谱系依据

- 156号：Lead 必须用蜂群，"长期/可选"不是合法排除
- 055号：无简单任务豁免

## 影响声明

- 新增谱系 157号
- 修改 `.claude/rules/no-unnecessary-escalation.md`：格式B排除分类删除 `long_term`
