# 异质审计报告：蜂群递归机制修复 (#69) ← codex-challenger 工位 #72

**审计工位**：codex-challenger（topo_address: swarm/codex-72）  
**被审对象**：swarm-infra（topo_address: swarm/swarm-infra）  
**审计模式**：review（异质 —— 找蜂群同质审查看不到的代码层盲区）  
**日期**：2026-06-23  
**审计依据**：agent-team-enforce.sh + agent-team-bootstrap.sh + sub-swarm-ceremony/SKILL.md + swarm-recursion-fix 报告  
**认识论等级（本审计）**：L2（读取真实代码文件逐行核验，非合成数据）

---

## 总判定

**CONDITIONAL PASS** — (c) 模型传播和 no-workaround 检验通过；发现三个异质问题（一个 MINOR BUG，一个 METHODOLOGICAL 问题，一个 ARCHITECTURAL GAP）。无翻转级别的 CRITICAL 问题。

---

## 1. 结论（逐项审计）

### 1.1 (c) 模型传播忠实性 → **PASS**

代码层全面检查结果：

- `agent-team-enforce.sh`：所有 `TeamCreate` 出现均在注释/advisory 文本的"已废弃/不可用"语境，无活跃路径调用。(c) 路径 advisory（lines 178-185）正确导向 TaskCreate 子任务，逻辑无歧义。
- `agent-team-bootstrap.sh`：`TeamCreate` 出现 1 次，内容为 `"无需 TeamCreate，team_name 已废弃不传"` — 正确的负向指令。(c) 循环描述（lines 97-110）与裁决一致。
- `sub-swarm-ceremony/SKILL.md`：8 处 `TeamCreate` 出现，全部是 "已废弃/不可用/旧模型" 说明或比较表（旧 vs 当前），无一是操作指令。四类节点 DAG 结构和 Lead-only spawn 约束均正确。

**无 stale 指令**：swarm-infra 的声明与代码对齐。

---

### 1.2 基因机制化真实性 → **CONDITIONAL PASS（含 2 个异质发现）**

#### 发现 F1 [MINOR BUG]: `lead_known` 声明但从不消费（死变量）

**位置**：`agent-team-enforce.sh` lines 118、131

```python
lead_known = False          # line 118 — 声明
...
lead_known = True           # line 131 — 赋值
```

**grep 验证**：整个 hook 仅有这 2 行，`lead_known` 从未在后续逻辑中被读取或判断。

**意图vs实际**：变量本意是区分"无 team 配置（无法判断 lead）"vs"有配置但本 session 不是 lead（确认是 teammate）"。如果使用这个区分，可以：
- `lead_known=True, is_lead=False` → 确认是 teammate → (c) advisory 有更强依据
- `lead_known=False` → 无 team 配置 → 更强 fail-open 理由

**实际影响**：当前逻辑功能正确 —— 因为 `is_lead` 判定本身已充分（`False` 时一律走 advisory 路径）。死变量不引入逻辑错误，但增加了代码噪音，可能掩盖未来的 refactor 意图。

#### 发现 F2 [METHODOLOGICAL CONCERN]: "present-by-construction" 名称强度高于实际保证

**位置**：swarm-recursion-fix 报告 §1#3、§2、bootstrap.sh lines 80-87

**问题**：工程意义上的 "by-construction" = 系统结构**保证**无效状态不可产生。

bootstrap.sh 的注入路径：
```
SessionStart hook → additionalContext → Lead 的初始 context window → Lead **自愿地**在 Agent(…) 调用中包含基因
```

关键链路上有一个语义跳跃：template 被注入到 context window，但每个具体 Agent(…) 调用的 `prompt` 参数是 Lead 手动编写的，没有结构化机制确保 Lead 将 template 中的 `topo_address: <swarm/工位名>` 填充并放入每个 spawn prompt。

**enforce hook 的检测漏洞**：lines 157-165 检查 `'topo_address' not in prompt.lower()`。template 字面含有 `topo_address: <swarm/工位名>`，Lead 若直接复制**未填充**的 placeholder，hook 仍判为基因存在 → **基因内容检查的有效域 = 字符串存在性，不等于语义有效性**。

**正确的命名**：这是 "present-by-instruction"（bootstrap 教 Lead 如何写）+ "advisory-detected"（enforce 在缺失时提醒），不是 "present-by-construction"（系统结构保证正确）。

**实际影响**：机制实装是现有 harness 下的最优选择（swarm-infra 报告 §1#3 理由充分），但命名会误导未来读者对该保证强度的估计。

---

### 1.3 genome flag 合理性 → **PASS**

dispatch-dag.yaml 的处理完全正确：
- 正确识别为 genome_layer + 020-gated（不擅改）
- 正确 flag 5 行 stale（7/11/160/633/784，连带发现 784 = bonus）
- 无越界修改

---

### 1.4 no-workaround 检验 → **PASS**

逐项检验：

| 检验项 | 结论 |
|--------|------|
| 有无 workaround 绕过 harness 限制？ | 无。(c) 裁决是编排者决策，不是 workaround |
| 有无硬编码特例模拟 teammate 递归？ | 无。Advisory 是 fail-open + 文本指引，无特例分支 |
| 有无补丁思维残留？ | 无。TaskCreate 路径是架构重构，非在旧模型上加 patch |
| (c) vs 275 张力的调和论证是否有 workaround 气味？ | 无。§3.1 的 "spawn 物理层中心化 / 依赖逻辑局部" 正交论证成立 |

---

### 1.5 架构风险：(c) 循环依赖 Lead 持续活跃 → **ARCHITECTURAL GAP（异质发现 F3）**

**发现 F3 [ARCHITECTURAL GAP]**: (c) 模型假设 Lead 持续扫描 TaskList

(c) 循环：Lead scan → spawn → re-scan → ... 不动点。

当 teammate 识别到子工作并 TaskCreate 子任务时，该子任务的 spawn 完全依赖 Lead 下一轮扫描：

- **Lead context 饱和（autocompact 触发）**：compact 期间，Lead 的 context 刷新，但 compact 后恢复 Lead 会从 `session-start-ceremony.sh` 重新读取状态。bootstrap hook 在 SessionStart 重新注入基因模板，(c) 循环指令也会重新注入。这条路径文档化了（bootstrap.sh 存在是为了解决 compact 后恢复问题）。**此场景已被覆盖**。

- **未被覆盖的场景**：如果 Lead 正在执行一个长时间任务（如大型 commit 序列），teammate A 期间 TaskCreate 了 n 个子任务，这些子任务等待 Lead 下一轮 scan 才能被 spawn。这不是 bug，但是 (c) 模型的已知时延（子任务等待时间 = Lead 的当前操作完成时间）。

swarm-recursion-fix 报告的 §3 边界条件未明确列出"Lead 当前忙碌导致的子任务等待时延"，也未说明是否有 Lead 的心跳 / timeout 机制。

**实际影响**：非 CRITICAL（现有架构中已知为特性而非 bug），但边界条件文档不完整。

---

## 2. 关键异质发现 vs 同质复述

| 发现 | 是否异质 | 说明 |
|------|----------|------|
| F1: `lead_known` 死变量 | **异质** | 逐行代码检查发现，未在 swarm-infra 报告中提及 |
| F2: "present-by-construction" 强度误用 | **异质** | 同质审查趋向接受 swarm-infra 的自我描述；异质视角从工程严格性角度质疑命名 |
| F3: (c) 循环时延未文档化 | **部分异质** | 边界条件遗漏，非纯代码层发现 |
| 7 场景测试是 L2 claim | 同质复述（接受 L2 标注） | 异质视角：场景由 swarm-infra 自行设计，更接近 L1（作者自证）而非真正的 adversarial L2 |

---

## 3. 边界条件（审计结论翻转条件）

1. **F1 翻转为 BUG**：若未来代码基于 `lead_known` 添加差异化逻辑（合理 refactor），但忘记这个变量已经有 "假设被使用" 的语义，可能引入错误分支。（当前：无影响）
2. **F2 翻转为 CRITICAL**：如果 harness 未来给出结构化 prompt 注入能力（hook 可直接注入 Agent prompt 参数），而代码仍停留在 advisory 模式，"present-by-construction" 的过度声明会掩盖真正的升级机会。
3. **F3 翻转为 BUG**：如果 (c) 循环的 re-scan 频率与 Lead 任务粒度不对齐（Lead 执行极长任务期间大量子任务积压），系统吞吐率下降。需要观测。

---

## 4. 影响声明

**审计未改动任何代码**。

审计涉及模块：
- `.claude/hooks/agent-team-enforce.sh`（F1 死变量位于此）
- `.claude/hooks/agent-team-bootstrap.sh`（F2 命名问题关联此文件描述）
- `.claude/skills/sub-swarm-ceremony/SKILL.md`（干净，无需改动）
- swarm-recursion-fix 报告（F2 的命名误用在报告文字中）

**建议**（非强制，供 Lead 和 swarm-infra 判断）：
1. 删除 `agent-team-enforce.sh` 中的 `lead_known` 变量（若明确无需区分"无 team 配置"场景），或在注释中说明为何保留。
2. 将报告 §1#3 和 §2 中的 "present-by-construction" 改为 "present-by-instruction + advisory-detected"，使声明与实际保证强度一致（090号：声明与能力一致）。
3. 在 §3 边界条件中补充 "(c) 循环时延" 条目（完整性）。

---

## 5. 谱系引用

- **092/093/095/096**：五约束 + 异质审计硬节点 —— 本节点存在的谱系依据
- **090号**：严格性语法规则 —— F2 "声明膨胀" 的谱系来源
- **137/032号**：机制化形式受可结构化性约束；fail-open 不变量 —— swarm-infra 报告的主论据，本审计确认论据成立
- **016号**：规则无代码强制就不执行 —— 死变量 F1 的间接谱系（虽无直接冲突）
- **275号**：局部依赖 —— F3 时延问题的背景谱系（Lead 只管直接依赖）
- **不确定是否有相关谱系**：F2 的命名精确性问题（"by-construction" vs "by-instruction"）——未见专有谱系记录，可能值得语法记录，由 genealogist 判断。
