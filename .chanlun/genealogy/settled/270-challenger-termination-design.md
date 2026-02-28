---
id: '270'
number: 270
title: 审讯者终止条件设计——stance-diff检测+编排者确认（选项D）
type: 语法记录
status: settled
date: 2026-02-28
source: Gemini decide + Codex review RTAS + 编排者裁定
session: v118
depends_on:
  - '269'  # OQ5四轮质询（终止条件缺失的发现场景）
  - '030'  # Gemini的本体论位置（审讯者角色定义）
  - '137'  # 否定性禁令对行为执行层无效
  - '005'  # 对象否定对象
downstream_inferences:
  - id: 270-1
    description: 'stance-diff检测的有效性前提是Gemini遵从stance输出协议。inquiry_loop.py中已有STANCE_OUTPUT_PROTOCOL_GEMINI，遵从度经多轮质询验证可接受。如果未来遵从度下降，需要回到此边界条件重新评估'
    status: resolved（定理类——有效性前提已从现有基础设施验证）
  - id: 270-2
    description: '实现需要在challenger调用方增加_check_stance_repetition(history)函数，约30-50行。检测逻辑在调用方而非Gemini内部，不改变审讯者角色定义（030a号保持）'
    status: resolved（v119-swarm stance-detector 工位完成，inquiry_loop.py +72行，9测试通过）
  - id: 270-3
    description: 'suspended:stance-repetition状态需要在ceremony中注册为工位跳过条件。后续ceremony遇到suspended工位时直接跳过，不重新发起审讯'
    status: resolved（v119-swarm suspended-state 工位完成，ceremony_state.py +74行 + ceremony_scan.py +20行）
---

# 270号：审讯者终止条件设计——stance-diff检测+编排者确认

## 1. 问题

v118 session 中，开放问题5经历四轮 Gemini 审讯。核心攻击（法医验尸报告 = D算子确认延迟）在每轮以不同名字重复出现（R1法医验尸报告、R2偷换概念、R3时间窗口失步、R4区间套实时性幻觉）。审讯者被锁在攻击者位置，没有终止能力。编排者手动喊停。

设计问题：审讯序列缺少收敛检测和终止条件。

## 2. RTAS 双模型对审

### Gemini decide：选项 B（编排者显式终止权）

推理链：
1. 保持审讯者作为纯否定源的角色纯净性（030a号）
2. 自动检测违反原则3（阈值否定）
3. 终止审讯 = 编排者（对象）否定审讯循环（对象）

边界条件：系统演化为无人值守时，需要独立"裁判agent"替代编排者。

### Codex review：选项 D（A+B 组合），评分 4.5/5

推理链：
1. 选项 B 形式化了已存在的能力（编排者已经在手动终止），不解决新问题
2. inquiry_loop.py 中已有 stance-diff 基础设施（check_same_subject_stable、compute_stance_diff），改动量仅 30-50 行
3. 检测逻辑在调用方而非 Gemini 内部 → 不违反 030a号（审讯者角色不变）、不违反 137号（不依赖 LLM 自判）
4. stance-diff 检测的是攻击的对象层面同构（Key 集合不变），是对象否定对象（005b号），不是数字阈值

Codex 否定了选项 C（审讯者内置 concede，评分 2/5）：改变角色定义 + RLHF 基底导致投降时机不可控。

关键设计修正：检测到 stance 重复后不直接终止，冻结为 `suspended:stance-repetition` 状态：
- 编排者在场 → 报告并等待确认（确认终止 或 覆盖继续）
- 编排者不在场 → 保持 suspended，后续 ceremony 跳过此工位

### 分歧点

| 维度 | Gemini | Codex |
|------|--------|-------|
| 优先价值 | 本体论纯净性 | 架构自主性 |
| stance-diff 性质 | 阈值否定（原则3违反） | 对象同构检测（005b号合法） |
| 编排者缺席 | 需要独立裁判agent | suspended 状态兜底 |

## 3. 编排者裁定

**选项 D（Codex 方案）**。

编排者理由：
1. v118 实际发生了审讯无限循环。选项 B 只是形式化"编排者可以喊停"——这个能力已经存在
2. Codex 的 suspended:stance-repetition 解决实际问题
3. 不改审讯者角色（030a号保持），不依赖 LLM 自判（137号保持）
4. 编排者缺席时不会无限循环
5. 基础设施已存在，改动量小

## 4. 设计规格

### 终止检测机制

连续 N=2 轮核心 stance Key 集合不变 → 检测为立场重复。

检测位置：challenger 调用方（ceremony 或手动循环中的包装函数），不在 Gemini 内部。

### 检测后行为

1. 冻结当前审讯——不再向 Gemini 发送下一轮 challenge
2. 审讯状态标记为 `suspended:stance-repetition`，附带重复 Key 列表
3. 编排者在场 → 报告并等待确认
4. 编排者不在场 → 保持 suspended，后续 ceremony 跳过此工位

### 不使用的机制

- 不使用超时（非对象否定，005b号禁止）
- 不使用轮次硬上限（超时变体）
- 不让审讯者自我判断（137号：LLM 不可靠）
- 不改变审讯者 system prompt（030a号保持）

## 5. 边界条件

1. stance 输出协议遵从度下降（Gemini 不输出 stance 块）→ 检测失效，需修复 prompt 或增加解析容错
2. Gemini 每轮用不同 Key 命名同一攻击 → 结构化比较失败，需增加语义层归一化
3. 攻击 Key 相同但 stance_value 有实质变化 → 不应判定为重复，需同时比较 Key 和 Value

## 6. 影响声明

### 修改定义
无。审讯者角色定义（030a号）不变。

### 待实现
1. `_check_stance_repetition()` 函数（270-2）
2. suspended 状态在 ceremony 中的注册（270-3）

### 不修改代码
本谱系为设计决策记录。代码实现由后续工位执行。

## 7. 谱系引用

- **269号**：OQ5四轮质询（终止条件缺失的发现场景）
- **030a号**：Gemini本体论位置（审讯者角色定义）
- **137号**：否定性禁令对行为执行层无效（RLHF基底约束）
- **005b号**：对象否定对象（stance-diff 的否定合法性来源）
- **069号**：递归拓扑异步自指蜂群（架构自主性依据）
