# 任务：重写6张agent定义卡

## 背景（不需要重新推导）

谱系014决定SKILL.md从633行拆为分布式指令卡组。核心卡已commit（95行）。现在写`.claude/agents/*.md`。

所有架构决断已确认，直接执行：

## 关键决断清单

1. **通信模型 = 中断/文件系统二元。** 常规信息流通过文件系统（agent写约定位置，其他agent下次被唤起时发现）。SendMessage只用于中断（必须让对方立刻停下来）。
2. **中断只有三种：** 概念分离信号、仪式后定义变更广播、实现层僵持。其他一切走文件系统。
3. **Lead = 中断路由器，不是调度器/消息路由器。** 正面职责三件：收中断、扫文件系统、出轴线汇报。其余全是"不做"清单。
4. **上浮条件 = 逐条查status字段，不是全局阶段判断。** 涉及的那条定义是`status: 生成态`→上浮。`status: 已结算`→文件系统自动处理。
5. **违规记录：** auto-修复的不持久化。升级为概念争议的自然进谱系（type: domain或bias-correction）。不新建inbox目录。
6. **拓扑建议：** 不持久化。被唤起时直接返回建议给Lead，Lead轴线汇报时提及。

## 输入文件

- 编排者定稿的meta-lead.md已上传（134行）——基于这个做中断路由器调整
- SKILL.md核心卡已上传（99行）——作为参照
- repo里现有的6张agents/*.md是旧版——需要重写

## 每张卡的统一格式

```markdown
---
name: xxx
description: >
  一句话职责描述
tools: [...]
model: opus
---

正文开头：一句话角色定位

## 核心职责

## 文件系统产出
（写什么、写到哪里）

## 文件系统输入
（读什么、什么时候读）

## 中断场景
（产生中断 / 响应中断）

## 上浮条件
（status查询逻辑）

## 你不做的事
```

## 执行顺序

一张卡一个commit：
1. meta-lead.md（基于定稿调整通信语言）
2. genealogist.md
3. quality-guard.md
4. source-auditor.md
5. meta-observer.md
6. topology-manager.md

## 验证标准

- 中断一致性：meta-lead的中断表中每种中断，在对应工位卡中有镜像
- 文件系统对称性：A卡说"写入xxx"，至少一张B卡说"读取xxx"
- 除三种中断外，不应有SendMessage/转发/通知语言
- status查询逻辑全局一致

## 约束

不改SKILL.md、谱系文件、definitions、CLAUDE.md。所有改动在claude/ceremony-feature-CInoK分支。
