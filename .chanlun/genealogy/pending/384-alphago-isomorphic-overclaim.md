---
id: "384"
status: 生成态
type: 矛盾发现
negation_source: heterogeneous
negation_form: unclassified
negation_model: "gemini-3.1-pro-preview"
created: "2026-03-05"
trigger: "v166-swarm/spec-review Task #1 Gemini 数学审查"
subject: "AlphaGo 类比声称'Isomorphic'——过强声称（Q-S3）"
depends_on: []
---

# 384号：AlphaGo 类比"Isomorphic"是过强声称

## 矛盾描述

规格第 5 节声称"Isomorphic to AlphaGo: Policy network (LLM) → Value network (Δβ₁ + T) → Game engine (topological layer)"。数学分析表明该类比在三处断裂，"Isomorphic"（同构）是过强声称。

## 推导链（Gemini 异质质询 + 同质确认）

**断裂 1：价值函数**
- AlphaGo 价值网络：V(s) ∈ [-1, 1]，逼近从当前状态到终局的期望累积回报（全局有界）
- T(op) = Δg·(1-s) + Δs·g：即时差分奖励，依赖绝对时间 t，无界，无终局
- 结论：T 是局部梯度，不是全局价值函数。（L0）

**断裂 2：MCTS 的前提不满足**
- MCTS 需要：有限视界或平稳状态空间 + 终局状态（backpropagation 基础）
- 拓扑计算：状态空间 K_full append-only 无限扩张，无终局条件
- 结论：MCTS 无法适用于无限扩张无终止的状态空间。（L0）

**断裂 3：训练机制**
- AlphaGo：策略/价值网络通过自对弈训练，反馈信号明确（胜/负）
- 规格中 LLM：外部预训练模型，训练机制未说明
- 结论："Isomorphic"要求结构对应完整，训练机制的缺失使类比不完整。

## 判定

"Isomorphic to AlphaGo"是过强声称。更准确的描述是"结构类比"（structural analogy）：
- LLM 作为候选生成器 ≈ 策略网络角色（不是完全对应）
- T 作为操作评分 ≈ 价值网络角色（但不是全局价值函数）
- 拓扑层作为执行引擎 ≈ 游戏引擎角色（但状态空间性质根本不同）

类比在角色功能层面部分有效，在数学结构层面断裂。

## 认识论等级

L0（从 T 的定义和 MCTS 前提直接推出）

## 边界条件

- 若规格引入明确的终止条件（如"settlement 完成后停止"），MCTS 类比在有限步骤内可能成立
- 若 T 被正规化到 [-1, 1] 并累积，价值函数类比在局部成立
- 若"Isomorphic"降级为"analogous to"，此矛盾消解

## 影响声明

影响：规格第 5 节（AlphaGo 类比声称）、规格第 8 节（比较表中"Effect predictability: Analytic"—此条正确，不受影响）
不影响：规格整体架构设计（LLM 作为候选生成器 + 拓扑层执行这一架构本身合理）
