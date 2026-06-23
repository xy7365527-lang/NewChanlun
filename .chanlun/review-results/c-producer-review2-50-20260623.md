# 审查节点 #50 — 第二轮 mandate 扩展复核（#48/#49）

> 审查节点：c-producer-review2（异工位，≠ c-producer-mechanize/c-producer-review/c-producer-codex-audit，约束3 执行不可自观）
> topo_address: L1.41.8 ｜ parent_callback: team-lead/main
> 审查目标：#48（spawn 基因 + 递归判断块 073a/274/原则15）+ #49（自动触发严格形式 + event_skill_map 075/no-workaround）
> 被审查产出快照：team-topology.json `689341e898` ｜ ceremony-completion-guard.sh `5139584e97`（均未提交，working tree）
> 日期：2026-06-23

---

## 结论（总裁决：PASS，附 2 CONCERN + 1 越界上浮）

第二轮 mandate 扩展（spawn 基因 + 递归判断块 + 自动触发严格形式 + event_skill_map）**逻辑正确、未假装 teammate 自涌现、未削弱 #48/#49 范围内的任何拦停判定、bash -n 与 JSON 均通过**。审查期间 producer 正在并行执行第三轮修复（已消除 codex 第一轮 C/D 的 template↔event_skill_map 双源 drift），当前快照已优于 codex 第一轮所审版本。

无 #48/#49 范围内的 FAIL。两个 CONCERN（fallback 注释声明膨胀残留、审查为移动目标）+ 一个越界项（check0 全局放行，task#40 编排者裁决，须 #51/Lead 确认其授权边界）。

---

## 逐项裁决（A/B/C/D）

### A. spawn 基因注入（topo_address + parent_callback + 递归判断块）—— **PASS（附 mechanization-ceiling 注记）**

**证据**：
- 基因定义：team-topology.json `genes`（行 137-140：`topo_address` / `parent_callback`）+ `recursion_block`（行 141，原则15 真递归默认）。
- 注入载体：`template`（行 159）开头即 `【spawn 基因(073a/274)】...注入 topo_address...+ parent_callback...【递归判断(原则15)】第一步 invoke sub-swarm-ceremony skill...`。
- hook 双路注入（优于 stale 版仅 check2）：
  - check2 (c)spawn 路由（hook 行 397）：`被 spawn 工位 prompt 必须含：{mandate}`（mandate=template）。
  - check1.5 结构工位路由（hook 行 271，第二轮新增）：`spawn 时 prompt 注入 topo_address+parent_callback 基因（073a/274号，#41 第二轮）`，并同步修正了 stale `Task(team_name=)` → `Agent(name=…, run_in_background=true)`（flat roster 一致性，附带修复，非削弱）。

**裁决**：基因 + 递归判断块在**每条 spawn 路由**中机制化携带，正确。

**mechanization-ceiling 注记（继承 codex 第一轮 A=CONCERN，非 #48/#49 引入）**：注入是**路由文本层**——hook 把 mandate 文本嵌入 block.reason，不改写实际 `Agent` payload、不校验被 spawn 工位是否真的 invoke sub-swarm-ceremony、不计算每任务的 topo_address 具体值（如 L1.41.8）。具体坐标值由 Lead 依 DAG 填写。这是 harness 不可约上限（hook 不能 spawn / 不能改 payload），且 137号正面格式诚实声明，非声明膨胀。"自动携带"成立于路由层，"自动注入并强制执行"不成立——须 Lead 合规。#51/Lead 知悉此天花板即可，无需修复。

### B. 自动触发机制（075 event_skill_map）逻辑正确 + 未假装 teammate 自涌现 —— **PASS**

**证据**：
- `auto_trigger_strict_form`（行 158）显式声明严格形式：`'自动触发' ≠ teammate 自 spawn（harness flat-roster 硬禁…spec-execution gap）。严格形式 = Lead 事件驱动 spawn…结构工位由 check1.5 强制常设存在 + 事件→TaskCreate(metadata.agent_type)→(c) 循环 spawn…载体是 TaskCreate（任何工位）+ Lead 唯一 spawn 源`。**明确否定 teammate 自涌现**，载体诚实落在 TaskCreate（生产端，任意工位）+ Lead 唯一 spawn 源（消费端）。
- 机制闭合性验证：
  - 消费端（hook 机制化）：check2 无主任务 → (c)spawn（hook 行 392-400）✓
  - 结构工位常设（hook 机制化）：check1.5 `required`（hook 行 243）强制 `meta-lead/genealogist/quality-guard/code-verifier/meta-observer/topology-manager` 存在 ✓
  - 生产端（工位纪律，非 hook）：事件→TaskCreate 依赖工位遵守 mandate——`hook 不能 spawn（创世 Gap）` 诚实声明，hook 物理上也不能 TaskCreate，故生产端必为工位驱动。这是 harness 上限，非伪机制。
- `event_skill_map`（行 142-157）13 事件→agent_type 映射，结构良好；**全部 agent_type 经核验均为已注册类型**（code-verifier/code-reviewer/python-reviewer/security-reviewer/build-error-resolver/refactor-cleaner/genealogist/gemini-challenger/codex-challenger/doc-updater/topology-manager/skill-crystallizer/source-auditor/meta-observer），无悬挂引用。
- 单一源（第三轮已修，消解 codex 第一轮 D 的 template-inline drift）：`event_skill_map.description`（行 143）标注 `SINGLE SOURCE——template 只引用本字段，不复制具体映射`；template（行 159）改为 `按 spawn_mandate.event_skill_map（单一源，不在此复制具体映射）查对应 metadata.agent_type`——**不再 inline 局部事件清单**，template↔map drift 已消除。

**裁决**：严格形式正确，无任何 no-workaround 违反（未绕过 flat-roster、未假装自涌现）。事件映射单一源、agent_type 全有效。

### C. 未削弱 check1/1.5/2/3/4/5 拦停逻辑 —— **PASS（#48/#49 范围内）+ 1 越界上浮（check0）**

**#48/#49 范围内**：第二轮改动 = JSON 内 spawn_mandate 扩展 + 将其文本注入 check1.5/check2 的 `reason` 字段。**无任何 block DECISION 改动**：
- check1.5（hook 行 264-275）：仍在 `STRUCT_MISSING` 非空时 `decision:block`，仅 reason 文本变富（加基因注入指令）✓
- check2（hook 行 352-417）：仍在 `ACTIVE_TASKS>0` 时 `decision:block`（行 412-415 未变），仅 route 文本携带 mandate（行 397）✓
- check1/3/4/5：diff 中无触及 ✓

**越界上浮（非 #48/#49，但同 working tree，须 #51/Lead 裁定）**：
- **check0**（hook 行 51-110，归属注释 `编排者裁决…task#40`）在 `context ≥ 85%` 时 `continue:true; exit 0`（hook 行 97-108），**绕过 check1/1.5/2/3/4/5 全部 block 检查**。
- 这正是 codex 第一轮 B=FAIL 所指（"违反硬约束：只许改 check2 文本，不许动任何 block 判定"）。
- 本审查立场：check0 是 **task#40 编排者裁决产物**（context 临界 compact 优先于蜂群持续），**非 #48/#49 引入**——两个未提交任务在同一文件 scope 渗透。本审查节点不独立 FAIL 编排者已裁决项，但须上浮一个待确认边界：**task#40 的裁决是否显式授权"绕过 block 检查 1-5"，还是仅授权"放行停机以触发 compact"**？若后者，check0 的实现超出授权（多绕过了 check1.5 结构 bootstrap / check3 谱系 / check4 proof / check5 四分法），须 #51 codex + Lead 确认。

### D. bash -n hook 语法 —— **PASS**

- `bash -n .claude/hooks/ceremony-completion-guard.sh` → `SYNTAX_OK`
- `python3 -c "json.load(...)"` team-topology.json → `JSON_VALID`
- MANDATE_FALLBACK 单引号包裹无 ASCII 撇号；mandate 经 `"$SPAWN_MANDATE"` 单 argv 传参 + `json.dumps(ensure_ascii=False)`，无 shell 注入（与 codex 第一轮 E=PASS 一致）。

---

## 附加发现

### F1（CONCERN，borderline，交 #51 裁定）— fallback 注释声明膨胀残留

- hook 行 362-364 注释声明：`不维护第二份完整副本…只保留一行降级提示指向 SKILL.md`。
- 实际 `MANDATE_FALLBACK`（hook 行 366）为多子句不变量：`…spawn 带基因 topo_address+parent_callback；工位 invoke sub-swarm-ceremony…→四类节点子DAG（含 codex-challenger 异质审计=约束4 硬节点）；事件→TaskCreate…→Lead spawn。`
- 评估：第三轮已实质缓解 codex 第一轮 D（fallback 现显式 `完整 template/event_skill_map 见 …canonical`，自我标注"降级"+deferral 到 canonical，不再冒充权威；单一源意图保留）。**残留**：(a) 注释"一行"与多子句现实不符（轻度描述性 声明膨胀）；(b) fallback 仅列 `codex-challenger`，canonical template 列 `codex-challenger/gemini-challenger`（窄化非矛盾，且 fallback 仅在 canonical 不可读的退化路径触发）。
- 注：codex 第一轮将前身 rated D=FAIL，故 #51 须确认本缓解是否充分。建议严格修复（任选其一，no-workaround 严格解倾向前者）：① 将 fallback 裁剪为真正单行指针（`canonical 不可读，Read team-topology.json + sub-swarm-ceremony/SKILL.md 取完整 mandate`），使代码匹配自身注释，彻底单一源；② 或修正注释为"最小不变量提示 + deferral 到 canonical"，诚实描述形态。

### F2（note）— 见 A 的 mechanization-ceiling，无需修复

### F3（CONCERN，并发）— 审查为移动目标

- 审查期间两文件均处于活跃编辑（mtime JSON 05:37:53 / hook 05:38:39；首次 `git diff` 与复读之间 JSON 已变化——producer 正在执行第三轮 codex C/D 修复）。
- 本裁决基于快照 JSON `689341e898` / hook `5139584e97`。**强烈建议 commit/freeze 后再交 #51 codex**，否则 #51 审计另一快照 = 移动目标，结论不可复现（feedback_task_queue_owner_liveness 同构风险）。

---

## 结果包六要素

1. **结论**：第二轮 mandate 扩展 PASS（A/B/D PASS，C 范围内 PASS）；2 CONCERN（F1 fallback 注释声明膨胀、F3 移动目标）+ 1 越界上浮（check0 全局放行授权边界）。无 #48/#49 范围内 FAIL。
2. **定义依据**：约束3（执行不可自观）→ 本节点异工位执行满足；约束4（093号 异质审计硬节点）→ template/fallback/SKILL.md 均保留 codex-challenger 审计节点不可省，满足；075号 event_skill_map → 单一源 + agent_type 全有效，满足；137号正面格式 → 基因/mandate 以正面指令嵌入 block.reason，满足；no-workaround → auto_trigger_strict_form 显式不绕过 flat-roster、不假装自涌现，满足。
3. **边界条件（结论翻转条件）**：
   - 若 #51 codex 裁定 F1 的 fallback 残留 drift 仍构成 D-级 FAIL（注释声明膨胀不可接受）→ 本 PASS 降级，须先修 fallback。
   - 若 Lead 确认 task#40 裁决**未**授权绕过 check1.5/3/4/5（仅授权放行停机触发 compact）→ check0 越界，C 降级为 FAIL，须收窄 check0 仅放行而非全 bypass。
   - 若 producer 在 #51 前继续编辑 → 本快照失效，须以新快照重审。
4. **下游推论**：若结论成立，则 #41 子 DAG 的生产端机制化（每次 spawn 自动携带基因+递归判断+事件触发 mandate）在 harness 上限内闭合——后续所有 (c)spawn 工位将自动收到 sub-swarm-ceremony 分解评估指令，封闭自证循环（缺异质审计节点）的结构性缺口被堵。但 mechanization-ceiling（A 注记）意味着实际合规仍依赖 Lead/工位执行，非 hook 强制——#53（多空双开递归滤波器）等后续重任务若经 (c)spawn，其子 DAG 是否真布四类节点仍需审查节点逐案验证。
5. **谱系引用**：073a（spawn 基因）、274（topo_address/parent_callback）、原则15（真递归默认）、075（event_skill_map）、093（约束4 异质审计）、137（正面格式机制化）、095/097（四类节点 DAG）、016（无代码无强制）。codex 第一轮审计（codex-review-c-producer-mechanize-20260623.md）A=CONCERN/B=FAIL/C=CONCERN/D=FAIL/E=PASS——本第二/三轮已修 C（mandate 永不为空）、D（template 单一源化），B（check0）越界未决、A（路由层 vs 强制层）为不可约上限。
6. **影响声明**：本产出为只读审查，**未改动任何文件**。落盘本报告 `.chanlun/review-results/c-producer-review2-50-20260623.md` 作为 #51 codex 异质审计输入。指认的待修点（F1 fallback、check0 授权边界）交 #51 + Lead，本节点不修复（约束3）。

---

## 移交 #51 codex 异质审计的焦点

1. F1：fallback 注释"一行降级提示" vs 多子句现实 + 仅列 codex-challenger——是否仍构成 D-级 单一源/声明膨胀 FAIL？
2. check0：task#40 裁决是否授权绕过 block 检查 1-5（而非仅放行停机）？codex 第一轮 B=FAIL 是否在第二/三轮快照下仍成立？
3. 复审快照须为 commit 后的冻结版本（见 F3），否则移动目标。
