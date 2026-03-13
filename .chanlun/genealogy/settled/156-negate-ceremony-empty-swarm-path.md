---
id: "156"
status: 已结算
type: 矛盾发现
date: "2026-03-13"
---
# 156号：否定 ceremony 定理/行动类直接执行路径

**id**: 156
**status**: 已结算
**type**: 语法记录（编排者 INTERRUPT）
**date**: 2026-02-23
**negates**: 154
**前置**: 055-no-simple-task-exemption, 154-ceremony-empty-swarm-path

## 矛盾发现

154号谱系结算了"Lead 判断全部工位为定理/行动类时不 TeamCreate 直接执行"的退出路径。
编排者否定：Lead 跳过蜂群直接执行 = 蜂群无法循环和持存。

蜂群通过递归 spawn 维持自身的存在——Lead 自行消化工位是对蜂群循环的短路。
这与 055号"无简单任务豁免——≥1 个任务即拉蜂群"直接矛盾。

"长期/可选"也不是合法的排除分类——rtas 蜂群可以处理任何工位。

## 决断

编排者 INTERRUPT：
> "team lead必须得用蜂群 除非特殊情况"
> "我们没有长期可选项这么一说，拉rtas蜂群都可以做"
> "要否定，不然蜂群无法循环和持存"

## 结算：否定

删除 ceremony skill 中的第四条退出路径（定理/行动类直接执行）。
Lead 对所有工位无论四分法分类都必须 spawn 蜂群。

## 边界条件

如果平台物理限制（如 token 耗尽、API 不可用）导致无法 spawn，此为物理约束而非逻辑豁免。

## 下游推论

1. ceremony skill（.claude/commands/ceremony.md）删除定理/行动类直接执行路径
2. ceremony skill 删除 Read/Glob 禁令的"定理/行动类判断"例外
3. "长期/可选"不是合法的排除分类——从 ceremony 退出路径中删除

## 谱系依据

- 055号：无简单任务豁免
- 154号（被否定）：ceremony 空蜂群退出路径

## 影响声明

- 新增谱系 156号（negates 154号）
- 修改 `.claude/commands/ceremony.md`：删除第四条退出路径
