---
id: '218'
number: 218
title: Lead 并行化——割掉 RTAS 的串行尾巴
type: 语法记录
status: 已结算
date: 2026-02-26
source: 编排者观察 + Gemini/Codex 双向诊断
session: v58-swarm
depends_on:
  - '069'   # 递归拓扑异步自指蜂群
  - '090'   # 严格性永久化为蜂群语法规则
  - '143'   # commit 后总结是 RLHF 停顿点
  - '217'   # v56-swarm 元观察（并行工位隐式依赖）
---

# 218号：Lead 并行化——割掉 RTAS 的串行尾巴

## 核心命题

Lead 是 RTAS 的一环，不是 RTAS 之外的串行瓶颈。Lead 的一切操作遵守与工位相同的并行原则——独立操作一律并行，只有严格数据依赖才允许串行。

## 编排者洞见

1. "team lead 还有串行的功能，那么 lead 作为 RTAS 最特殊的一环，却并不是严格 RTAS 的，这就是小尾巴"
2. "像 Claude Code agent 用 30 行 bash 把自己调用出来的方式，让 team lead 只变成一种形式。这样做才叫递归拓扑"
3. "这样结晶才是有用的，如果不是这种纯粹的自举形式，结晶没有意义，因为这依赖记忆而不是结晶"

## 诊断结果（Gemini + Codex 一致）

### RLHF 串行惯性（已识别 5 种）
1. 先总结再行动（143号已识别）
2. 逐个检查工位状态（应批量 TaskList + 批量处理）
3. 等待测试完成再 commit（Lead 自己跑 pytest）
4. 先全关工位再开始下一轮（shutdown 和 re-scan 可重叠）
5. 等待确认再 re-scan

### 可并行化（3 项）
1. pytest 内嵌于 ceremony_scan.py（最严重，阻塞 120s）→ 提取为独立测试工位
2. push ∥ ceremony_scan：无数据依赖
3. shutdown ∥ re-scan：流水线化

### 必须串行（2 项）
1. git add → commit → push：git 硬依赖
2. ceremony_scan 输出 → spawn 工位：数据依赖

## 落地变更

1. `.claude/commands/ceremony.md` 步骤6 重写：并行优先，批量处理，Lead 不空转
2. `scripts/ceremony_scan.py`：移除内嵌 pytest，改为顶层 `test_verification_needed` 信号
3. 新增 `.claude/rules/lead-parallel-dispatch.md`：禁止 RLHF 串行惯性

## Lead 最小自举形式（目标，未完全落地）

```
scan → TeamCreate → spawn_all → consume → persist → re-scan → TeamDelete
```

Lead 的形式本身就是行为——不依赖记忆长文档，而是从最小形式中自然涌现。
ceremony.md 的最终目标是缩减到 ~25 行伪代码级别。

## 边界条件

- 创世 Gap 最小残余（不可委托）：scan 调用、TeamCreate、spawn、git commit/push、TeamDelete
- gangju_analysis → ceremony_scan 有数据依赖（gangju 写 pending，scan 读 pending），不可并行

## 下游推论

1. ~~ceremony_scan.py 应扩展为输出所有工位（业务+结构），让 Lead 只做 spawn_all~~ → **resolved**（commit 3412cfa：添加 --phase 参数 + 结构工位推导）
2. ~~ceremony.md 应进一步缩减到自举形式~~ → **resolved**（commit 7120720：重写为 65 行）
3. Codex API 不可用时不阻塞蜂群——Gemini 单向质询可作为降级方案
