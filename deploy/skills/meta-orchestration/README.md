# meta-orchestration

蜂群元编排方法论 — AI Agent蜂群的组织原则。

## 这是什么

这不是又一个项目管理框架。这是一套从AI蜂群的本体论出发推导的编排原则，核心观点：

- **蜂群是矛盾显现机器**，不是代码生产线。形式化过程中暴露的概念矛盾是最有价值的产出。
- **绕过矛盾是最大的敌人**。Workaround扼杀新知识的诞生。
- **编排者的位置是欲望和决断**，不是控制。AI有生成性但没有方向，编排者有方向但不需要懂代码。
- **谱系是系统的免疫机制**。记录每一次矛盾、分离、否定，防止系统退化回已被否定的方案。

## 适用场景

- 将领域知识形式化为代码的项目（如量化交易系统、法律规则引擎等）
- 编排者是领域专家而非程序员
- 项目中概念定义本身在演化
- 需要多agent协作并保持概念一致性

## 安装

```bash
# Claude Code
npx skills add [your-repo]/meta-orchestration

# 或手动复制到
~/.claude/skills/meta-orchestration/
```

## 文件结构

```
meta-orchestration/
├── SKILL.md                              # 主skill文件（agent指令）
├── README.md                             # 本文件
└── references/
    ├── methodology-v2.md                 # 完整方法论文档（十五条原则）
    ├── genealogy-template.md             # 谱系记录模板
    └── result-package-template.md        # 结果包模板
```

## 与 Everything Claude Code 的配合

本skill与 affaan-m/everything-claude-code 互补：
- ECC管工程层（TDD、代码审查、构建修复、安全审计）
- 本skill管概念层（质询循环、矛盾处理、谱系）
- 当两者冲突时，概念层优先
- Agent Teams模式下，ECC的agents自动受本skill的rules约束

## 理论谱系

本方法论的生成过程：
1. 从Nowak《超级合作者》和Chwe《理性的仪式》出发
2. 否定了核心预设（AI无背叛动机，人类合作理论不适用；共同知识概念被放弃——蜂群本体论上是一，不存在多主体间的互相知道）
3. 保留并重新定义了拓扑、图书馆/广场区分
4. 从蜂群本体论重新推导：接收→产出→传递
5. 产出质询循环、欲望驱动、生成态/结算态、回溯结算、谱系免疫
6. Agent Teams的并行劳动产生真实的认知分化，使teammates间质询具有实质内容
