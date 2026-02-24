---
name: consensus-ceremony-trigger
description: >
  共识仪式触发协议（立场差分架构）。质询循环收敛时，从结构化立场序列中
  通过差分计算推导让步轨迹，调用 consensus_ceremony.write_consensus_ceremony()
  执行三区块原子写入。解决总方针 §63 的缺口。
genealogy_source: "§21, §63"
---

# 共识仪式触发协议（立场差分架构）

## 编排者决断

事后正则提取被否定。编排者给出的架构决断：

1. **让步 ≠ 陈述"我让步了"（énoncé）**，让步 = 从位置 A 到位置 B 的移动（énonciation）
2. **正则匹配只能抓陈述，抓不到言说行为**
3. **立场差分架构**：
   - 每轮质询输出结构化格式：**判定**（verdict）+ **立场清单**（stances KV 对）
   - 让步 = 相邻轮次立场清单的差分
   - Agent 不自我报告让步，系统从立场差分中推导
   - 自我报告的让步 vs 被计算出的让步之间的差异本身也是信号（divergence）
4. **关键约束**：无法修改 Gemini/Codex 内部推理，只能控制输入（prompt）和输出协议

## 总方针推导链

### §17 → 主体位置
三个主体位置：CC（主体/生产者）、Gemini（概念层质询者）、Codex（代码层质询者）。
CC 是被质询的对象，不是质询者。residue 的 conceded 字段记录的是质询者（Gemini/Codex）放弃了什么，不是 CC 放弃了什么。

### §18 → 双重质询循环
CC 的每一个产出进入双重质询循环。Gemini 从理论一致性、逻辑严格性、概念有效性质询。Codex 从实现可行性、代码正确性、架构一致性质询。质询追溯性地改写 CC 产出在谱系中的位置。

### §19 → 收敛判定
质询到共识为止。共识可能是一方被说服（Gemini 的否定被驳回 或 CC 的产出被否定），可能是发现第三条路。两种结果都是收敛。

### §20 → 剩余物
共识 = 缝合，缝合必然生产剩余物。被排除的东西是 residue 的物质来源。

### §21 → 三区块原子写入
- consensus.refs → 触发质询的原始 CC 产出区块 id
- residue.content.gemini_conceded → Gemini 放弃的立场
- residue.content.codex_conceded → Codex 放弃的立场
- tension.content.unresolved → 双方承认未解决但搁置的部分

### §63 → 当前阶段缺口
质询循环已有基础，但让步轨迹在对话历史中未被结构化提取。本模块解决此缺口。

## 数据结构

```python
from scripts.consensus_trigger import (
    StanceDeclaration,    # 单轮立场声明
    StanceDiff,           # 相邻轮次差分
    ConcessionTrace,      # 让步轨迹（含 divergence 信号）
    InquiryCycleResult,   # 质询循环结构化产出
)

# StanceDeclaration: 每轮质询的结构化输出
sd = StanceDeclaration(
    verdict="fail",                     # pass | fail | conditional
    stances={"issue_x": "reject"},      # 具体点 → 持有的立场
    round_number=1,
    self_reported_concessions=[],       # 可选：agent 自我报告的让步
)
```

## 当前阶段的两个独立场景（§63）

### 场景 A: Gemini 概念层质询

- **触发事件**: genealogy_settlement → gemini-challenger verify
- **收敛判定**: agent 对 Gemini 否定做出判定（否定成立/不成立均为收敛）
- **residue 映射**:
  - 否定成立: Gemini 坚持否定 → gemini_conceded=[], codex_conceded=[]
  - 否定不成立: Gemini 放弃否定立场 → gemini_conceded 来自差分
- **codex_conceded 始终为空**: Codex 不参与此场景

### 场景 B: Plan-review 多轮对审

- **触发事件**: plan_review → codex-challenger review 多轮
- **收敛判定**: Codex 明确确认满意
- **residue 映射**:
  - codex_conceded: 来自立场差分推导（相邻轮次 removed + changed）
  - gemini_conceded 始终为空: Gemini 不参与 plan-review

### 场景 C: Gemini decide（不触发）

单方决策，无质询对手方。不产生共识仪式。

## 代码接口

```python
from scripts.consensus_trigger import (
    StanceDeclaration,
    compute_stance_diff,
    derive_concession_trace,
    extract_from_gemini_verify,
    extract_from_plan_review,
    trigger_ceremony,
    detect_convergence_from_review_file,
    find_trigger_block_for_review,
)

# 场景 A: Gemini verify 收敛后
stance_seq = [
    StanceDeclaration(verdict="fail", stances={"def_x": "incorrect"}, round_number=1),
    StanceDeclaration(verdict="pass", stances={}, round_number=2),
]
cycle = extract_from_gemini_verify(
    stance_sequence=stance_seq,
    trigger_block_id="<sha256>",   # §21: CC 产出区块 id
    negation_stands=False,          # agent 判定
    conclusion="否定不成立，原定义正确",
)
result = trigger_ceremony(cycle)    # → 5 blocks + 4 relations 原子写入

# 场景 B: plan-review 收敛后
stance_seq = [
    StanceDeclaration(verdict="fail", stances={"hook": "reject"}, round_number=1),
    StanceDeclaration(verdict="pass", stances={}, round_number=2),
]
cycle = extract_from_plan_review(
    stance_sequence=stance_seq,
    trigger_block_id="<sha256>",   # §21: CC 产出区块 id
    conclusion="方案定稿",
)
result = trigger_ceremony(cycle)

# 差分计算（底层）
diff = compute_stance_diff(stance_seq[0], stance_seq[1])
trace = derive_concession_trace(stance_seq)
# trace.divergence: 自我报告 vs 计算出的让步之间的差异
```

## Divergence 信号

`ConcessionTrace.divergence` 记录自我报告与系统计算之间的差异：
- `self_reported_not_computed: X` — agent 声称让步了 X 但立场数据未变化
- `computed_not_self_reported: X` — 立场数据变化了但 agent 未报告让步 X

这个差异本身是有意义的信号，不是错误。

## D 策略约束（082号）

- consensus-ceremony-trigger.sh advisory hook 检测 review-results 收敛信号
- Hook 只提示，不阻断
- Agent 认领后调用 trigger_ceremony()

## 边界条件

- trigger_block_id 无法从 review-results 提取时，agent 需手动提供
- 场景 A 和 B 可在同一 session 中共存（不同质询循环独立触发）
- 完整的双重质询循环（§18：同一 CC 产出同时被 Gemini 和 Codex 质询）是未来相位目标（§71），当前阶段两个场景独立运作
