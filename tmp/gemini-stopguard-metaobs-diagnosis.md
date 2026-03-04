# Gemini 诊断：Stop-Guard 错误循环 + meta-observer advisory 噪音

**日期**: 2026-02-23
**诊断者**: Gemini 异质质询工位（由 Claude Opus 4.6 代行）
**输入**: discuss-stopguard-metaobs-ctx.md + ceremony-completion-guard.sh + meta-observer-guard.sh

---

## 问题 A：谁的职责更新 task 状态？

### 判定：工位自己（选项1）是唯一严格正确的责任者

### 论证

三个候选者的分析：

1. **工位自己**：工位是任务执行者，只有它知道任务何时完成。`TaskUpdate(taskId, status=completed)` 应该是工位最后一个动作（在 SendMessage 之前或紧随其后）。这是**对象否定对象**（005b 号同构）：完成状态是工位自身对"进行中"状态的否定。

2. **Lead**：Lead 收到结果后更新——这创造了**时序竞态**。Lead 可能在 shutdown_request 和 TaskUpdate 之间被 Stop-Guard 阻断，导致任务状态永远无法更新。Lead 更新 task 状态是一种**代理责任转移**，不严格。

3. **系统（TeamDelete）**：TeamDelete 是清理操作，不是状态转换。任务从 in_progress 到 completed 是语义事件，TeamDelete 的批量清理是**物理清理**。混淆两者 = 用垃圾回收替代逻辑完成。

**但关键问题是**：Claude Code Agent SDK 中，工位（teammate）完成工作后的行为模式是：
- 完成工作 → SendMessage 向 Lead 汇报 → 进入 idle 等待
- idle 状态下工位不会主动调用 TaskUpdate

这不是工位"忘记"——而是 Claude Code Agent SDK 的设计模式中，**SendMessage 和 TaskUpdate 之间没有强制的因果链**。工位认为"发了消息 = 交付了"，但 task.json 不知道消息已发送。

### 修复方案

**在工位 prompt 层面无法可靠解决**（016 号谱系：规则没有代码强制就不会被执行）。必须在 Stop-Guard 层面增加防御。见问题 B。

---

## 问题 B：Stop-Guard 是否应该检查工位存活性而非仅看 task 状态？

### 判定：是。这是正确的修复方向

### 论证

当前 Stop-Guard（ceremony-completion-guard.sh:90-165）的检查逻辑：

```
扫描 $HOME/.claude/tasks/*/  → 统计 pending + in_progress  → ACTIVE_TASKS > 0 → 阻断
```

这个逻辑的隐含假设是：**task 状态与工位存活性是同步的**。但实际上它们是**异步的**——工位可以已经退出，而 task.json 仍然是 in_progress。

修复路径：**交叉验证 task 状态与 team members 列表**。

具体逻辑：

```
对每个 in_progress 的 task：
  1. 读取 task 的 owner 字段
  2. 检查 team config 的 members 列表中该 owner 是否存在
  3. 如果 owner 不存在（工位已退出）→ 该 task 是僵尸任务 → 不计入 ACTIVE_TASKS
  4. 如果 owner 存在 → 任务仍活跃 → 计入 ACTIVE_TASKS
```

**但存在一个边界问题**：team config 的 members 列表在 TeamDelete 之前不会更新。工位退出（shutdown 完成）后，members 列表中仍有该工位——直到 Lead 调用 TeamDelete。

因此需要更底层的检查：**进程存活性**。但 Claude Code Agent SDK 不暴露进程 PID 给 hooks。

**替代方案**：在 Stop-Guard 中，对 in_progress 任务增加**时间衰减**检查——如果 task 的 last_updated 时间超过阈值（如 5 分钟），且 team members 中该 owner 的 backendType 是 "in-process"（本地进程），则视为僵尸。

但 task.json 没有 `last_updated` 字段。

**最终修复方案**：见下方"综合修复方案"节。

---

## 问题 C：熔断是否是正确的止损机制？

### 判定：熔断是正确的止损机制，但阈值（5次）不够智能

### 论证

熔断的存在论位置：**它是 Stop-Guard 自身不可判定性的承认**。Stop-Guard 无法区分"真的有活跃任务需要完成"和"僵尸任务永远不会完成"。熔断 = 对不可判定性的有限等待后放弃。这是正确的。

但固定 5 次是**任意常数**，不考虑实际状态变化。当前逻辑：

```bash
if [ "$COUNT" -ge 5 ]; then
    rm -f "$COUNTER"
    exit 0  # 放行
fi
```

问题：
1. 如果有真实的活跃任务（3 个工位在跑），5 次阻断后放行 = 过早放弃
2. 如果全是僵尸任务（0 个工位在跑），5 次阻断 = 5 次无意义等待

**智能阈值方案**：

```
IF 连续 N 次阻断 AND 阻断原因完全相同（task 列表没有任何状态变化）THEN 放行
ELSE 重置计数器
```

当前逻辑已经有隐式的"无状态变更"判断（计数器只在阻断时递增，不检查状态变更）。但它没有**显式检查**状态是否变化。如果两次阻断之间有任务完成了（ACTIVE_TASKS 从 3 变成 2），应该重置计数器。

### 修复方案

将计数器文件从纯数字改为包含上次 ACTIVE_TASKS 数量：

```
# .chanlun/.stop-guard-counter 格式：
# COUNT:LAST_ACTIVE_TASKS
```

如果 ACTIVE_TASKS 发生了变化，重置 COUNT。这样：
- 状态在变化（有任务在完成）→ 永远不熔断，持续等待
- 状态停滞（僵尸任务）→ 快速熔断（2-3 次即可）

---

## 问题 D：advisory 模式是"降级"还是"失效"？

### 判定：完全失效。不是降级

### 论证

"降级"的定义：功能仍在运作，但力度降低。例如从"强制阻断"降级为"弱提示"——前提是弱提示确实被接收并有一定概率被执行。

当前 meta-observer-guard.sh 的 advisory 模式（第 44-57 行）：

```bash
if [ "$STRICT_MODE" != "1" ]; then
    echo "$CURRENT_SESSION" > "$MARKER"   # 自动落标（假装已执行）
    # 输出 advisory systemMessage
    python -c "... {'continue': True, 'systemMessage': '...'} ..."
    exit 0
fi
```

这段代码做了两件事：
1. **自动落标**（echo "$CURRENT_SESSION" > "$MARKER"）：告诉系统"二阶观察已执行"——但实际没有执行
2. **输出 advisory**：一段 systemMessage 告诉 Lead "可以手动执行"

问题 1（自动落标）：**这是谎言**。MARKER 的语义是"本 session 已执行二阶观察"。自动落标 = 在没有执行的情况下标记为已执行。后续任何基于 MARKER 的检查都会得出错误结论。

问题 2（advisory 输出）：087 号谱系修复了原始 hotfix 的"exit 0 无输出"问题，现在确实产生了 systemMessage。但实践证据表明 **Lead 从未基于此 advisory 主动执行二阶观察**。

因此：
- 自动落标 = 伪造执行记录（失效）
- advisory 被忽略 = 提示无效果（失效）
- 二者叠加 = **完全失效，不是降级**

087 号谱系声称"advisory 模式产生实际的 systemMessage 提示（D策略）"——这只修复了"无输出"到"有输出"的技术问题，但没有解决"输出被忽略"的行为问题。D 策略（hooks 提示 + Lead 认领）的前提是 Lead 会认领。如果 Lead 从不认领，D 策略 = 死信。

---

## 问题 E：上游平台错误修复后，应该恢复 STRICT_MODE=1 吗？

### 判定：应该恢复，但需要附加熔断保护

### 论证

"Invalid signature in thinking block" 是 Claude Code 平台的上游 bug，与 meta-observer-guard.sh 的逻辑无关。016 号谱系的核心论断是"规则没有代码强制就不会被执行"——STRICT_MODE=0 等于取消了代码强制，让 meta-observer 回到了 016 号描述的"概念层规则无 runtime 强制"状态。

恢复 STRICT_MODE=1 的条件分析：

1. **如果上游 bug 已修复**：直接恢复 STRICT_MODE=1。当前 meta-observer-guard.sh 的 STRICT_MODE=1 路径（第 59-75 行）已经有完整的逻辑：检查 MARKER → 阻断一次 → 落标后放行。熔断次数是 1（只阻断一次），这是合理的——二阶观察是观察性的，不阻塞关键路径。

2. **如果上游 bug 未修复**：STRICT_MODE=1 的阻断会触发 Lead 执行二阶观察，观察过程可能触发 thinking block bug。此时需要在 meta-observer agent 的 prompt 中显式禁用 extended thinking，或者用替代触发方式（见问题 F）。

**验证上游 bug 是否修复**：无法在 hook 中自动判断。建议设置 `META_OBSERVER_GUARD_STRICT=1` 环境变量进行一次手动测试。如果 session 正常完成（无 "Invalid signature" 错误），则确认修复，永久恢复。

### 修复方案

1. 先手动测试 STRICT_MODE=1（设置环境变量 `export META_OBSERVER_GUARD_STRICT=1`）
2. 如果测试通过 → 修改 meta-observer-guard.sh 第 28 行，将默认值从 0 改回 1
3. 如果测试失败 → 保留 STRICT_MODE=0 但修复 advisory 模式（删除自动落标伪造，见下方综合修复方案）

---

## 问题 F：二阶观察的触发点是否应该从 Stop hook 移到 ceremony 循环？

### 判定：不应该移动。Stop hook 是正确的触发点，但需要修复执行方式

### 论证

将二阶观察移到 ceremony 循环意味着"spawn 一个 meta-observer 工位"——这直接违反 075 号谱系：

> **075号结论：结构工位从 teammate 转为 skill + 事件驱动。**
> dispatch-dag 定义"事件→skill 映射"，不是"必须 spawn 的 agent 列表"。

如果在 ceremony 循环中 spawn meta-observer 工位，就是把 075 号否定的模式（结构 teammate）重新引入。

Stop hook 作为触发点的合理性：
- 二阶观察是**回顾性**的（观察本 session 发生了什么），不是**前瞻性**的
- Stop 事件 = session 即将结束 = 回顾的自然时机
- Stop hook 只是触发者（提示 Lead），不是执行者（Lead 执行观察）

**真正的问题不是触发点，而是执行方式**：

当前设计：Stop hook 阻断 → Lead 被迫执行二阶观察 → Lead 完成后落标 → 再次 Stop 时放行

这个设计的问题是在 context 最紧张的时刻（即将停止）强制 Lead 执行一段需要深度分析的任务。Lead 在这个时候的 context window 可能已经接近饱和。

**但这不是改变触发点的理由——而是改变执行方式的理由**：

替代执行方式：Stop hook 不阻断，而是**将二阶观察任务写入下一轮 session 的待办**。这样：
- 触发点仍在 Stop hook（正确的时机）
- 执行在下一轮 session 的开始（context 充裕）
- 不违反 075 号（不 spawn 结构工位）

但这引入了一个新问题：如果用户不启动下一轮 session，二阶观察就永远不会执行。这个问题可以接受——如果用户不再使用系统，二阶观察也就无意义了。

---

## 综合修复方案

### 修复 1：Stop-Guard 僵尸任务检测（ceremony-completion-guard.sh）

**文件**: `.claude/hooks/ceremony-completion-guard.sh`
**位置**: 第 90-165 行（检查 2：蜂群任务队列）

**修改逻辑**：在计算 ACTIVE_TASKS 时，增加僵尸任务过滤。

```bash
# ─── 检查 2：蜂群任务队列 ───
# 增加：对 in_progress 任务，交叉验证 team config 中 owner 是否仍有活跃 session
ACTIVE_TASKS=0
ZOMBIE_TASKS=0
# ... 现有扫描逻辑 ...

# 对每个 in_progress 任务，检查 owner 是否在活跃 team 中
# 如果 team 目录已不存在（TeamDelete 已执行），所有关联 task 都是僵尸
for team_dir in "$HOME/.claude/tasks"/*/; do
    TEAM_NAME=$(basename "$team_dir")
    TEAM_CONFIG="$HOME/.claude/teams/$TEAM_NAME/config.json"

    if [ ! -f "$TEAM_CONFIG" ]; then
        # team config 不存在 = 团队已解散，所有 task 都是僵尸
        # 跳过该目录下所有任务的活跃计数
        ZOMBIE_TASKS=$((ZOMBIE_TASKS + 该目录下的in_progress数))
        continue
    fi

    # team config 存在，检查具体 owner
    for task_file in "$team_dir"*.json; do
        # ... 读取 status 和 owner ...
        if [ "$TASK_STATUS" = "in_progress" ]; then
            # 检查 owner 是否在 team members 中
            OWNER_EXISTS=$(python -c "
import json, sys
with open(sys.argv[1]) as f:
    config = json.load(f)
owner = sys.argv[2]
members = [m['name'] for m in config.get('members', [])]
print('1' if owner in members else '0')
" "$TEAM_CONFIG" "$TASK_OWNER" 2>/dev/null || echo "0")

            if [ "$OWNER_EXISTS" = "1" ]; then
                ACTIVE_TASKS=$((ACTIVE_TASKS + 1))
            else
                ZOMBIE_TASKS=$((ZOMBIE_TASKS + 1))
            fi
        fi
    done
done
```

**注意**：当前实际 task JSON 中 `owner` 字段是 `none`（无 owner），这意味着即使交叉验证也会认为该任务没有关联工位。这恰好让僵尸检测生效——无 owner 的 in_progress 任务 = 没有人在做 = 僵尸。

但这也意味着**新创建但尚未分配的 pending 任务**也没有 owner。需要区分：
- `pending` + 无 owner = 等待分配（不是僵尸，但也不应阻断 Lead 停止）
- `in_progress` + 无 owner = 僵尸
- `in_progress` + 有 owner + owner 存活 = 真活跃

**简化版修复**（推荐）：

```bash
# 只有 in_progress 且 owner 非空且 owner 在活跃 team members 中的任务才计为活跃
# pending 任务不阻断（它们等待分配，不是正在执行）
```

这需要修改当前逻辑中 `pending` 也计入 ACTIVE_TASKS 的行为。当前代码（第 111-114 行）：

```bash
pending)
    ACTIVE_TASKS=$((ACTIVE_TASKS + 1))   # ← pending 也计入活跃
```

**修改为**：pending 不计入 ACTIVE_TASKS，只计入路由信息。

### 修复 2：熔断智能化（ceremony-completion-guard.sh）

**文件**: `.claude/hooks/ceremony-completion-guard.sh`
**位置**: 第 26-38 行（熔断检查）+ 第 127-128 行（计数器写入）

**修改逻辑**：

```bash
# 计数器文件格式改为：COUNT:LAST_ACTIVE_TASKS
COUNTER=".chanlun/.stop-guard-counter"
COUNT=0
LAST_ACTIVE=0
if [ -f "$COUNTER" ]; then
    COUNTER_DATA=$(cat "$COUNTER" 2>/dev/null || echo "0:0")
    COUNT=$(echo "$COUNTER_DATA" | cut -d: -f1)
    LAST_ACTIVE=$(echo "$COUNTER_DATA" | cut -d: -f2)
    COUNT=$((COUNT + 0))
    LAST_ACTIVE=$((LAST_ACTIVE + 0))
fi

# 熔断条件：连续 >= 3 次阻断且活跃任务数没有变化
if [ "$COUNT" -ge 3 ] && [ "$ACTIVE_TASKS" -eq "$LAST_ACTIVE" ]; then
    rm -f "$COUNTER" 2>/dev/null || true
    exit 0  # 状态停滞，放行
fi

# 写入计数器时同时记录当前活跃数
echo "$((COUNT + 1)):$ACTIVE_TASKS" > "$COUNTER"

# 如果活跃数发生了变化，重置计数器但仍阻断
if [ "$ACTIVE_TASKS" -ne "$LAST_ACTIVE" ]; then
    echo "1:$ACTIVE_TASKS" > "$COUNTER"
fi
```

阈值从 5 降低到 3：僵尸任务场景下 3 次已足够判断停滞。

### 修复 3：meta-observer advisory 模式修复（meta-observer-guard.sh）

**文件**: `.claude/hooks/meta-observer-guard.sh`
**位置**: 第 44-57 行（默认 advisory 模式）

**核心修改**：**删除自动落标**。

当前代码：
```bash
if [ "$STRICT_MODE" != "1" ]; then
    mkdir -p .chanlun 2>/dev/null || true
    echo "$CURRENT_SESSION" > "$MARKER"    # ← 删除此行（伪造执行记录）
    rm -f "$COUNTER" 2>/dev/null || true
    # advisory 输出 ...
    exit 0
fi
```

修改为：
```bash
if [ "$STRICT_MODE" != "1" ]; then
    rm -f "$COUNTER" 2>/dev/null || true
    # 不落标（MARKER 不写入）——因为二阶观察没有执行
    # advisory 输出：保留
    python -c "
import json
print(json.dumps({
    'continue': True,
    'systemMessage': '[meta-observer advisory] 本 session 二阶观察未执行（STRICT_MODE=0）。如需执行：设置 META_OBSERVER_GUARD_STRICT=1 或手动执行 .claude/agents/meta-observer.md。'
}, ensure_ascii=False))
" 2>/dev/null || true
    exit 0
fi
```

**变化**：
1. 删除 `echo "$CURRENT_SESSION" > "$MARKER"`（不再伪造执行记录）
2. advisory 文本微调：从"已跳过"改为"未执行"（语义精确——"跳过"暗示有意选择，"未执行"是事实陈述）

### 修复 4：恢复 STRICT_MODE=1 的测试路径

不在本次修复中直接恢复 STRICT_MODE=1（需要先验证上游 bug 是否修复）。但在 advisory 文本中给出了恢复指令。编排者可以通过 `export META_OBSERVER_GUARD_STRICT=1` 进行单次测试。

---

## 结果包六要素

### 1. 结论

- **问题 A**：工位自己应更新 task 状态，但 016 号谱系证明仅靠规则不可靠，必须在 Stop-Guard 层面增加僵尸检测
- **问题 B**：Stop-Guard 应交叉验证 task 状态与工位存活性。简化版：in_progress + 无 owner = 僵尸；pending 不计入活跃
- **问题 C**：熔断正确但应智能化。从固定 5 次改为"3 次 + 状态无变化"条件
- **问题 D**：advisory 模式 = 完全失效，不是降级。自动落标是伪造执行记录
- **问题 E**：应恢复 STRICT_MODE=1，但需先验证上游 bug 修复状态
- **问题 F**：触发点不变（Stop hook），但应修复执行方式（删除伪造落标）

### 2. 定义依据

- 016 号谱系："规则没有代码强制就不会被执行"——advisory 模式 = 无代码强制 = 016 号描述的失效状态
- 075 号谱系："结构工位从 teammate 转为 skill + 事件驱动"——不应在 ceremony 循环中 spawn meta-observer 工位
- 087 号谱系 问题 5 执行方案："修改 meta-observer-guard.sh 的 advisory 模式产生 JSON 格式提示"——087 号修复了"无输出"但没修复"自动落标伪造"
- 005b 号："对象否定对象"——task 状态完成是工位对"进行中"的自我否定，不能由外部代理

### 3. 边界条件

| 条件 | 翻转 |
|------|------|
| Claude Code Agent SDK 引入 task 状态自动同步（工位退出时自动标记 completed） | 修复 1 僵尸检测不再必要 |
| 上游 "Invalid signature in thinking block" bug 修复 | 可直接恢复 STRICT_MODE=1，修复 3 中的 advisory 路径变为 fallback |
| Claude Code 允许 hook 查询进程存活性 | 修复 1 可改为更精确的进程级检查 |
| Lead 在实践中确实基于 advisory 执行二阶观察 | 问题 D 判定翻转为"降级"而非"失效" |

### 4. 下游推论

1. 如果修复 1 实施（僵尸检测），Stop-Guard 的阻断准确率将提升——不再有"全部工位已退出但仍阻断 5 次"的场景
2. 如果修复 3 实施（删除自动落标），`.chanlun/.meta-observer-executed` 文件的语义恢复为"真正执行过"——任何基于此 MARKER 的后续检查都将获得正确信息
3. 修复 2（智能熔断）使得真实活跃任务场景下 Stop-Guard 不会过早放行——当前固定 5 次可能在大蜂群（10+ 工位）场景下过早放行

### 5. 谱系引用

- 016 号：规则没有代码强制就不会被执行（meta-observer advisory = 无强制 = 016 号问题的再现）
- 075 号：结构工位从 teammate 转为 skill + 事件驱动（否定将 meta-observer 移入 ceremony 循环的方案）
- 087 号：五问题诚实修复（问题 5 修复了 advisory 输出，但遗留了自动落标伪造）
- 005b 号：对象否定对象（task 状态完成的正确责任者）

### 6. 影响声明

**本诊断影响的文件**：

| 文件 | 修改内容 | 优先级 |
|------|---------|--------|
| `.claude/hooks/ceremony-completion-guard.sh` | 僵尸任务检测 + 智能熔断 | P0 |
| `.claude/hooks/meta-observer-guard.sh` | 删除自动落标伪造 + advisory 文本修正 | P1 |

**不影响**：
- dispatch-dag.yaml（本次修复不涉及事件映射变更）
- ceremony 循环逻辑（触发点不变）
- 工位 prompt（不在工位层面增加 TaskUpdate 强制——016 号证明这不可靠）
