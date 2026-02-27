---
id: '228'
number: 228
title: 元观察——v77-swarm ceremony-step-guard 爆炸半径 + stagnation 诊断收敛 + 227号集成缺口部分修复
type: meta-rule
status: 生成态
date: 2026-02-27
source: meta-observer 二阶观察
session: v77-swarm
rule_version_baseline:
  claude_md_commit: "e56ebbcc"
  rules_dir_mtime: "2026-02-27 03:01:30 +0000"
depends_on:
  - '227'   # v70-swarm 元观察（ceremony_state.py 集成缺口）
  - '226'   # Lead 中断问题完整根因（三层无状态）
  - '036'   # 声明-能力一致性原则
---

# 228号：元观察——v77-swarm ceremony-step-guard 爆炸半径

## 观察对象

v77-swarm session：
- 仅一个 stagnation-auditor 工位运行，诊断 RTAS 停滞为正常间歇
- meta-observer（本工位）执行二阶观察
- topology-analyst 工位并行运行

## 观察 1（发散信号）：ceremony-step-guard 对非 Lead 工位产生误阻断

### 现象

meta-observer 的每次工具调用（Read、Grep、Glob、TaskList）都收到 ceremony-step-guard.sh 的 block 决策：

```
[ceremony-step-guard] 你正在 ceremony 步骤 6（consume）。立即继续执行下一步。
不允许回应用户消息、输出总结、或执行白名单外的操作。ceremony 序列不可被中断（224号谱系）。
```

### 根因

`.ceremony-step` 文件内容：`{"step": 6, "phase": "consume", "timestamp": "2026-02-27T07:33:06.166700+00:00"}`

ceremony-step-guard.sh 是全局 PostToolUse hook，对**所有 agent**（Lead + 工位）生效。hook 逻辑仅检查 `.ceremony-step` 文件是否存在（第29行 `[ -f "$STATE_FILE" ] || exit 0`），不检查当前 agent 身份。

因果链：
1. Lead 在 ceremony 步骤1 写入 `.ceremony-step`（227号修复方向已落地——ceremony.md 步骤1 现在调用 `ceremony_state.py write`）
2. Lead spawn 工位后进入步骤6（consume）
3. 工位开始执行，每次工具调用触发 ceremony-step-guard
4. hook 无法区分"Lead 的工具调用"和"工位的工具调用"
5. 所有工位收到误阻断信号

### 分类

036号模式（声明-能力一致性缺口）的延续。227号观察1 识别了"ceremony_state.py 无调用者"——现在调用者补上了（ceremony.md 步骤1），但 guard 的作用域设计有缺陷：它无法区分 agent 身份。

这是 227号修复的**二阶后果**：修复了"从不写入"，暴露了"写入后对谁生效"。

### 严重程度

中等。当前 hook 返回 `{"decision": "block", ...}` 但 Claude Code 平台对 PostToolUse hook 的 block 决策处理方式是：将 reason 作为 system-reminder 注入上下文，但**不实际阻止工具执行**（工具结果已产生）。因此工位功能未受影响，但：
1. 每次工具调用注入约200 token 噪声到工位上下文
2. 上下文消耗加速，工位可用推理长度缩短
3. 工位可能被误导执行 ceremony 步骤（如果工位未正确忽略不属于自己的指令）

### 修复方向（行动类）

ceremony-step-guard.sh 需要增加 agent 身份过滤。两种方案：

**方案A**（最小改动）：hook 检查环境变量或 INPUT_JSON 中的 agent 标识，仅对 Lead 生效。
```bash
# 在 [ -f "$STATE_FILE" ] || exit 0 之后增加：
AGENT_NAME=$(echo "$INPUT" | "$PYTHON_BIN" -c "import json,sys; print(json.loads(sys.stdin.read()).get('agent_name',''))" 2>/dev/null || echo "")
[ "$AGENT_NAME" = "team-lead" ] || exit 0
```

**方案B**（依赖 Claude Code 平台能力）：如果 PostToolUse hook 的 INPUT_JSON 不携带 agent_name，则 hook 无法区分——这是平台限制（226号类型B的又一实例）。此时需要改变策略：ceremony_state.py 在步骤5（spawn）后 clear，步骤7（push）前重新 write。这样工位执行期间 `.ceremony-step` 不存在。

### 与 226号/057号的关系

226号分类了三类 Lead 中断：
- 类型A（Bash后断裂）→ hook 可修
- 类型B（纯文本后断裂）→ 平台限制
- 类型C（角色边界僭越）→ 规则层

本观察揭示的是**类型D**（hook 误伤非目标 agent）——226号未覆盖的新类型。hook 系统的设计假设是"一个 session 一个 agent"，但蜂群架构下"一个 session 多个 agent"是常态。

## 观察 2（收敛信号）：stagnation-auditor 诊断质量

v77-swarm 的 stagnation-auditor 正确诊断了 0620→0724 的停滞为正常间歇。诊断逻辑：
1. 检查 pending 目录（空）
2. 检查 227号下游推论（已 resolved）
3. 检查 v78-swarm 状态（已启动）
4. 检查 git log（v71-v76 commit 产出正常）

这与 217号观察3（stagnation 检测语义问题）形成对比：217号指出 ceremony_scan 的自动检测仅以谱系增量为指标，会误报。v77-swarm 的 stagnation-auditor 是人工工位，能做更完整的判断。**收敛信号**——自动检测的不足由人工工位补偿。

## 观察 3（收敛信号）：227号集成缺口已部分修复

227号观察1 指出 ceremony_state.py write_step 无调用者。当前状态：
- ceremony.md 步骤1 已包含 `python scripts/ceremony_state.py write 1 initial`（修复落地）
- `.ceremony-step` 文件在 v77-swarm 中确实存在（证明 write 被调用了）
- 但 clear 路径可能未完全覆盖——文件残留说明 ceremony 终止时未 clear

从"完全空操作"到"写入已工作但清理和作用域有缺陷"——进步了一步但未完成。

## 观察 4（收敛信号）：蜂群规模收缩模式

v77-swarm 仅产出3个工位（stagnation-auditor + topology-analyst + meta-observer），全部是结构性/审计性工位，无域推进工位。这与 v78-swarm（4个域推进工位）形成互补。

模式：当 ceremony_scan 检测到无新概念分离但有审计需求时，产出审计蜂群而非推进蜂群。这符合 RTAS 的"概念收敛 + 工程推进"阶段预期（227号下游推论3）。

## 自环检查

| 历史观察 | v77 状态 | 判定 |
|---------|---------|------|
| 227§1：ceremony_state.py 集成缺口 | 部分修复——write 已工作，clear 和作用域有缺陷 | 发散——修复引入新问题 |
| 227§3：ceremony_push_and_rescan.sh 是137号工程范式 | 未直接触发 | 维持 |
| 219§1：语言封闭性违反 | 未触发（v77无域推进） | 维持待观察 |
| 217§1：并行工位隐式依赖 | 未触发（v77无API变更） | 维持待观察 |
| 217§3：stagnation 检测语义问题 | 人工工位补偿了自动检测不足 | 收敛 |
| 216§4：任务粒度默认偏小 | v77工位粒度适中（审计型） | 收敛 |

## 下游推论

1. 观察1（ceremony-step-guard 爆炸半径）是行动类，需要在 ceremony-step-guard.sh 中增加 agent 身份过滤或调整 `.ceremony-step` 的生命周期
2. 需要确认 Claude Code PostToolUse hook 的 INPUT_JSON 是否携带 agent 标识——如果不携带，方案A 不可行，必须走方案B
3. 227号下游推论1（ceremony_state.py 集成缺口是行动类）的状态从"已修复"细化为"部分修复——write 落地，clear/scope 待修"

## 边界条件

- 如果 Claude Code PostToolUse hook 的 INPUT_JSON 携带 agent_name 或类似字段，观察1可通过方案A低成本修复
- 如果不携带，则需要方案B（调整 `.ceremony-step` 生命周期），复杂度更高
- 如果 `.ceremony-step` 残留是因为 ceremony 正在进行中（Lead 尚未到达终止路径），则 clear 缺失的判断需要修正为"ceremony 仍在执行"
