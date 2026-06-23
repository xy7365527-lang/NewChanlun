# 蜂群机制修复：auto-ceremony hook + teammate-mode 强制 + ceremony skill 扬弃075同步

**工位**：swarm-mechanism-fix（topo_address: swarm/swarm-mechanism-fix）
**任务**：#30　**日期**：2026-06-23　**parent_callback**：main
**编排者裁决依据**：(1) 蜂群递归也要自动用 teammate swarm mode 不用 subagent；(2) ceremony 应通过 hook 自动拉起。
**认识论等级**：L2（真实 harness 实测核实 spawn 机制 + hook 行为逐场景验证；非合成/纯代数）

---

## 0. 当前 harness 真实 spawn 机制（核实结论，先核实再改）

任务强制要求"先核实当前 harness 真实机制，不照搬 stale TeamCreate 模型"。以下为**实测**结论：

| 问题 | 实测结论 | 证据 |
|------|---------|------|
| teammate 用什么工具 spawn？ | **`Agent` 工具**（不是已废弃的 `Task` agent-spawn 工具——`Task*` 现仅为 todo 管理工具 TaskCreate/Get/List/Update） | 本 session 工具集只有 `Agent` 无 agent-spawn `Task`；ceremony.md/sub-swarm-ceremony 的 `Task(...)` 是 stale |
| `team_name` 是 teammate 标志吗？ | **否，已 deprecated/ignored**。单一隐式 team（每 session 一个，自动命名 `session-<id>`） | Workflow 工具文档明示"team_name: Deprecated; ignored. single implicit team"；当前 team 名 `session-14c95478`（隐式命名，非旧式 `ceremony-v72` 显式 TeamCreate 命名） |
| teammate 模式 vs 孤立 subagent 的信号？ | **`name` 参数**：存在=可寻址 peer teammate（入隐式 team 共享 inbox）；省略=孤立 subagent | harness 报错原文："To spawn a subagent instead, omit the `name` parameter." |
| `run_in_background` 是 teammate 标志？ | 否。它控制同步/异步（前台阻塞 vs 后台异步回报），不区分 teammate/subagent。真正信号是 `name` | Agent 工具描述 + 上条报错 |
| **teammate 能 spawn teammate 吗？** | **不能**（实测核实）。team roster 是 flat，只有 LEAD(main) 能 spawn teammate；teammate 的 Agent 调用只能产出 subagent（必须省略 name） | 我（teammate）尝试 `Agent(name="probe-spawn-test", run_in_background=true)` → harness 报错："**Teammates cannot spawn other teammates — the team roster is flat. To spawn a subagent instead, omit the `name` parameter.**" |
| backend 演化 | 旧 `tmux` paneId 模型 → 当前同为 tmux backend 但单一隐式 flat team；旧 `TeamCreate`-命名 team（ceremony-v72）→ 新隐式 session team | `~/.claude/teams/*/config.json` 对比 |

**核心发现**：旧的 `TeamCreate + Task(team_name=…)` 模型已被 harness 演化淘汰。当前 = 单一隐式 flat team + `Agent` 工具 + `name` 作 teammate 信号 + **teammate→teammate 被 harness 禁止**。

---

## 1. 结论

三机制缺口中**两个半可机制实现**已修复，**半个（teammate 递归 teammate-mode）是 harness 不可弥合的 spec-execution gap，上浮编排者（不硬编码 workaround）**。

| 缺口 | 修复前 | 修复后 | 状态 |
|------|--------|--------|------|
| **1. auto-ceremony** | `session-start-ceremony.sh` 注入文本 `"ceremony 差异检查：对比下方状态快照(session vs 当前 git/谱系)"` ——**hook 本身在指示 Lead 做手动 diff**；且引用 stale `Task spawn` | FIRST_ACTION 改为正面格式强制 `invoke /ceremony 序列：python scripts/ceremony_state.py write 1 + ceremony_scan.py`，**显式禁止手动 git diff 代替 scan**，spawn 改 `Agent tool（name+run_in_background）` | ✅ 已修复 |
| **2a/2b. 匹配真实 spawn 工具** | `settings.json` PreToolUse matcher=`"Task"` + hook 引用废弃 `TeamCreate`/`Task(team_name)` ——**matcher 匹配一个不存在的工具，hook 完全失效，Lead 用 Agent tool 100% 绕过** | matcher 改 `"Agent"`；hook 重写匹配 `Agent` 工具，删除废弃 team_name 检查 | ✅ 已修复 |
| **2c. 强制 teammate mode（Lead 侧）** | 无（hook 死的） | Lead spawn 工位缺 `name`（=孤立 subagent）→ **block**（fail-open：仅 POSITIVE 确认 Lead 时 block） | ✅ 已修复 |
| **2d. teammate 递归 teammate mode** | 无 | **harness 禁止 teammate→teammate（实测）** → 不可机制实现 | ⚠ **上浮编排者** |
| **3. ceremony skill 扬弃075** | `ceremony.md` 引用 `075号（skill事件驱动）`，无常设结构工位 spawn 步骤 | 新增步骤4「6 常设结构工位 spawn（562号扬弃075）」+ 谱系引用更正 `075→562扬弃` + 隐式 team/Agent 工具同步 | ✅ 已修复 |

---

## 2. 定义依据

- **562号**（`.chanlun/genealogy/settled/562-bootstrap-structural-enforce.md`，已结算，`negates: ["075"]`）：编排者裁决「结构能力是 teammate（spawn，095/096），非纯 skill 事件驱动」。075 被**扬弃**（Aufhebung：否定结构=skill，保留孤岛化解动机由 095/096 共享 inbox 实现，提升 skill 层为轻量事件守卫并存）。→ 缺口3 是把已结算裁决传播到 ceremony.md（非新冲突）。
- **075号**（`negated_by: ["562"]`，已结算）：dispatch-dag.yaml 仍编码的 pre-扬弃 模型来源。
- **137号**：否定性文本提示对行为执行层无效 → 机制强制。缺口1 的 SessionStart 受 097号纯化边界约束（只能注入 context 无 tool 阻断），故"机制强制"= 072号双前提：提示前提（本 hook 正面格式注入 reliable additionalContext 通道）+ 机制前提（Stop-Guard 检查1.5 缺结构工位即 block，562号——Lead 跳过 ceremony 即被阻断）。
- **095/096号**：Agent Team 真递归是默认模式、无孤立 subagent。缺口2 的 teammate-mode（`name` 必需）是其在新 harness 的落地（team_name→name 信号迁移）。
- **016号**：规则无代码强制就不被执行 → matcher="Task" 死代码=规则未被执行的精确实例。

**输入数据满足定义的哪些条件**：当前 team config（session-14c95478）20 成员全在单一 flat roster，6 结构工位（meta-lead/genealogist/quality-guard/code-verifier/meta-observer/topology-manager）均为 live teammate（agentType 落盘）——经验证实 562（结构=teammate）正在运作，075（结构=skill）已不适用。

---

## 3. 边界条件（结论翻转条件）

1. **harness 再演化**：若平台恢复 teammate→teammate spawn 能力（flat roster 解除），缺口2d 的上浮自动消解，sub-swarm-ceremony 的 teammate 递归恢复可机制实现。
2. **`name` 语义变更**：若 harness 改用其他信号区分 teammate/subagent（非 `name`），enforce hook 的 teammate-mode 检测失效，需改信号。
3. **`leadSessionId` 落盘语义变更**：若 PreToolUse 不再提供 `session_id` 或 team config 不再记 `leadSessionId`，enforce hook 的 lead 判定退化为全 fail-open（advisory only，不 block）——fail-safe，不误杀，但 2c 强制失效。
4. **075 复辟**：若结构工位再被裁决回 skill（562 被否定），缺口3 的 ceremony.md 同步与 Stop-Guard 检查1.5 须同步回退。
5. **fail-open 的代价**：enforce hook 仅在 POSITIVE 确认 Lead 时 block 缺 name。若 Lead 的 session_id 首次 spawn 时 team config 未就绪（lead 未知），name 强制不触发——此窗口由 Stop-Guard 检查1.5 兜底。

---

## 4. 下游推论

1. **缺口2d 上浮**：编排者要点1「蜂群递归也要自动用 teammate swarm mode 不用 subagent」在当前 harness **对 LEAD 以下的任何节点不可成立**——teammate 递归只能 spawn subagent。这意味着 `sub-swarm-ceremony` skill（步骤1 `TeamCreate` by teammate + 步骤3 `Task(... run_in_background)` spawn 子 teammate）整体不可在当前 harness 执行。需编排者裁决方向：(a) 接受 LEAD 以下递归用 subagent（放弃"每层共享 inbox peer"性质，这削弱 562 引用的"孤岛化解"在递归层的有效性）；或 (b) 等待/要求 harness 恢复 teammate→teammate；或 (c) 改为「所有 spawn 集中由 LEAD 代理」的中心化递归（与275号局部依赖原则张力）。**这是 016/032/033/034 spec-execution-gap 链的新实例。**
2. **dispatch-dag.yaml（genome_layer）仍 stale**：line 7/11/160/633-634 仍写「结构=skill（075）」「子蜂群不再 spawn 结构 teammates」。与 562 矛盾，但：(i) 562 已结算 → 这是 stale 文档非开放冲突；(ii) dispatch-dag.yaml 是 genome_layer，修改触发 020号阻断等待，且**超出本任务缺口3 的 ceremony.md 范围**。→ **不擅自修改，flag 给 Lead 走 genome 修改流程（020-gated）**。
3. **sub-swarm-ceremony.md 同样含 stale `TeamCreate`/`Task`**：与缺口2d 同根（teammate 递归），一并上浮，不在本次以 workaround 重写。

---

## 5. 谱系引用

- **075→562 扬弃链**（结构工位 teammate↔skill）：本修复缺口3 直接消费此链。
- **095/096/097**（Agent Team 真递归 + 五特征子蜂群）：缺口2d 冲突的"应然"一侧。
- **137/072/016**（文本无效→机制强制双前提→代码强制）：三缺口共同方法论依据。
- **275号**（局部依赖原则）：与缺口2d 解法(c)中心化递归存在张力，须编排者权衡。
- **是否有相关谱系不确定的领域**：缺口2d 的「harness 物理能力 vs 蜂群架构应然」冲突可能值得新谱系记录（语法记录类）——但应由 genealogist 工位在编排者裁决后写入，本工位仅 flag。

---

## 6. 影响声明

**改动文件清单（交 Lead commit）**：

1. `.claude/hooks/session-start-ceremony.sh`（缺口1）：改 FIRST_ACTION/HEADER/冷启动 MSG——强制 invoke ceremony_scan 序列，禁止手动 diff，spawn 改 Agent tool。**纯字符串内容改动，emit_json/json.dumps 逻辑不变；两路径（cold/compact）输出经直接 file-load 验证为 valid JSON**（先前 echo 测试的 INVALID 是测试假象，非真 bug）。
2. `.claude/hooks/agent-team-enforce.sh`（缺口2）：**重写**——matcher Agent；删废弃 team_name 检查；新增 lead 判定（session_id==leadSessionId）+ teammate-mode（name）block（fail-open）；保留两基因+递归判断 advisory；INPUT 经 env var 传递（修双 stdin 重定向 bug）。
3. `.claude/settings.json`（缺口2）：PreToolUse matcher `"Task"→"Agent"`。
4. `.claude/commands/ceremony.md`（缺口3）：步骤4 新增 6 常设结构工位 spawn（562）；步骤5 `Task→Agent`、删 team_name；topo_address 模板 `{team_name}→swarm`；白名单 `TeamCreate→Agent spawn`；谱系引用 `075（skill）→075已扬弃+562+095/096`。

**验证**：
- `bash -n` 两 hook 全过；settings.json valid JSON + matcher=Agent。
- enforce hook 7 场景全绿：非Agent静默/Explore advisory/Lead无name→block/Lead有name缺基因→advisory/teammate无name→fail-open advisory/Lead齐全→clean pass/空session_id→fail-open。
- session-start 两路径 valid JSON + 含 ceremony_scan 强制 + 禁手动diff + 无 stale Task-spawn。

**未改动（明确不碰）**：`dispatch-dag.yaml`（genome_layer，020-gated，上浮）、`sub-swarm-ceremony.md`（缺口2d 同根，上浮）、`agent-team-bootstrap.sh`（文本提示前提，与机制前提并存，无需改）、Stop-Guard 5+1 检查（562 检查1.5 是机制兜底，保留）。

---

## ⚠ 上浮编排者（no-workaround，不硬编码）

**缺口2d 是真实 spec-execution gap**：编排者要点1「蜂群递归也要自动 teammate swarm mode」与 harness 实测「teammate 不能 spawn teammate（flat roster）」不可弥合。请编排者裁决递归方向（推论4.1 的 a/b/c）。同时 dispatch-dag.yaml（genome）+ sub-swarm-ceremony（skill）的 stale TeamCreate 模型需配套处置（020-gated genome 修改 + skill 重写），均不在本工位以 workaround 自行决定。
