# 异质审计报告（#45 正式节点 / 约束4 093号硬节点）

- **审计节点**：c-producer-codex-audit（codex-challenger，#41 子 DAG 的异质审计叶节点）
- **被审对象**：`ceremony-completion-guard.sh` + `team-topology.json` 的 (c)spawn mandate 机制化（task#43/#41.2）
- **Codex 执行路径**：`codex exec` via ChatGPT 订阅（API 路径因 429/insufficient_quota 降级）
- **Codex 原始结果**：`.chanlun/review-results/codex-review-c-producer-mechanize-20260623.md`（tokens=43,182，gpt-5.2-codex）
- **审计时序**：Codex 审计 → c-producer-mechanize 预应用 C/D 修复 → #45 本节点对预修有效性质询
- **认识论等级**：L1（逻辑正确性分析 + 差异追踪；非蜂群运行的 L2 行为验证）
- **日期**：2026-06-23

---

## 0. 审计前：Codex 原始产出摘要

Codex 对五项（A-E）的原始裁决：

| 项 | Codex 裁决 | 核心依据 |
|----|-----------|---------|
| A | CONCERN | 路由携带 mandate 文本✓，但 hook 无法校验子工位是否真正 invoke sub-swarm-ceremony |
| B | **FAIL** | check0（L51-110）在 context ≥ 85% 时 `continue:true + exit 0`，绕过 check1/1.5/2/3/4/5 全部 |
| C | CONCERN | `\|\| SPAWN_MANDATE=""` 弱 fallback——Python 整体失败时 mandate 变空串，路由变空指令 |
| D | **FAIL** | topology.json 与 hook 内联 fallback 内容不一致（双源 drift 隐患） |
| E | PASS | 无 shell 注入风险 |

**Codex 总裁决**：方向正确，但存在明确拦停回归（B）+ 双源 drift（D）→ 不能裁定为"正确且无拦停回归"。

---

## 1. 逐项质询判定

### A. CONCERN（Codex）→ **CONCERN 成立，已知上界**

**质询**：是否误读上下文？

**答**：Codex 的 CONCERN 准确描述了一个结构性限制（016号 "no code = no enforce"）：
- Stop-Guard 能做到：路由 reason 文本**总是携带** mandate（L393-400 + L417 argv[5]）✓
- Stop-Guard 不能做到：拦截 `Agent()` 工具调用并硬注入 child prompt（Stop hook 不在工具调用栈上）

**判定**：CONCERN 成立，且已被 #44 识别。这是 hook 机制的固有天花板，不是 #43 的 bug。#46 结晶节点须诚实声明此上界（不可声明为"硬注入"）。

**机制已被覆盖**：`no-patch-mentality` + `016号` 的诚实声明要求——应在结晶文档中明确标注"生产端机制化 = 路由必带 mandate，非 Agent() payload 级强制"。

---

### B. FAIL（Codex）→ **异质否定成立，#44 判定范围正确但 Codex 视角增加了更严格的系统级视角**

**质询**：Codex 的 B FAIL 是否合理？与 #44 的 B PASS 矛盾吗？

**精确分析**：

| 维度 | Codex | #44 |
|------|-------|-----|
| 判定范围 | 整个 diff（含 check0/task#40）| 仅 #43 改动范围（check2 内） |
| check0 归属 | 纳入本 diff 审计 | 归属 task#40（分离改动 flag）|
| check1-5 内容 | 未被 #43 改动（字面不变）✓ | 同 |
| check0 对 check1-5 的旁路 | **FAIL**（check0 fire 时全部旁路）| flag，非 FAIL |

**Codex 观点的有效性**：check0 在 context ≥ 85% 时执行 `continue:true + exit 0`，**无条件跳过** check1/1.5/2/3/4/5——包括正在运行中的蜂群任务。这是一个真实的系统级属性：

```
context ≥ 85% + 蜂群有活跃任务 → Stop-Guard 仍放行停机
```

这不是 Codex 误读上下文，而是 Codex 从"系统 stop 语义"角度提出的更严格质询。

**#44 的分离是正确的**：check0 是 task#40 的独立裁决（编排者明确批准，注释引用"no-workaround 严格解"），不是 #43 的意外引入。

**综合判定（异质视角补充）**：
- task#43 的 B PASS（check1-5 未被改动）**维持**
- **但异质否定增加了更严格的系统声明**：Stop-Guard 的"蜂群任务活跃 → 必 block"承诺**不再成立**——在 context ≥ 85% 的边界条件下，该承诺被 check0 覆盖。任何依赖"Stop-Guard 在任务活跃时绝对 block"的下游逻辑都需要知晓此例外。

---

### C. CONCERN（Codex）→ **CONCERN 已被预修解决，当前版本 PASS**

**质询**：预修（`MANDATE_FALLBACK` + `[ -z ] && SPAWN_MANDATE="$MANDATE_FALLBACK"`）是否真正解决？

**Codex 原始 CONCERN**：`|| SPAWN_MANDATE=""` 弱 fallback，Python 整体失败时 mandate 变空串。

**当前 hook 代码**（L362-377）：
```bash
MANDATE_FALLBACK='[team-topology.json spawn_mandate 不可读——降级 fallback] spawn 须带基因 ...'
SPAWN_MANDATE=$(python -c "..." 2>/dev/null)
# codex 异质审计 C 修复：python 整体失败或 canonical 为空 → 用降级 fallback，mandate 永不为空。
[ -z "$SPAWN_MANDATE" ] && SPAWN_MANDATE="$MANDATE_FALLBACK"
```

**逐失败模式验证**：
- Python 二进制崩溃（command-substitution 整体失败）→ `$SPAWN_MANDATE` 为空 → 守卫填 MANDATE_FALLBACK ✓
- 文件缺失 → try/except pass → 空 → 守卫填 MANDATE_FALLBACK ✓
- JSON 损坏 → 同上 ✓
- template 为空串 → `.get(...,'')` 返回空 → 守卫填 MANDATE_FALLBACK ✓
- `set -uo pipefail`（无 `-e`）→ command-substitution 失败不中止脚本 ✓

**判定**：C CONCERN **已由预修解决**，当前版本 mandate 永不为空。

---

### D. FAIL（Codex）→ **FAIL 已被重新设计为"接受不一致"，但遗留一条 comment-code 不一致（#44 未发现）**

**质询**：预修（显式标注降级 fallback + 不维护第二份完整副本）是否解决了双源 drift？

**Codex 原始 FAIL**：topology.json 与 hook 内联 fallback 内容不一致（双源，后续必然 drift）。

**预修策略**：不消灭双源，而是将 fallback 重新设计为"已知不完整的降级路径"：
- 明确标注 `[team-topology.json spawn_mandate 不可读——降级 fallback]`
- 注释声明"不维护第二份完整副本（避免双源 drift/声明膨胀）"
- 单一权威源仍是 topology.json，fallback 只在读取失败时启用

**当前 fallback 内容**（L366）：
```
[team-topology.json spawn_mandate 不可读——降级 fallback] spawn 须带基因 topo_address+parent_callback(073a/274)；工位第一步 invoke sub-swarm-ceremony skill（Read .claude/skills/sub-swarm-ceremony/SKILL.md）评估分解；≥2 独立子工作则按四类节点布设子 DAG：任务+审查(异工位/约束3)+异质审计(codex-challenger/约束4硬节点)+结晶；产出触发事件→TaskCreate(metadata.agent_type)→(c)循环 spawn(075,非 teammate 自 spawn)；原子则报告写明理由。
```

**双源 drift 解决程度**：已将 fallback 显式降格为"非权威、已知不完整版本"。Codex 的 D FAIL 诊断的是"两者声称等权但内容不同"——现在的设计是"topology 是权威，fallback 是降级路径"，不存在等权双源。**D FAIL 设计层面已解决。**

---

### ⚠ D 遗留问题：COMMENT-CODE 不一致（#44 未发现的异质发现）

**问题**（L362-363 注释 vs L366 代码）：

```bash
# 降级 fallback（codex 异质审计 D 修复）：canonical 不可读时启用，明确标注 [降级 fallback]，
#   不维护第二份完整副本（避免双源 drift/声明膨胀）——只保留一行降级提示指向 SKILL.md。
MANDATE_FALLBACK='[team-topology.json spawn_mandate 不可读——降级 fallback] spawn 须带基因 topo_address+parent_callback(073a/274)；工位第一步 invoke sub-swarm-ceremony skill（Read .claude/skills/sub-swarm-ceremony/SKILL.md）评估分解；≥2 独立子工作则按四类节点布设子 DAG：任务+审查(异工位/约束3)+异质审计(codex-challenger/约束4硬节点)+结晶；产出触发事件→TaskCreate(metadata.agent_type)→(c)循环 spawn(075,非 teammate 自 spawn)；原子则报告写明理由。'
```

**不一致点**：注释说"只保留**一行**降级提示**指向 SKILL.md**"，但实际 `MANDATE_FALLBACK` 是一个包含完整四类节点模板、七子句、两处括号说明的大段文字——既不是"一行提示"，也不是"指向 SKILL.md"。

**严重性**：MEDIUM
- 功能正常（fallback 有内容总比空串好）
- 但注释描述的设计意图（极简单行 SKILL.md 指针）与实现（完整模板摘要）不符
- 这是 `no-patch-mentality` 要求的"代码能做什么就声明什么"的轻度违反（声明比实际简化）
- 后续维护者若按注释理解修改，会错误地把 fallback 简化为一行，丢失关键内容

**修复建议**：将注释修改为如实描述当前实现：
```bash
# 降级 fallback（codex 异质审计 D 修复）：canonical 不可读时启用，明确标注 [降级 fallback]，
#   不维护与 topology template 等权的第二份副本——fallback 是已知不完整的降级摘要，
#   包含最小必要指令（spawn 基因 + 递归判断入口 + 四类节点结构），template 读取成功时被替换。
```

---

### E. PASS（Codex）→ **PASS 确认**

无 shell 注入风险（`$SPAWN_MANDATE` 单一 argv 传入 Python，`json.dumps` 做 JSON 转义）。当前版本未引入新注入面。

---

## 2. 综合判定

### 异质否定成立项

| 项 | 判定 | 性质 |
|----|------|------|
| A（CONCERN） | 成立·已知上界 | 016号结构限制，不可在 hook 层解决 |
| **B（FAIL）** | **异质否定成立**·系统声明需更新 | check0 存在——"任务活跃 → 必 block"承诺不再普适 |
| C（CONCERN） | 已解决（预修有效）| mandate 永不为空，C 关闭 |
| D（FAIL） | 已解决（设计层）+ **遗留 comment-code 不一致** | MEDIUM 级别（功能正常，文档有误）|
| E（PASS） | 确认 | 无注入风险 |

### 定义层冲突检查（no-workaround）

check0 放行口并非定义层冲突，而是 task#40 对真矛盾（蜂群持续性 vs context compact 必要性）的已批准严格解。不需要上浮矛盾。但**系统声明需要更新**：

> 旧声明（隐含）："Stop-Guard 在任务活跃时总是 block"  
> 正确声明："Stop-Guard 在任务活跃时 block，**除非** context ≥ 85%（此时 compact 优先，check0 放行）"

---

## 3. 边界条件（本判定翻转条件）

1. 若 task#40 编排者裁决被撤销或 check0 被回滚 → B 项 FAIL 消失
2. 若 hook 中 `MANDATE_FALLBACK` 被简化为真正的单行 SKILL.md 指针（按注释字面意图）→ C 重新打开（丢失关键指令）
3. 若 topology.json `spawn_mandate.template` 与 `MANDATE_FALLBACK` 在结构上再次出现单一源的虚假等权声明 → D 重新打开
4. 若 Codex 对当前版本（含 C/D 预修）重新运行 → C/D 判断可能变化（当前质询基于对 Codex 原始输出的推导分析，非对修复后版本的 Codex 二次运行）

---

## 4. 影响声明

**本产出（审计结论）未修改任何代码或配置**（异质审计节点不替修）。

- **标记 FAIL/继续阻断**：无需阻断 commit（B 是 task#40 设计性，D 遗留为 MEDIUM 级文档问题）
- **标记需跟进**：
  - `ceremony-completion-guard.sh` L362-363 注释需修正（MEDIUM，不阻断但应修）
  - #46 结晶节点须诚实声明 check0 放行例外（不可声明 Stop-Guard 绝对 block）
- **解锁 #46**：本节点完成后，#46 结晶节点可开始汇总子 DAG 产出

---

## 5. 定义依据

- **约束4（093号）**：异质审计硬节点不可省略，否则封闭自证循环；本节点满足异工位（codex-challenger ≠ c-producer-mechanize / c-producer-review）
- **016号**：no code = no enforce——A 项 CONCERN 的根因
- **090号 / no-patch-mentality**：声明与实际一致要求——D 遗留 comment-code 不一致的裁决依据
- **task#40 编排者裁决**：check0 放行口的合法性来源（no-workaround 严格解，蜂群持续 vs compact 矛盾的正式裁决）

---

## 6. 谱系引用

- **093号**：异质审计约束4硬节点
- **016号**：no code = no enforce（A 项上界）
- **090号**：严格性语法规则（D 遗留 comment-code 不一致）
- **137号**：否定性文本提示对执行层无效（mandate 注入的机制化基础）
- **task#40 裁决**：check0 放行口的存在依据（不确定是否已有 settled 谱系条目，建议 #46/genealogist 核查）

---

## 7. Codex 调用元数据

```
模式：diagnose/review → 由先前 session via codex exec 执行（ChatGPT 订阅路径）
原始结果文件：.chanlun/review-results/codex-review-c-producer-mechanize-20260623.md
tokens：43,182
预修状态：C/D 已在 Codex 审计后预应用，本 #45 节点对预修有效性进行质询判定
newchan.codex API：不可用（429/insufficient_quota，降级已记录）
```
