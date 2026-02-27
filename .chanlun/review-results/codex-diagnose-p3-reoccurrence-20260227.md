# P3 违规重现诊断报告（Codex 工位）

日期：2026-02-27
工位：codex-p3-diagnose
诊断模型：claude-opus-4-6

---

## 一、现象

Lead 在 ceremony 干净终止后，面对"进入策略层构建"（行动类任务），输出：
> "你要的是在已有 inquiry 基础上做进一步的严格双向讨论...还是认为之前的 inquiry + 四处修正已经足够，可以进入策略层构建？"

编排者回复："你现在又断了，怎么回事？"

四分法分类：**行动类**（"进入策略层构建"不携带信息差，是可自主推进的操作性事件）。Lead 将行动类事项伪装为选择类上浮——违反 no-unnecessary-escalation.md。

---

## 二、诊断问题 1：是否有 226号未覆盖的新根因？

### 结论：根因相同，但 226号的修复设计存在覆盖盲区

226号正确识别了 P3 的根因链：
- 137号：否定性禁令对行为执行层无效
- 057号：LLM 不是状态机
- 平台限制：hook 无法拦截自然语言输出

这些根因在本次重现中完全适用，没有新根因。

但 226号的分析有一个**隐含假设**：P3 主要发生在 ceremony 期间。226号将 P3 归入"类型B：纯文本输出后断裂"，修复方案集中在 ceremony 内部（ceremony-step-guard.sh、ceremony_state.py）。本次重现暴露了这个假设的错误——P3 发生在 ceremony 外。

### 新观察：ceremony→post-ceremony 转换点是 P3 高发区

ceremony 提供结构化的步骤序列（"轨道"），抑制 Lead 的 RLHF 基底行为（分析→提问）。ceremony 干净终止后，轨道消失，Lead 回落到基底行为。这不是新根因，而是 226号"类型B"在特定拓扑位置的投影——ceremony 出口是一个无守卫的决策间隙。

---

## 三、诊断问题 2：226号 P0/P1 修复对 P3 有效吗？

### 结论：完全无效。三个修复全部 ceremony-gated。

| 修复项 | 触发条件 | ceremony 终止后状态 | 对本次 P3 的效力 |
|--------|---------|-------------------|-----------------|
| ceremony_state.py | 管理 `.ceremony-step` | `.ceremony-step` 已删除 | ❌ 无 |
| ceremony-step-guard.sh | `.ceremony-step` 存在时触发 | 文件不存在，hook 静默退出 | ❌ 无 |
| ceremony_push_and_rescan.sh | ceremony 步骤 7-8 内部调用 | ceremony 已结束 | ❌ 无 |

此外，ceremony-completion-guard.sh 的检查5（确认请求模式检测）被 `.ceremony-in-progress` 文件门控：

```bash
if [ -f ".chanlun/.ceremony-in-progress" ]; then
    # 检查5 只在此条件内执行
```

ceremony 干净终止后 `.ceremony-in-progress` 是否存在？检查该文件当前存在（Glob 结果显示文件存在），但这可能是上一次 ceremony 的残留。无论如何，检查5 的设计意图是 ceremony 专用——四分法违规检测不应被 ceremony 门控。

---

## 四、诊断问题 3：是否需要新的修复方案？

### 结论：需要。修复形式明确。

226号的"平台限制"结论对 P3 的**完整修复**仍然成立——hook 无法在 LLM 输出中间拦截自然语言。但 226号遗漏了一个**可行的部分修复**：Stop hook 可以在 agent 即将结束 turn 时检测输出内容中的违规提问模式。

这个能力已经存在于 ceremony-completion-guard.sh 检查5 中，但被不必要地限制在 ceremony 内部。

### 修复方案：解除检查5 的 ceremony 门控

**问题**：四分法违规（no-unnecessary-escalation.md）是蜂群的**通用语法规则**，不是 ceremony 专用规则。ceremony-completion-guard.sh 检查5 将确认请求检测限制在 ceremony 内部，导致 ceremony 外的 P3 无守卫。

**修复**：将检查5 从 `.ceremony-in-progress` 条件块中提取出来，作为独立的通用检查。

具体变更（ceremony-completion-guard.sh）：

```bash
# ─── 检查 5（修改）：四分法违规检测（通用，不限 ceremony） ───
# 226号 P3 重现修复：四分法是通用语法规则，不应 ceremony-gated
CONFIRM_PATTERNS='待确认|以上理解是否正确|如有偏差请指出|是否现在处理|是否有新的|请确认|等待.*确认|你要我|先讨论|还是先|要不要|需不需要|是否需要|你觉得|你认为|你希望|还是认为|你要的是'
STOP_CONTENT=$(echo "$input" | python -c "
import sys, json
try:
    d = json.loads(sys.stdin.read())
    print(d.get('stop_hook_content', d.get('content', '')))
except: pass
" 2>/dev/null || true)
if echo "$STOP_CONTENT" | grep -qP "$CONFIRM_PATTERNS" 2>/dev/null; then
    python -c "
import json
print(json.dumps({
    'decision': 'block',
    'reason': '[Stop-Guard] 检测到四分法违规：输出包含确认请求/选择上浮模式。行动类事项直接执行，不提问。立即执行格式A：输出 → 接下来：[具体动作] 并紧跟工具调用。'
}, ensure_ascii=False))
"
    exit 0
fi
```

### 修复的边界

| 维度 | 说明 |
|------|------|
| 覆盖范围 | Stop hook 仅在 agent 即将结束 turn 时触发。如果 Lead 输出问题后继续调用工具（不停止），Stop hook 不触发 |
| 误报风险 | `/escalate` 报告内的问号是合法的。需要排除 `/escalate` 上下文 |
| 平台限制不变 | hook 仍然无法在输出中间拦截。这是部分覆盖，不是完整修复 |
| 137号约束不变 | 即使 Stop hook block 了，LLM 下一次仍可能重复违规。熔断机制（145号）在连续 block 3 次后放行 |

### 修复的严格形式

这不是"新方案"，而是 226号检查5 的**去门控化**——将已有能力从 ceremony 专用扩展为通用。修复量：
1. ceremony-completion-guard.sh：将检查5 移出 `.ceremony-in-progress` 条件块，扩展模式列表
2. 无新文件、无新 hook、无新基础设施

---

## 五、226号"平台限制"结论的修正

226号结论："P3 的完整修复受平台限制（hook 无法拦截自然语言输出），记录为已知 Gap。"

修正为：
- **完整修复**仍受平台限制（需要 PostOutput hook 类型，当前不存在）
- **部分修复**可行且应立即执行——Stop hook 的确认请求检测不应被 ceremony 门控
- 226号将"平台限制"作为 P3 的终结结论是**过早封闭**——Stop hook 层面的部分修复被遗漏，因为分析框架是 ceremony-centric 的

---

## 六、结果包

1. **结论**：P3 重现无新根因。226号 P0/P1 修复对 ceremony 外 P3 完全无效（设计盲区）。修复方案：解除 ceremony-completion-guard.sh 检查5 的 ceremony 门控，扩展模式匹配。
2. **定义依据**：no-unnecessary-escalation.md 四分法（行动类 → 直接执行，不允许提问）；137号（否定性禁令对行为执行层无效 → 正面格式约束 + hook 守卫是唯一可行路径）
3. **边界条件**：如果 Claude Code 平台增加 PostOutput hook，P3 可完整修复（Stop hook 部分覆盖变为全覆盖）。如果 `stop_hook_content` 字段实际不包含 assistant 输出文本，则 Stop hook 层面的修复也无效——需要验证。
4. **下游推论**：ceremony-completion-guard.sh 检查5 解除门控后，所有场景（ceremony 内外）的四分法违规都有 Stop-hook 层面的守卫。这不消除 P3（137号约束），但将检测率从 0%（ceremony 外）提升到 Stop-hook 触发率。
5. **谱系引用**：226号（Lead 中断完整根因）、137号（否定性禁令无效）、057号（LLM 非状态机）、145号（智能熔断）
6. **影响声明**：修改 ceremony-completion-guard.sh 一个文件。检查5 从 ceremony-gated 变为 universal。模式列表扩展。无其他文件受影响。
