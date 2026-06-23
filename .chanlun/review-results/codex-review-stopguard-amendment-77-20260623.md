# 异质审计报告：Stop-Guard 阻断逻辑修订方案（#77）← codex-challenger 工位

**审计工位**：codex-challenger（topo_address: swarm/codex-72）  
**被审对象**：ceremony-completion-guard.sh check3 阻断逻辑 + 571号候选修订方案（责任方过滤）  
**审计模式**：review（代码层异质审查——找同质审查看不到的代码层盲区）  
**日期**：2026-06-23  
**认识论等级**：L2（读取真实代码逐行核验 + 候选方案代码路径分析）

---

## 总判定

**CONDITIONAL PASS（有条件通过）** — 候选方案（责任方过滤）概念层正确，代码层可实装；  
发现4个代码层问题（1个 CRITICAL 竞态、1个 HIGH 身份解析、1个 MEDIUM 无 owner 字段降级策略、1个 METHODOLOGICAL 计数器设计）。  
方案B比方案A在代码层的正确性更高（方案A存在结构性缺陷）。

---

## 1. 方案A（维持现状）代码层缺陷分析

### 发现 FA-1 [CRITICAL]: counter 文件竞态导致 check3 熔断永不 fire

**位置**：lines 352-359（check2 counter 重置）、511-512（check3 counter 递增）

**代码证据**：

```bash
# check2（lines 352-359）：当 ACTIVE_TASKS 发生变化时，重置计数器
if [ "$ACTIVE_TASKS" -gt 0 ]; then
    if [ "$ACTIVE_TASKS" -ne "$LAST_ACTIVE" ]; then
        echo "1:$ACTIVE_TASKS" > "$COUNTER"   # ← 重置到 COUNT=1
    else
        echo "$((COUNT + 1)):$ACTIVE_TASKS" > "$COUNTER"
    fi
    exit 0
fi

# check3（lines 511-512）：无条件递增
if [ "$PENDING_COUNT" -gt 0 ]; then
    echo "$((COUNT + 1)):$PRE_ACTIVE_TASKS" > "$COUNTER"
    exit 0
fi
```

**竞态序列**（多 session 并发环境，counter 文件 `.chanlun/.stop-guard-counter` 是全局共享单文件）：

```
1. meta-lead(Session-A) check3 fires → 写 "1:0"
2. Lead(Session-B) check2 fires, ACTIVE_TASKS=N, N≠LAST_ACTIVE(0) → 重置写 "1:N"
3. meta-lead(Session-A) check3 fires → 读 "1:N", COUNT=1, LAST_ACTIVE=N
   PRE_ACTIVE_TASKS=0 ≠ LAST_ACTIVE=N → 熔断 skip → 写 "2:0"
4. Lead(Session-B) check2 fires, ACTIVE_TASKS=M≠2 → 重置写 "1:M"
5. meta-lead(Session-A) check3 fires → COUNT=1 AGAIN（被重置了）
```

**结论**：在 Lead session 频繁更新 counter 的活跃蜂群中，meta-lead 的 check3 熔断计数不断被 check2 重置，理论上永不 fire。这是方案A（靠熔断兜底）的结构性失效——不是偶发 race，是必然失效。

**这是 Q4 的代码层坐实**（gemini-challenger 发现了现象，本审计找到了代码根因）。

---

### 发现 FA-2 [STRUCTURAL]: 对 teammates 而言 check2 永不触发

**位置**：lines 115-140（LEAD_TASK_DIR 计算）

对于 teammate（如 meta-lead），其 SESSION_ID 不是任何 team 的 leadSessionId，所以 LEAD_TASK_DIR="" → PRE_ACTIVE_TASKS=0 → check2 的 ACTIVE_TASKS=0 → check2 不触发。

Teammates 直接跳过 check2，进入 check3。这意味着：
- 对于 teammate，check3 是唯一可能触发的阻断
- PRE_ACTIVE_TASKS 恒为 0（team 目录扫描为空）
- 熔断条件 `PRE_ACTIVE_TASKS==LAST_ACTIVE` 在 teammate 层面恒真（0==0）
- 但 counter 文件被 Lead 的 check2 重置（FA-1），打破此条件

**结论**：对 teammates 而言，熔断机制设计是合适的（PRE_ACTIVE_TASKS 恒 0），但 counter 共享导致 Lead 的 check2 重置破坏了 teammate 的计数进程。

---

## 2. 方案B（责任方过滤）代码层分析

### 发现 FB-1 [HIGH]: agent 身份解析路径已存在，复用成本低

**位置**：check1.5（大约在 lines 200-250 区域，已在 agentType 检测逻辑中读取 team-topology.json）

check1.5 已经实现：从 SESSION_ID 找 team config，读取 member.agentType。  
check3 可以复用完全相同的路径（SESSION_ID → team → agentType）。

**代码复用模式**：
```bash
# 已有路径（check1.5 区域）
AGENT_TYPE=$(python -c "
import json, os, sys
sid = sys.argv[1]; teams_dir = sys.argv[2]
for name in os.listdir(teams_dir):
    cfg_path = os.path.join(teams_dir, name, 'config.json')
    if not os.path.isfile(cfg_path): continue
    cfg = json.load(open(cfg_path))
    for m in cfg.get('members', []):
        if m.get('sessionId') == sid:
            print(m.get('agentType', 'unknown'))
            break
" "$SESSION_ID" "$HOME/.claude/teams" 2>/dev/null || echo "unknown")

# check3 中基于 agentType 过滤
RESPONSIBLE_TYPES=("genealogist" "orchestrator")  # 从 pending frontmatter 读取，或硬编码映射
if [[ " ${RESPONSIBLE_TYPES[@]} " =~ " $AGENT_TYPE " ]]; then
    # 是责任方 → 阻断
else
    # 非责任方 → 放行
fi
```

**正面发现**：实装代码路径已有先例，复用成本约 20 行 bash，不是零基础开发。

---

### 发现 FB-2 [HIGH]: pending frontmatter 无 owner 字段——降级策略必须明确

**实测**：567/571/572 front matter 中均无结构化 `owner` 或 `responsibility_parties` 字段。

```yaml
# 567 现有 frontmatter（部分）
type: genealogy
status: 生成态
settlement_blocked_on:
  task: 69
  reason: "..."
```

无 `owner` 字段 → 方案B 实装时，check3 无法读取 pending 责任方。

**降级策略选择**（编排者需裁决）：

| 策略 | 行为 | 风险 |
|------|------|------|
| **Fail-safe（默认阻断）** | owner 字段不存在 → 视为所有人都是责任方 → 阻断（保持现有行为）| 无倒退，但 meta-lead 在过渡期仍被困 |
| **AgentType 类型映射** | 硬编码 `genealogist_types = ["genealogist"]` → genealogist 阻断，其他放行 | 不依赖 pending owner 字段，可立即实装 |
| **Fail-open（默认放行）** | owner 字段不存在 → 视为非责任方 → 放行 | 违反 048（可能错误放行真责任方）|

**建议（codex 视角）**：AgentType 类型映射是最可立即实装的方案，不依赖 pending frontmatter schema 变更，而是将 agentType 与 pending 类型的责任关系硬编码。例如：
- `type: genealogy` pending → 责任方 = `genealogist` + `orchestrator`（需人工裁决）
- `type: meta-rule` pending → 责任方 = `meta-observer` + `orchestrator`

但此方案需要 pending frontmatter 的 `type` 字段规范化（当前已有，见上例）。

---

### 发现 FB-3 [MEDIUM]: Counter 共享问题在方案B中消失（间接修复）

如果方案B实装，check3 对非责任方不再阻断 → 非责任方（meta-lead 等）不进入 check3 的 counter 写入路径。

这意味着 FA-1 的竞态在实装方案B后**自动消解**：
- 非责任方（meta-lead）在 check3 处被放行，不写 counter
- Counter 只被 Lead 的 check2 和责任方（genealogist）的 check3 写入
- 单个 session 的 check2 和 check3 不并发（顺序执行），无竞态

**方案B 的 counter 竞态免疫性**：genealogist 被 check3 阻断时，PRE_ACTIVE_TASKS=0（genealogist 也是 teammate，不是任何 team 的 lead），故 LAST_ACTIVE=0 恒成立 → 熔断 3 次后正常 fire。Lead 的 check2 重置对 genealogist 的阻断计数仍有影响（FA-1 残留），但 genealogist 被阻断是正确的（责任方应被阻断），熔断对它反而不该 fire。

---

### 发现 FB-4 [METHODOLOGICAL]: 熔断阈值 3 在方案B下无副作用

当方案B实装后：
- 非责任方（meta-lead 等）→ check3 放行，熔断无关
- 责任方（genealogist）→ check3 阻断，熔断在 3 次无进展后放行（允许 genealogist 短暂退出，不死锁）

熔断对责任方的语义：「我已被阻断 3 次，pending 仍无法推进，允许暂时退出（待 #69 落盘后重新激活）」——这是合理的，不是问题。

---

## 3. 方案A vs 方案B 代码层对比

| 维度 | 方案A（维持现状，靠熔断兜底）| 方案B（责任方过滤）|
|------|------|------|
| **FA-1 counter 竞态** | 存在，永久性结构缺陷 | 消解（非责任方不进入 counter 写入） |
| **FA-2 teammates 跳过 check2** | 存在，根本原因 | 无关（非责任方已被 check3 放行）|
| **实装成本** | 零（不改代码）| 约 50 行 bash + pending type→agentType 映射表 |
| **pending owner 字段依赖** | 无 | 可用 type 映射绕过（FB-2 AgentType 方案）|
| **048号保证** | 维持（任何 agent 被 pending 阻断）| 维持（责任方仍被阻断，蜂群循环继续）|
| **元层/基因组级修改** | 不需要（现状）| 需要（hook 修改 → /ritual 门控）|

**代码层结论**：方案B 在代码层正确性上严格优于方案A（方案A 存在结构性熔断失效 FA-1）。

---

## 4. 代码层发现 vs 概念层状态

571 候选规则在概念层已通过 gemini-challenger 同质质询。  
本审计补充**代码层实证**：

| 概念层命题（571）| 代码层对应 |
|------|------|
| 「熔断兜底不可靠」（Q4 gemini 发现）| FA-1：counter 竞态导致熔断永不 fire 的代码根因（lines 354-356 vs 511-512） |
| 「责任方过滤可实装」| FB-1：check1.5 的 agentType 读取路径可直接复用 |
| 「pending frontmatter 需新增 owner 字段」| FB-2：现有 frontmatter 无此字段，可用 type→agentType 映射绕过 |
| 「非责任方放行后 counter 竞态消解」| FB-3：自动修复 FA-1 |

---

## 5. 审计结论（六要素）

1. **结论**：方案A（维持现状）存在结构性代码缺陷（FA-1：counter 竞态 → 熔断永不 fire）；方案B（责任方过滤）代码层可实装（FB-1：复用 agentType 路径）且自动修复 FA-1。建议选方案B。

2. **定义依据**：
   - 090号：声明膨胀禁止——方案A 声称"靠熔断兜底"但 FA-1 证明熔断在活跃蜂群中不可靠
   - 048号：蜂群循环不该停——方案B 保持此约束（责任方仍被阻断）
   - no-patch-mentality：方案A = 知道 FA-1 仍保留 = 补丁思维；方案B = 严格解

3. **边界条件**（结论翻转条件）：
   - 若蜂群永远单 session（无并发），FA-1 不成立，熔断可靠，方案A 可接受
   - 若 pending frontmatter 引入 `responsibility_parties` 字段，FB-2 的 type 映射可被更精确的映射替代
   - 若 pending 类型与 agentType 映射关系变化（新增 pending type），方案B 的 type 映射表需同步更新

4. **下游推论**：
   - 选方案B → 需要 pending frontmatter `type` 字段规范化 + agentType 责任映射表 + hook 修改 + /ritual
   - 选方案A → FA-1 竞态在每次活跃蜂群操作中触发，meta-lead 永久被困（直到某次 counter 状态恰好满足）
   - 无论选哪方案，571 §三 死锁机制分析应补入 FA-1 代码层根因

5. **谱系引用**：
   - 571号：Stop-Guard 阻断对象维度（本审计的直接对象）
   - 145号：智能熔断（FA-1 的熔断失效涉及此）
   - 097号：hook 纯化（方案B 在纯化框架内，只改"阻断谁"不改"注入什么"）
   - 090号：声明膨胀——方案A 的"靠熔断兜底"声明与实际代码不符

6. **影响声明**：未改动任何代码。审计涉及 `ceremony-completion-guard.sh` lines 352-359（check2 counter 重置）+ lines 499-523（check3）+ lines 142-174（熔断）。建议改动点：check3 增加 agentType 过滤 + pending type→agentType 映射表（约 50 行）。

---

## 附录：方案B 最小实现草案（供参考，不实装）

```bash
# ─── 检查 3：生成态谱系矛盾（方案B：责任方过滤版）───

# 3a. 获取当前 agent 的 agentType（复用 check1.5 路径）
CURRENT_AGENT_TYPE=$(python -c "
import json, os, sys
sid = sys.argv[1]; teams_dir = sys.argv[2]
if os.path.isdir(teams_dir):
    for name in os.listdir(teams_dir):
        cfg = os.path.join(teams_dir, name, 'config.json')
        if not os.path.isfile(cfg): continue
        try:
            d = json.load(open(cfg))
            for m in d.get('members', []):
                if m.get('sessionId') == sid:
                    print(m.get('agentType', 'unknown'))
                    sys.exit(0)
        except: pass
print('unknown')
" "$SESSION_ID" "$HOME/.claude/teams" 2>/dev/null || echo "unknown")

# 3b. pending 类型 → 责任方 agentType 映射（静态表，待编排者裁决可否使用）
GENEALOGY_RESPONSIBLE_TYPES="genealogist orchestrator"
META_RULE_RESPONSIBLE_TYPES="meta-observer orchestrator gemini-challenger"

# 3c. 扫 pending 并判断当前 agent 是否为责任方
IS_RESPONSIBLE=0
if [ -d ".chanlun/genealogy/pending" ]; then
    while IFS= read -r f; do
        [ -z "$f" ] && continue
        PENDING_TYPE=$(python -c "
import sys, re
with open(sys.argv[1]) as fp:
    for line in fp:
        m = re.match(r'^type:\s*(.+)$', line.strip())
        if m:
            print(m.group(1).strip()); break
" "$f" 2>/dev/null || echo "unknown")
        case "$PENDING_TYPE" in
            genealogy)
                [[ " $GENEALOGY_RESPONSIBLE_TYPES " == *" $CURRENT_AGENT_TYPE "* ]] && IS_RESPONSIBLE=1 ;;
            meta-rule)
                [[ " $META_RULE_RESPONSIBLE_TYPES " == *" $CURRENT_AGENT_TYPE "* ]] && IS_RESPONSIBLE=1 ;;
            *)
                # 未知类型 → fail-safe 视为责任方
                IS_RESPONSIBLE=1 ;;
        esac
        [ "$IS_RESPONSIBLE" -eq 1 ] && break
    done < <(find .chanlun/genealogy/pending -name "*.md" -type f 2>/dev/null)
fi

if [ "$PENDING_COUNT" -gt 0 ] && [ "$IS_RESPONSIBLE" -eq 1 ]; then
    # 是责任方 → 阻断
    echo "$((COUNT + 1)):$PRE_ACTIVE_TASKS" > "$COUNTER"
    # ... 输出 block 决策 ...
fi
# 非责任方 → check3 pass through，继续后续检查
```

**草案注意**：此草案不是实装建议，仅供 /escalate 时编排者评估方案B 的实现形态参考。实装需经 /ritual 元层门控。
