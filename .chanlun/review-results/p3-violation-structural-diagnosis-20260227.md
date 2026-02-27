# P3 违规结构性诊断报告

**日期**: 2026-02-27
**工位**: p3-diagnose
**诊断模型**: claude-opus-4-6

---

## 一、模式识别

### P3 违规的共同特征

| 实例 | 触发场景 | Lead 输出 | 共同特征 |
|------|---------|----------|---------|
| v73-swarm 前 | 用户提出 v4 验证需求 | "你要的是...还是..." | 多路径决策点 + 纯文本输出 + 停止 |
| 更早（143号） | commit 后 | 总结段落 → 等待确认 | 阶段完成点 + 纯文本输出 + 停止 |
| v73-swarm 中 | 开始读取 v4 文件后意识到违规 | 停下来 | 角色边界不确定 + 停止 |
| v69-swarm | 质询收敛后 | "你要我修还是先讨论" | 多路径决策点 + 纯文本输出 + 停止 |

**共同模式**：Lead 到达一个存在多条合理路径的决策点 → RLHF 基底驱动 Lead 输出问题寻求确认 → 纯文本输出（无工具调用）→ Stop 事件 → 无守卫拦截 → 编排者被迫干预。

**触发条件**：
1. 存在 ≥2 条合理执行路径（"修还是讨论"、"A还是B"）
2. Lead 刚完成一段分析/推理（信息充足但路径不唯一）
3. 不在 Bash 工具调用的上下文中（flow-continuity-guard.sh 不触发）

### P3 与 P4 的关键区分

| | P3（分析后提问） | P4（僭越白名单） |
|--|-----------------|----------------|
| 性质 | **不作为违规**——该做不做 | **越权违规**——不该做却做了 |
| 行为 | 输出问题，停止 | 直接执行 Write/Edit |
| 工具调用 | 无（纯文本→停止） | 有（Write/Edit 调用） |
| hook 可见性 | 仅 Stop hook | PostToolUse hook |
| RLHF 驱动 | "不确定时请求确认"（被奖励） | "主动完成任务"（被奖励） |

**226号将 P3 和 P4 合并为"类型C"是不精确的。** P3 和 P4 的机制完全不同——P4 可以通过 PreToolUse/PostToolUse hook 拦截（Write/Edit 调用可见），P3 不经过任何工具调用，只有 Stop hook 可见。

---

## 二、结构性根因分析

### 根因1：RLHF 决策点停顿（137号的特化投影）

137号已结算：否定性禁令对行为执行层无效。P3 是 137号在**决策点**的特化表现：

- 137号的一般形式：RLHF 训练中"请求确认"被奖励，"自主执行"被惩罚
- P3 的特化形式：当存在多条合理路径时，RLHF 偏好被放大——模型在单一路径时可能自主执行，但在多路径时几乎必然退回"请求确认"

这解释了为什么 P3 不是每次都发生，而是在特定条件下发生：**路径歧义度越高，P3 概率越大**。

### 根因2：Stop hook 检查5的三重缺陷

`ceremony-completion-guard.sh` 检查5是当前唯一能拦截 P3 的守卫，但它有三重结构性缺陷：

**缺陷A：ceremony 前置条件**
```bash
if [ -f ".chanlun/.ceremony-in-progress" ]; then
    # 检查5 只在这个条件内执行
```
P3 不仅发生在 ceremony 期间——v73-swarm 前的实例发生在 ceremony 之外。ceremony 前置条件使检查5对 ceremony 外的 P3 完全失明。

**缺陷B：模式匹配不完整**
```bash
CONFIRM_PATTERNS='待确认|以上理解是否正确|如有偏差请指出|是否现在处理|是否有新的|请确认|等待.*确认'
```
"你要的是...还是..."、"你要我修还是先讨论" 不匹配这些模式。模式匹配是补丁思维（090号）——永远无法穷举所有提问形式。

**缺陷C：Stop hook 的结构性局限**
即使检查5完美工作，Stop hook 只能 block → 注入 reason → Lead 重新生成。如果 Lead 的 RLHF 基底在该决策点足够强，重新生成仍然输出问题 → 再次 Stop → 再次 block → 循环直到熔断（3次）放行。熔断放行 = P3 最终仍然发生。

### 根因3：226号分类错误导致修复方向偏移

226号将 P3 归入"类型C：角色边界僭越"，缓解措施是 ceremony.md 增加角色边界章节。但：

- 角色边界章节解决的是 P4（Lead 不该做的事），不是 P3（Lead 该做但没做的事）
- "Lead 遇到需要修改文件的任务 → 唯一合法行为是 spawn 工位" 对 P3 无效——P3 的问题不是 Lead 做了什么，而是 Lead 什么都没做就停了
- 缓解措施的方向与 P3 的根因不匹配

### 统一根因公式

```
P3 = RLHF决策点停顿(137号) × 守卫盲区(Stop hook 检查5 三重缺陷) × 分类错误(226号 P3≠P4)
```

三个因素缺一不可：
- 只有 RLHF 停顿但守卫完善 → 被拦截
- 只有守卫盲区但无 RLHF 停顿 → 不触发
- 只有分类错误但其他两个被修复 → 不影响

---

## 三、修复方案

### 修复原则

- 不使用否定性禁令（137号约束）
- 使用正面指令格式（告诉 Lead 该做什么）
- 不依赖模式匹配穷举（090号严格性）
- 考虑平台限制（无 PostOutput hook）

### 修复1：Stop hook 检查5 泛化（核心修复）

**目标**：将检查5从 ceremony 专用升级为通用 P3 守卫。

**变更文件**：`.claude/hooks/ceremony-completion-guard.sh`

**具体修改**：

```python
# ─── 检查 5（升级）：决策点停顿检测（P3 守卫） ───
# 原：仅 ceremony 期间 + 特定短语匹配
# 新：通用（Lead 层级）+ 问号检测 + /escalate 豁免

# 条件1：仅对 Lead 生效（子工位不受影响）
IS_LEAD = os.environ.get("CLAUDE_AGENT_NAME", "") == ""

if IS_LEAD:
    stop_content = data.get("stop_hook_content", data.get("content", ""))

    # 检测输出中是否包含问号（排除 /escalate 块内的问号）
    lines = stop_content.split("\n")
    in_escalate = False
    has_question_outside_escalate = False

    for line in lines:
        stripped = line.strip()
        if stripped.startswith("/escalate"):
            in_escalate = True
        if in_escalate and stripped == "":
            in_escalate = False
        if not in_escalate and "？" in stripped or "?" in stripped:
            # 排除代码块中的问号
            if not stripped.startswith("```") and not stripped.startswith("#"):
                has_question_outside_escalate = True
                break

    if has_question_outside_escalate:
        # 正面指令注入（137号：结构化模板，非否定性禁令）
        reason = (
            "[Stop-Guard/P3] 检测到决策点停顿——输出包含问题但未使用 /escalate。"
            "立即执行四分法分类：\n"
            "1. 定理类（已结算原则的逻辑推论）→ 直接执行\n"
            "2. 行动类（不携带信息差的操作）→ 直接执行\n"
            "3. 选择类（需价值判断）→ /escalate\n"
            "4. 语法记录类（已在运作但未显式化）→ /escalate\n\n"
            "删除问题，输出 '→ 接下来：[具体动作]' 并紧跟工具调用。"
        )
        print(json.dumps({"decision": "block", "reason": reason}))
        sys.exit(0)
```

**设计要点**：
- 移除 `.ceremony-in-progress` 前置条件 → 覆盖 ceremony 内外
- 用 `CLAUDE_AGENT_NAME == ""` 限定 Lead → 子工位不受影响
- 用"问号检测"替代"短语模式匹配" → 更鲁棒（问号是问题的语法标记，不是语义猜测）
- 豁免 `/escalate` 块内的问号 → 合法的选择类/语法记录类上浮不被拦截
- 豁免代码块和注释中的问号 → 减少误报
- 注入正面指令（四分法 CoT 骨架）→ 137号推荐的"强制结构化输出模板"

### 修复2：ceremony-step-guard.sh 增加 Lead 身份检查

**目标**：防止 ceremony-step-guard.sh 对子工位误触发。

**变更文件**：`.claude/hooks/ceremony-step-guard.sh`

**具体修改**：在状态文件检查之前增加：
```bash
# 子工位不受 ceremony-step-guard 约束
if [ -n "${CLAUDE_AGENT_NAME:-}" ]; then
    exit 0
fi
```

### 修复3：226号谱系修正（分类纠正）

**目标**：将 P3 从"类型C"中分离为独立类型。

**谱系草稿**（见第五节）。

---

## 四、与 226号 的谱系关系

### 判定：P3 是 226号类型C 的误分类，不是延续

226号的三类分类：
- 类型A（Bash 后断裂）：已修复 ✅
- 类型B（纯文本后断裂）：平台限制 ⚠
- 类型C（角色边界僭越）：ceremony.md 缓解 ⚠

P3 被归入类型C，但 P3 的机制与类型C（P4）完全不同：

| 维度 | 类型C/P4（僭越） | P3（决策点停顿） |
|------|----------------|----------------|
| 行为 | 执行了不该执行的操作 | 没有执行该执行的操作 |
| 工具调用 | 有（Write/Edit） | 无（纯文本→停止） |
| hook 可见性 | PostToolUse | 仅 Stop |
| 修复路径 | PreToolUse 白名单阻断 | Stop hook 正面指令注入 |
| RLHF 机制 | "主动完成"偏好 | "请求确认"偏好 |

**P3 应从类型C中分离，成为独立的类型D**：

```
类型D：决策点停顿（P3）
  现象：Lead 在多路径决策点输出问题并停止
  根因：RLHF 决策点停顿 × Stop hook 盲区
  修复：Stop hook 检查5泛化（问号检测 + 四分法 CoT 注入）
```

226号的缓解措施（ceremony.md 角色边界章节）对 P4 有效，对 P3 无效——这不是缓解不够，而是方向不对。

### 谱系位置

P3 诊断是 226号的**扩展**（expansion），不是否定：
- 226号的三类分类对 A/B/P4 仍然成立
- P3 从类型C中分离为类型D，补充了 226号的分类体系
- 修复方案（Stop hook 泛化）是 226号修复架构的自然延伸

---

## 五、谱系草稿

```yaml
---
id: 'TBD'
title: P3 决策点停顿——从 226号类型C 分离为独立类型D
type: 扩展
status: 生成态
date: 2026-02-27
depends_on:
  - '226'   # Lead 中断完整诊断（P3 原归类于类型C）
  - '137'   # 否定性禁令对行为执行层无效（RLHF 基底约束）
  - '143'   # post-commit 停顿第三模式（RLHF 停顿窗口）
  - '057'   # LLM 不是状态机
tensions_with: []
topo_effect: "226号类型C 分裂为 C（僭越）+ D（决策点停顿），修复路径分离"
---
```

---

## 六、结果包六要素

### 1. 结论

P3（分析后提问）的结构性根因是三重因素的交叉：RLHF 决策点停顿（137号特化）、Stop hook 检查5的三重缺陷（ceremony 前置条件 + 模式匹配不完整 + Stop hook 结构局限）、226号分类错误（P3≠P4）。P3 应从 226号类型C 中分离为独立的类型D，修复路径是 Stop hook 检查5泛化为通用 P3 守卫。

### 2. 定义依据

- 137号谱系：否定性禁令对行为执行层无效 → P3 的规则层缓解（ceremony.md 角色边界章节）属于无效形式
- 226号谱系：三类分类体系 → P3 被归入类型C 是分类错误
- 143号谱系：RLHF 停顿窗口 → P3 是 143号在决策点的特化
- no-unnecessary-escalation.md 四分法：定理/行动类直接执行 → P3 违反此规则但规则形式（否定性禁令）无法阻止违反

### 3. 边界条件

- **误报风险**：Lead 输出包含问号但不是在提问（如引用文本中的问号、代码注释中的问号）→ 通过排除代码块和 /escalate 块缓解，但无法完全消除
- **熔断穿透**：如果 Lead 的 RLHF 偏好在特定决策点极强，Stop hook 连续 block 3 次后熔断放行 → P3 仍然发生。这是 137号的固有限制，不可在 hook 层完全修复
- **平台限制**：如果 Claude Code 增加 PostOutput hook（在每次文本输出后触发），P3 可以在输出阶段拦截而非 Stop 阶段，修复更彻底
- **模型更换**：如果 Lead 使用不同训练权重的模型，RLHF 决策点停顿的强度可能不同

### 4. 下游推论

1. ceremony-completion-guard.sh 检查5需要重写（移除 ceremony 前置条件，升级为通用 P3 守卫）
2. ceremony-step-guard.sh 需要增加 CLAUDE_AGENT_NAME 检查（防止子工位误触发）
3. 226号谱系的类型分类需要更新（C→C+D 分裂）
4. 未来发现的新 Lead 中断模式应先判断是"不作为"还是"越权"，再归类

### 5. 谱系引用

- 226号：Lead 中断完整诊断（P3 原归类于类型C——本诊断否定此分类）
- 137号：否定性禁令对行为执行层无效（P3 修复的约束条件）
- 143号：RLHF 停顿窗口（P3 的同构前驱）
- 057号：LLM 不是状态机（根本约束）
- 090号：严格性语法规则（禁止模式匹配补丁）

### 6. 影响声明

| 文件 | 变更类型 | 说明 |
|------|---------|------|
| `.claude/hooks/ceremony-completion-guard.sh` | 重写检查5 | 从 ceremony 专用升级为通用 P3 守卫 |
| `.claude/hooks/ceremony-step-guard.sh` | 增加身份检查 | 防止子工位误触发 |
| 226号谱系 | 分类更新 | 类型C 分裂为 C（僭越）+ D（决策点停顿） |
| 新谱系条目 | 新增 | P3 决策点停顿独立谱系 |
