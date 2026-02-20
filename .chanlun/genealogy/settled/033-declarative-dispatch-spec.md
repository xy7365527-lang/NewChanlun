---
id: "033"
title: "033 声明式工位分派规范"
status: "已结算"
type: "矛盾记录"
date: "2026-02-17"
depends_on: []
related: ["005a", "016", "018", "020", "032"]
negated_by: []
negates: []
---

# 033 声明式工位分派规范

status: 已结算
type: 语法记录
provenance: "[新缠论]"
date: 2026-02-19
version: v1.0

## 触发事件

032 号谱系（神圣疯狂/Lead 自我权限剥夺）解决了 Lead 的执行自由度（不能自己干活），但未约束路由自由度（Lead 决定 spawn 什么）。Lead 在三次 ceremony 中均跳过结构工位（genealogist/quality-guard/meta-observer），只 spawn 任务工位。

模式链：016（知道规则 ≠ 执行规则）→ 032（约束执行自由度）→ 033（约束路由自由度）。

## 诊断

### Gemini 异质质询结论

三个约束方案（A: ceremony 执行者倒置 / B: SessionStart hook 注入 / C: PostToolUse hook 验证）均被否决：

| 方案 | 致命弱点 |
|------|----------|
| A | 混淆权限与角色，问题转移到主线程 |
| B | 依赖生成惯性（概率性），范畴错误：把规范性约束转化为概率性倾向 |
| C | 事后检查无法纠正，PostToolUse 阶段 Lead 已做出决策 |

### 根因分析

016→032→033 的递归模式是**约束思路的必然结果**：
- 系统中的自由度是多维的（执行权、工具权、路由权、决策权、优先级权、跳过权……）
- 每次约束一个维度，问题转移到相邻维度
- 这是无限的打地鼠游戏

### 范式转换

从**约束**（prohibition，消极："不能做什么"）转向**规范**（specification，积极："应该做什么"）：

- 约束思路：自由度空间无限大，永远堵不完
- 规范思路：行为空间被正面定义，偏差变成可判定的实现错误

与 005a（对象否定对象）同构：否定必须来自内在结构，不能来自外部阈值。

## 解法

### dispatch-spec.yaml

声明式工位分派规范文件（`.chanlun/dispatch-spec.yaml`），正面定义：
1. **结构工位**：每次 ceremony 必须 spawn 的工位列表（mandatory=true, can_be_skipped=false）
2. **任务工位**：从中断点动态派生的规则
3. **ceremony 执行序列**：冷启动/热启动的完整步骤链
4. **验证规则**：post-ceremony 检查条件

### Lead 角色重定义

Lead 从"决策者"变为"spec 解释器"：
- 读取 dispatch-spec.yaml
- 按 spec 执行 spawn
- 偏离 spec = 实现错误（可判定、可审计）

### 修改路径

dispatch-spec.yaml 的修改与 definitions 同级，走 `/ritual` 仪式门控。修改提案由 meta-observer 产出。零新增常驻工位。

## 推导链

016（runtime enforcement layer）
  → 032（Lead 自我权限剥夺 = 约束执行自由度）
    → 033（dispatch-spec = 消除路由自由度）

关键转折：从"约束"到"规范"的范式转换。约束是在自由度空间中打补丁（无限维），规范是正面定义行为空间（有限、可判定）。

## 谱系链接

- 前置：032-divine-madness-lead-self-restriction（直接前驱）
- 前置：016-runtime-enforcement-layer（模式起源）
- 关联：005a-prohibition-theorem（对象否定对象 = 否定来自内在结构）
- 关联：020-constitutive-contradiction（无特权编排者）
- 关联：018-four-way-classification（本条为"语法记录"类型）

## 边界条件

- 如果 Lead 的实现不读取 dispatch-spec.yaml → spec 形同虚设（需要 ceremony.md 中硬编码引用）
- 如果 spec 本身有错误 → meta-observer 负责诊断，/ritual 负责修正
- 如果出现 spec 中未预见的工位类型 → 需要扩展 spec schema（走 /ritual）

## 影响声明

- 新增文件：`.chanlun/dispatch-spec.yaml`
- 需更新：`.claude/commands/ceremony.md`（引用 dispatch-spec）
- 需更新：`.claude/agents/meta-observer.md`（增加 dispatch-spec 修改提案职责）
- 需更新：session 模板（记录 dispatch-spec 版本）
