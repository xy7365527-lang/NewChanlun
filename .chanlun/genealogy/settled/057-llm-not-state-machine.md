---
id: "057"
title: "LLM 不是状态机——状态管理权外移原则"
status: "已结算"
type: "语法记录"
date: "2026-02-20"
depends_on: ["043", "044", "048"]
related: ["055"]
negated_by: []
negates: []
---

# 057: LLM 不是状态机——状态管理权外移原则

**类型**: 语法记录
**状态**: 已结算
**日期**: 2026-02-20
**前置**: 048, 044, 043

## 语法规则

**LLM 是推理引擎，不是状态机。** 系统不应将以下责任委托给 LLM agent：
- 生命周期锁的创建/销毁时序
- 元数据与代码的一致性同步
- 任何需要原子性保证的状态转换

**推论**：凡是需要状态一致性的操作，应由框架层（hook/脚本/CI）自动执行，agent 只触发信号，不执行状态变更。

## 发现经过

ceremony 热启动中暴露两个独立故障：
1. agent 在同一 turn 内先 `rm .ceremony-in-progress` 再停止，stop-guard 检查时 flag 已无，放行
2. `definitions.yaml` 的 `implementation_status` 有 2/14 条目腐化（买卖点 stub→实际complete，流转关系 partial→实际complete）

Gemini 3.1 Pro Preview（challenge 模式）诊断共同根因为"对 LLM 行为确定性的错误假设"——系统将状态管理委托给 LLM，而 LLM 无法保证语义约束与物理动作的原子性绑定。

## 已执行修复

| 修复 | 文件 | 机制 |
|------|------|------|
| 信任倒置消除 | `ceremony-completion-guard.sh` | 二次 block 机制：第一次设标记，第二次框架自动清除 flag。agent 全程不接触 flag |
| 文件名不一致 | `session-start-ceremony.sh` | `.ceremony-stop-counter` → `.stop-guard-counter` + 清理 `.ceremony-blocked-once` |
| 数据腐化 | `definitions.yaml` | 买卖点 stub→complete，流转关系 partial→complete |

## 异质否定来源

Gemini 3.1 Pro Preview，三条否定全部成立：
1. 守卫信任倒置（致命）→ 已修复
2. 静态缓存必然腐化（重要）→ 部分修复（数据已更正，机制待决）
3. LLM 不是状态机（致命，共同根因）→ 已显式化为语法规则

## 下游开放问题（已决断）

`definitions.yaml` 的 `implementation_status` 字段：Gemini decide 模式选择 **A. 废除**。
- 理由：代码是唯一事实来源，YAML 缓存产生认知锚定偏差，方案 B/C 是半吊子
- 已执行：14 个实体的 `implementation_status` 字段全部移除
- 边界条件：项目规模膨胀到动态扫描成本不可接受时，改用 AST 自动生成器（非手动 YAML）
- 人类保留 INTERRUPT 权

## 谱系链接

- 048号（universal-stop-guard）：被本条修改
- 044号（phase-transition-runtime）：ceremony 相位转换设计
- 043号（self-growth-loop）：模式缓冲区，自动化检测雏形
- 055号（double-helix-architecture）：hook 网络的整体架构
