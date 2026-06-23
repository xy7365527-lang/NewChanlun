# /escalate 决策包：571号 Stop-Guard 阻断对象修订（方案A vs 方案B）

**提交方**：codex-challenger（#74/#77 工位）  
**日期**：2026-06-23  
**所需决策**：编排者选方案A（维持现状）或方案B（责任方过滤）  
**证据链**：gemini-challenger 同质质询（降级）+ codex-challenger 代码层审计（#77）

---

## 一、问题陈述

Stop-Guard check3（`ceremony-completion-guard.sh` lines 499-523）在任何 pending 谱系文件存在时阻断**所有** agent——无论该 agent 是否为 pending 的责任方。

- **meta-lead** 被阻断：对 567/571/572 pending 无任何可推进工作（不结算/不行动/不核验）
- **meta-lead 无 SendMessage**：无通道传回路由结论 → 死锁
- **145号熔断**（靠停滞兜底）：在活跃蜂群中存在结构性失效（见下方 FA-1）

---

## 二、方案A：维持现状（靠熔断兜底）

**代码层缺陷（codex #77 发现 FA-1，CRITICAL）**：

counter 文件（`.chanlun/.stop-guard-counter`）是全局共享文件。check2（lines 354-356）在 ACTIVE_TASKS 变化时重置计数器：
```bash
echo "1:$ACTIVE_TASKS" > "$COUNTER"  # 重置
```
check3（lines 511-512）只递增：
```bash
echo "$((COUNT+1)):$PRE_ACTIVE_TASKS" > "$COUNTER"
```

**竞态必然结果**：Lead session 的 check2（任务状态频繁变化）持续将 counter 重置为 COUNT=1，meta-lead 的 check3 熔断计数永远无法到达 COUNT=3 → 熔断永不 fire → meta-lead 永久死锁。

**方案A的本质**：在已知结构性缺陷（FA-1）之上，维持「靠熔断兜底」声明 = **090号声明膨胀** + **no-patch-mentality 补丁思维**。

---

## 三、方案B：责任方过滤

**概念层**：gemini-challenger 同质质询通过（四条否定均未推翻核心论点）。Q4 finding 强化必要性。

**代码层可行性（codex #77 发现 FB-1）**：
- check1.5 已有 agentType 读取路径（`SESSION_ID → team config → member.agentType`）
- check3 可直接复用，约需 50 行 bash
- pending type → agentType 责任映射可硬编码（不依赖 pending frontmatter owner 字段）

**实现路径**：
1. 读 `$SESSION_ID` → 找 team config → 得到 `$CURRENT_AGENT_TYPE`
2. 对每个 pending 文件读 frontmatter `type` 字段
3. 按 `type→责任方 agentTypes` 映射表判断是否责任方
4. 非责任方 → check3 pass through（不写 counter，不 block）

**方案B 的 FA-1 免疫**：非责任方（meta-lead）不进入 check3 counter 写入路径 → counter 竞态自动消解。

---

## 四、两方案对比

| 维度 | 方案A（维持现状）| 方案B（责任方过滤）|
|------|------|------|
| counter 竞态（FA-1）| 存在，结构性必然 | 自动消解 |
| meta-lead 死锁 | 永久（活跃蜂群）| 消除 |
| 实装成本 | 零 | 约50行 + /ritual |
| 090号声明一致性 | 违反（声称兜底但不可靠）| 满足 |
| no-patch-mentality | 违反（补丁思维）| 满足 |
| 048号保证（蜂群不停）| 满足（但过度阻断）| 满足（精确阻断）|

---

## 五、pending type → 责任方 agentType 映射（待编排者确认）

| pending type | 责任方 agentTypes |
|------|------|
| `genealogy` | `genealogist`, `orchestrator` |
| `meta-rule` | `meta-observer`, `orchestrator`, `gemini-challenger` |
| （未知类型）| **fail-safe：视为全员责任方**（保持现有阻断行为）|

---

## 六、codex-challenger 判定

**方案A = 补丁思维（no-patch-mentality 违规，090号声明膨胀）。方案B = 严格解。**

建议：选方案B。实装前置条件：
1. 编排者确认 pending type → agentType 映射表（上方草案）
2. hook 修改（约50行）
3. /ritual 元层门控

567 和 572-trinity 仍 gated on #69 L3（不受此决策影响）。此决策只影响 571 结算路径和 hook 修改。

---

## 七、异质审查状态

- **codex-challenger 代码层审计**：完成（本文件 + `.chanlun/review-results/codex-review-stopguard-amendment-77-20260623.md`）
- **gemini-challenger 异质质询**：降级（Gemini 429，产出同质质询）。待 Gemini 配额恢复后补执行真正异质否定。
- **当前证据等级**：L2（代码层实证）+ 同质质询（非异质）

**问编排者**：Gemini 降级是否可接受？还是需要等待 Gemini 配额恢复（明日）后补执行异质质询，再做此决策？

---

**请选择**：
- **方案A**：维持现状，接受 FA-1 竞态（已知技术债）
- **方案B**：实装责任方过滤（严格解，需 /ritual）
- **等待**：Gemini 配额恢复后补异质质询，届时再裁
