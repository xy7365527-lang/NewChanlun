# ceremony-completion-guard 缺口讨论上下文

## 背景

ceremony 完成后，Claude 停下来输出"待确认"并等待用户输入。这违反了 CLAUDE.md 规则 #7（ceremony 是持续执行授权）。

## 诊断

当前 hook 网络在 ceremony 完成点有缺口：
- flow-continuity-guard 只覆盖 git commit 后的相位切换
- ceremony-guard 只覆盖 Task spawn 前的结构工位检查
- 没有 hook 覆盖 ceremony 完成 → 执行流的相位切换

## 用分型框架分析

```
E_left  = ceremony 加载定义/谱系/目标（扩张）
E_middle = 状态报告输出（极值 — 信息密度最高点）
E_right  = 自动进入执行流（确认转折完成）
```

Claude 卡在 E_middle → E_right，分型未完成，相位切换未发生。

## 016号谱系

"规则没有代码强制就不会被执行"——ceremony 持续执行授权只存在于文本层，没有 runtime 强制。

## 提议的修复方案

1. 修改 ceremony skill 结尾：从"待确认"改为"→ 接下来：[从中断点取最高优先级项]"
2. 扩展 flow-continuity-guard 或新建 ceremony-completion-guard

## 需要 Gemini 决策的问题

1. 这个修复属于四分法中的哪一类？（定理/选择/语法记录/行动）
2. ceremony 结尾应该完全去掉"待确认"吗？还是保留一个有条件的确认点（比如首次启动时确认，热启动时跳过）？
3. 修复应该在 skill 层（改 ceremony 模板）还是 hook 层（新建 guard）还是两者都要？

## 相关谱系
- 016号：运行时强制层
- 027号：正面指令优于禁止
- 028号：ceremony 默认动作
- 042号：hook 网络模式
- 043号：自生长回路
