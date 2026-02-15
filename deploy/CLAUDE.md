# 缠论量化交易系统

## 项目性质

这是一个将缠论理论形式化为量化交易代码的项目。编排者是缠论领域专家，不写代码。

**核心原则：概念层优先于工程层。** 当代码实现与缠论定义冲突时，停下来上浮矛盾，不写workaround。

## 技术栈

- 语言：Python
- 蜂群框架：Claude Code + everything-claude-code plugin + meta-orchestration skill

## 目录约定

```
project/
├── knowledge/          # 概念定义仓库（缠论术语的形式化定义）
├── genealogy/          # 谱系记录
│   ├── settled/        # 已结算的矛盾
│   └── pending/        # 生成态的矛盾
├── src/                # 源代码
├── tests/              # 测试
└── CLAUDE.md           # 本文件
```

## 工作流

1. 编排者用缠论语言描述需求
2. `/ceremony` 启动蜂群，加载定义和谱系
3. `/plan` 制定实现计划（自动检查定义结算状态）
4. `/tdd` 执行TDD循环（遇到定义冲突自动停止）
5. `/code-review` 审查代码（概念层质询 + 工程层审查）
6. `/inquire` 对结果包执行质询
7. `/escalate` 上浮不可解的矛盾
8. 编排者做决断后 `/ritual` 广播新定义

## 规则优先级

1. `rules/no-workaround.md` — 核心禁令（最高优先）
2. `rules/result-package.md` — 结果包格式
3. `rules/testing-override.md` — 测试规则（含生成态例外）
4. ECC 原版 rules（security, git-workflow, coding-style）
