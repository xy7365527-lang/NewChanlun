---
number: 181
type: meta-rule
status: 已结算
date: 2026-02-24
title: ceremony_scan 与 session 结晶的职责边界确认
rule_version_baseline:
  claude_md_commit: "97f3ba3186f1919ff6abf02dea4247724d47c79a"
  rules_dir_mtime: "2026-02-23 21:10:15 +0000"
genealogy_refs:
  - 154  # ceremony 空蜂群退出路径
  - 162  # RTAS 持久化由 session 结晶保证
  - 174  # 谱系即生成引擎
topo_effect: "无——现有设计正确，不需要变更"
downstream:
  - "ceremony_scan 不扩展：它是文件系统扫描器，不是对话理解器"
  - "session 结晶继续作为编排者决断的独立传递通道"
---

# 181号: ceremony_scan 与 session 结晶的职责边界确认

## 原始观察

v43-swarm session 中，ceremony_scan 返回 `clean_terminate=true`（无工位），但编排者在 v42-swarm 结束时给出了明确的架构决断（立场差分架构），session 恢复指引的"待处理"段记录了具体的实现任务。Lead 从 session 文件手动读取编排者决断，手动创建蜂群和工位。

## 原始判断（已修正）

~~原始判断将此视为"接口缺口"和"语法记录候选"。~~

## 编排者修正

这不是缺口，不是张力，不需要谱系化为语法记录。原始判断犯了角色越界的错误：

1. **ceremony_scan 的输入是物质变化（文件系统）**，不是对话历史。让它读 session 恢复指引来推导工位 = 让文件系统扫描器变成对话理解器 = 职责膨胀。
2. **两条独立信息通道各自正确运作**：
   - ceremony_scan：从谱系/区块增量自动推导工位
   - session 结晶：传递编排者决断给下一轮 CC
3. **Lead 从 session 读取决断启动工位不是"绕过"ceremony_scan**——这就是正确路径。编排者决断是人类行为，通过人类可读通道（session 文件）传递，由 CC 在下一轮启动时读取。

结论：两个系统的正确分工被误读为"接口缺口"。无遗漏，无张力。

## 附加观察（保留）

v43-swarm 的 stance-diff-impl 工位从编排者决断中自主推导出未显式列出的工作项（stance_parser.py + registry.py 修改）。四分法"定理"类的正确执行。
