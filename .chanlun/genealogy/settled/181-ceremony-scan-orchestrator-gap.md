---
number: 181
type: meta-rule
status: 已结算
date: 2026-02-24
title: ceremony_scan 与编排者决断的接口缺口
rule_version_baseline:
  claude_md_commit: "97f3ba3186f1919ff6abf02dea4247724d47c79a"
  rules_dir_mtime: "2026-02-23 21:10:15 +0000"
genealogy_refs:
  - 154  # ceremony 空蜂群退出路径
  - 162  # RTAS 持久化由 session 结晶保证
  - 174  # 谱系即生成引擎
topo_effect: "ceremony_scan 的工位推导逻辑需要识别 session 恢复指引中的编排者决断"
downstream:
  - "ceremony_scan.py 增加 session 恢复指引解析（从 session 的'待处理'段提取未完成的编排者决断工位）"
  - "或者：承认 ceremony_scan 只推导谱系增量工位，编排者决断注入的工位由 Lead 从 session 手动读取——这是合理的设计边界"
---

# 181号: ceremony_scan 与编排者决断的接口缺口

## 观察

v43-swarm session 中，ceremony_scan 返回 `clean_terminate=true`（无工位），但编排者在 v42-swarm 结束时给出了明确的架构决断（立场差分架构），session 恢复指引的"待处理"段记录了具体的实现任务。

Lead 从 session 文件手动读取编排者决断，手动创建蜂群和工位。这个模式已在运作但未显式化。

## 张力

ceremony_scan 的工位推导逻辑基于两个数据源：
1. 谱系增量（delta_genealogy）
2. 区块增量（delta_blocks）

**缺失的第三数据源**：session 恢复指引中的"待处理"段（编排者决断产生的实现任务）。

这不是 bug——ceremony_scan 设计时就只关注谱系状态的确定性推导。编排者决断是非确定性的（每次可能不同），ceremony_scan 无法预测。

## 四分法分类

**语法记录候选**：Lead 已经在实践中用"从 session 恢复指引读取编排者决断 → 手动 TeamCreate"的方式处理这个缺口。这个模式需要决定：

- **选项 A**：显式化这个模式为 ceremony skill 的一部分（warm_start 时自动解析 session 的待处理段）
- **选项 B**：承认这是合理的设计边界——编排者决断超出自动推导范围，由 Lead 手动处理

两个选项都合理，这是一个需要编排者价值判断的**选择**。

## 收敛/发散判定

与 154号（ceremony 空蜂群退出路径）相关但是**发散**——154号处理的是空工位的退出路径设计，本次观察的是非空但非谱系推导的工位来源。新维度。

## 附加观察

v43-swarm 的 stance-diff-impl 工位展示了一个正面模式：工位从编排者决断中自主推导出未显式列出的工作项（stance_parser.py + registry.py 修改）。这是四分法"定理"类的正确执行——已知约束（"只能控制输入和输出协议"）的逻辑必然推论。
