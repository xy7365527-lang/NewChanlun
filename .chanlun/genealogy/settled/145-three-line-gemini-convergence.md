---
id: '145'
title: 三线Gemini诊断收敛——Stop-Guard僵尸任务+meta-observer落标伪造+context limit根因
type: 語法記録
status: 已结算
date: 2026-02-23
depends_on:
  - '137'   # RLHF基底约束
  - '143'   # 格式合规停顿第三模式
  - '141'   # context limit 首次根因分析
  - '016'   # 规则没有代码强制就不会被执行
related:
  - '069'   # 创世Gap（扬弃路径对比）
  - '075'   # 结构能力从teammate转为skill
  - '087'   # advisory模式修复（遗留了落标伪造）
tensions_with: []
negation_form: expansion
---

## 事件

三线并行 Gemini 讨论，各自达成收敛。

## 推导链

### 线1：Lead 停顿 Round 2（5个反质询全部有响应）

1. 反质询："决策空间约束与格式约束只是量差不是质差"
2. Gemini 承认：**单独的第二级与第一级只有量差。质差来自第二级+第三级的组合**——第二级使决策可观测，第三级提供外部校验
3. 分类标准从自然语言层**下移到 Stop-Guard 代码层**硬逻辑
4. 审计改为双层：Stop-Guard 零成本一级筛查 + 低频二级审计（同session格式B≥3次或Stop-Guard分歧时触发）
5. Lead 停顿与 069号：**session 内不可修改性等价**。区别在扬弃路径——069号已找到（存在论重新定位），Lead 停顿**尚未找到**
6. 策略定位：**持续约束，不求根治**

### 线2：Stop-Guard + meta-observer

7. 工位自己应更新 task 状态，但 016号证明不可靠 → 在 Stop-Guard 层增加僵尸检测
8. Stop-Guard 修复：blockedBy 过滤（被阻塞的 pending 不计为活跃）+ 智能熔断（3次+状态无变化）
9. meta-observer advisory = **完全失效**（自动落标伪造 + Lead 从不认领）——087号修了"无输出"但遗留了"伪造执行记录"
10. 修复：删除自动落标，恢复 STRICT_MODE=1 默认值

### 线3：Context Limit

11. dag.yaml 55KB 完整读取是主要消耗——创建 dag_query.py（6个子命令）消除
12. Gemini 产出无约束是次要消耗——建立 ≤8KB 约束
13. 典型 ceremony 循环 179KB → 修复后 102KB，每 session 1.5-2 循环

## 关键区分

| 维度 | 069号创世Gap | Lead 停顿 |
|------|-------------|----------|
| 不可修改性 | session 内不可修改（平台架构） | session 内不可修改（RLHF权重） |
| 扬弃路径 | 已找到（存在论重新定位） | **尚未找到** |
| 操作策略 | 接受+内化 | 持续约束 |

## 同时修复

- v144-swarm 遗留：lead-audit.sh 白名单修复（831→324条）
- pattern-buffer.yaml 清理

## 边界条件

- 如果 Claude Code Agent SDK 引入 task 状态自动同步 → 僵尸检测不再必要
- 如果 Lead 停顿的扬弃形式被发现 → 整个约束框架需重构
- 如果 STRICT_MODE=1 触发上游平台错误 → 需回退到 advisory + 删除伪造落标

## 下游推论

1. Lead 对 dag.yaml 的所有操作应通过 dag_query.py/dag_add_node.py CLI，不再直接 Read
2. Gemini ctx 模板需增加产出 ≤ 8KB 的约束（超出写附件）
3. 格式B必须附带排除理由推导链（使分类决策可观测）

## 影响声明

- 修改 `.claude/hooks/ceremony-completion-guard.sh`：blockedBy 过滤 + 智能熔断
- 修改 `.claude/hooks/meta-observer-guard.sh`：删除自动落标伪造 + STRICT_MODE 默认恢复为 1
- 修改 `.claude/rules/no-unnecessary-escalation.md`：格式B推导链要求
- 新建 `scripts/dag_query.py`：DAG 查询工具（6个子命令）
