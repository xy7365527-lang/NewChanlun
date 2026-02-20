---
id: "034"
title: "034 工具声明与工具能力的不一致性"
status: "已结算"
type: "矛盾记录"
date: "2026-02-17"
depends_on: []
related: ["016", "032", "033"]
negated_by: []
negates: []
---

# 034 工具声明与工具能力的不一致性

status: 已结算
type: 语法记录
provenance: "[新缠论]"
date: 2026-02-19
version: v1.0

## 触发事件

033 dispatch-spec 首次正式执行中，发现三个工具层面的不一致：

1. Bash subagent 幻觉执行：报告成功但文件未写入（Read 验证 3 次确认）
2. Serena 编辑工具注册缺口：get_current_config 显示 active，ToolSearch 找不到
3. Agent 定义 tools 与实际授权不一致：meta-observer.md 声明 Write，spawn 后无 Write

## 诊断

工具声明 != 工具能力。与 016（知道规则 != 执行规则）同构，发生在工具层。

| 016 | 034 |
|-----|-----|
| 规则写在 CLAUDE.md | 工具写在 agent 定义 |
| Lead 知道但不执行 | Agent 声明但无法使用 |
| 解法：runtime enforcement | 解法：工具能力验证 |

稳定信号：背驰（3 次验证，净新区分=0）+ 分型（从尝试修复到确认不可修复，范围闭合）

## 解法

1. 可靠写入路径：生成内容 -> 编排者终端 Python 写入 -> Read 验证
2. dispatch-spec 扩展：structural_stations 增加 tools_required 字段
3. ceremony 增加工具能力验证步骤

## 推导链

016（知道规则 != 执行规则）-> 033（dispatch-spec）-> 034（工具声明 != 工具能力）

## 谱系链接

- 前置：033-declarative-dispatch-spec
- 前置：016-runtime-enforcement-layer
- 关联：032-divine-madness-lead-self-restriction

## 边界条件

- Claude Code 更新修复 Bash subagent -> 幻觉部分失效，结构洞察仍有效
- Serena MCP 注册修复 -> 编辑工具可用，验证步骤仍应保留
- 工具能力验证增加 ceremony 开销 -> 需评估是否只在首次 spawn 时验证

## 影响声明

- 新增 Serena memory：tool-capability-map.md
- 需更新：dispatch-spec.yaml（增加 tools_required）
- 需结晶：tool-capability-verification skill
- 需更新：ceremony.md（增加工具能力验证步骤）
