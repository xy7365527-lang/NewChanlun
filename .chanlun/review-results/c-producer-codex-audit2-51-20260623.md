# 异质审计报告（#51 第二轮 / 约束4 093号硬节点）

- **审计节点**：c-producer-codex-audit2（codex-challenger，#41 子 DAG 第二轮的异质审计叶节点）
- **被审对象**：`ceremony-completion-guard.sh` + `team-topology.json` 第二轮 mandate 扩展（task#48：spawn 基因 + 递归判断块；task#49：自动触发严格形式 + event_skill_map）
- **Codex 执行路径**：`newchan.codex` 不可用（`No module named 'newchan'`，094号降级已记录）→ 人工异质审计（与 #45 相同降级协议）
- **先前轮次**：第一轮 codex 审计 = `.chanlun/review-results/c-producer-codex-audit-45-20260623.md`（A=CONCERN/B=FAIL/C→resolved/D→partially-resolved/E=PASS）
- **审计快照**：hook `SYNTAX_OK`（bash -n）；team-topology.json 当前 HEAD working tree
- **认识论等级**：L1（逻辑正确性分析 + 差异追踪）
- **日期**：2026-06-23

---

## 0. 审计前：快照确认

```
bash -n .claude/hooks/ceremony-completion-guard.sh → SYNTAX_OK
python3 json.load('.claude/team-topology.json') → VALID
Codex CLI: No module named 'newchan' → 094号降级，人工审计
```

第二轮改动范围（与 #50 同质审查确认的快照一致）：
- `team-topology.json`：新增 `spawn_mandate` 块（genes / recursion_block / event_skill_map / auto_trigger_strict_form / template / exemption / consumed_by）
- `ceremony-completion-guard.sh`：check1.5 reason 文字更新（Task→Agent + 基因注入指令）+ check2 SPAWN_MANDATE 读取块（362-377行）+ Python 调用增加 argv[5]=mandate

---

## 1. 逐项判定（A / B / C / D）

### A. spawn 基因注入正确性 — **CONCERN（同 #45-A，+NEW H1 风险）**

**证据**：
- `team-topology.json` `genes` 字段（行 136-140）定义两基因：`topo_address`（"节点在 DAG 中的拓扑坐标，如 L0.41.6"）和 `parent_callback`。
- `template` 字段（行 159）开头：`【spawn 基因(073a/274)】Lead spawn 时 prompt 必须注入 topo_address（DAG 拓扑坐标如 L0.41.6）+ parent_callback...`。
- check1.5 reason（hook 行 271）：`spawn 时 prompt 注入 topo_address+parent_callback 基因（073a/274号，#41 第二轮）`。

**继承 #45-A CONCERN（mechanization ceiling）**：hook 把 mandate 文本（含基因注入指令）嵌入 block.reason，不改写实际 `Agent()` payload，不校验被 spawn 工位是否真的携带了正确基因值。这是 harness 不可约上限（016号 no code = no enforce）。#50 同质审查正确标注此为 ceiling。

**NEW H1 — 坐标示例混淆（异质新发现）**：
- `genes.topo_address` 定义给出格式示例 "如 L0.41.6"；`template` 同样写 "topo_address（DAG 拓扑坐标如 L0.41.6）"。
- **风险**：Lead 按 mandate 文字执行时，可能把格式示例 "L0.41.6" 当作实际坐标注入，而不是计算真实 DAG 位置。
- **可观测症状**：如果后续子 DAG 的 topo_address 普遍显示 "L0.41.6"（而非实际路径如 "L1.53.1"），则为此风险已实例化的证据。
- **严重性**：MEDIUM。功能不中断（mandate仍被注入），但 DAG 拓扑追踪能力退化（016号原则的执行层失真）。
- **修复建议**：在 `genes.topo_address` 和 `template` 中增加明确区分声明，例如 `（下方 L0.41.6 仅格式示例，Lead 须按实际任务路径计算填写，不可照抄示例）`。

**裁决**：A CONCERN 维持（继承 #45），新增 H1 MEDIUM。

---

### B. 自动触发严格形式无 teammate 自涌现假象 — **PASS（附 H2 CONCERN）**

**证据**：
- `auto_trigger_strict_form`（topology 行 158）：`'自动触发' ≠ teammate 自 spawn（harness flat-roster 硬禁，teammate 不能 spawn teammate...）。严格形式 = Lead 事件驱动 spawn...载体是 TaskCreate（任何工位）+ Lead 唯一 spawn 源。`
- `template` 结尾段（行 159 末）：`产出触发事件时，按 spawn_mandate.event_skill_map（单一源，不在此复制具体映射）查对应 metadata.agent_type → TaskCreate 无主跟进任务 → (c)循环 spawn（非 teammate 自 spawn；载体=TaskCreate+Lead 唯一 spawn 源）。`
- `event_skill_map` 全部 13 个 agent_type 经核验均为已注册类型（code-verifier / code-reviewer / python-reviewer / security-reviewer / build-error-resolver / refactor-cleaner / genealogist / gemini-challenger / codex-challenger / doc-updater / topology-manager / skill-crystallizer / source-auditor / meta-observer）。
- **SINGLE SOURCE 设计**：template 不 inline 具体事件映射，只用 pointer（`spawn_mandate.event_skill_map`）引用；经确认 `template` 中不含任何具体事件键（`code_change_any` / `code_produced_needs_audit` 等不在 template inline 中）。

**严格形式正确**：不假装 teammate 自 spawn。

**NEW H2 — event_skill_map 指令非自包含（异质新发现）**：
- `template` 指令说 "按 spawn_mandate.event_skill_map 查对应 metadata.agent_type"——这依赖 Lead 事件触发时**分两步**操作：（1）收到 mandate 文本；（2）另行 Read `.claude/team-topology.json` 得到具体映射。
- **风险**：mandate 文本是由 hook 注入的。如果 Lead 在 compact 恢复后仅有 hook 注入的 mandate 文本（而无完整 topology 文件在上下文中），事件路由查找步骤（2）可能被跳过——Lead 会知道"该 TaskCreate 一个对应 agent_type 的任务"，但不知道哪个事件对应哪个 agent_type。
- **评估**：这是 single-source 设计的结构性 trade-off（正确消灭了 drift，但引入了 2-step 依赖）。与 R1 的 D=FAIL（双源 drift）相比，这是一个**改进但不完全**的状态。
- **严重性**：MEDIUM。不是回归（R1 的 drift 问题更严重），但引入了新的执行层依赖。
- **缓解建议**：在 `event_skill_map.description` 中增加执行注意项："Lead 事件路由时须明确 Read 本文件；mandate 文本仅指针，不含具体映射。"或在 `template` 末尾加一句 "（如需查映射：Read .claude/team-topology.json spawn_mandate.event_skill_map）"。

**裁决**：B PASS（无 teammate 自涌现假象），附 H2 MEDIUM CONCERN。

---

### C. check1/1.5/2/3/4/5 拦停未削弱 — **PASS（check0 FAIL 维持 task#40）**

**逐检查验证**：

| 检查 | 改动 | block 判定 | 裁决 |
|------|------|-----------|------|
| check0 | 新增（task#40）| `continue:true + exit 0`（绕过全部）| FAIL 继承 #45-B（task#40 授权设计）|
| check1 | 无改动 | `decision: block`（不变）| PASS |
| check1.5 | reason 文字更新（Task→Agent + 基因指令）| `decision: block`（不变，行 269-270）| PASS |
| check2 | 增加 SPAWN_MANDATE 读取 + argv[5] | `decision: block`（不变，行 412-414）| PASS |
| check3 | 无改动 | `decision: block`（不变）| PASS |
| check4 | 无改动 | `decision: block`（不变）| PASS |
| check5 | 无改动 | `decision: block`（不变）| PASS |

**check2 argv 安全性（异质视角验证）**：
- Python inline 代码（行 379-415）：`mandate = sys.argv[5] if len(sys.argv) > 5 else ''`
- 调用（行 416）：`"$ACTIVE_TASKS" "$PENDING_TASKS" "$IN_PROGRESS_TASKS" "$UNOWNED_TASKS" "$SPAWN_MANDATE"`
- 索引：argv[0]=脚本体 / argv[1-5]=五参数，`len > 5` 即 ≥6 元素，对应 5 参数调用 ✓。
- `$SPAWN_MANDATE` 双引号包裹，含换行/特殊字符也作单一 argv ✓。
- Command substitution 剥离尾部换行，template 无内嵌换行（经验证 template 是一行 JSON 字符串）✓。

**裁决**：check1/1.5/2/3/4/5 均未削弱（PASS）。check0 bypass 维持 task#40 授权设计（与 #45 B=FAIL 一致），不构成本轮引入的回归。

---

### D. 异质视角：#50 同质审查漏了什么 — **三项新发现（H1 / H2 / H5）+ 一项精化**

**H1 已在 A 项说明**（坐标示例混淆，MEDIUM）。

**H2 已在 B 项说明**（event_skill_map 2-step 依赖，MEDIUM）。

**NEW H5 — audit_exempt 声明无 hook 执行层（异质新发现）**：
- `team-topology.json` 新增 `exemption` 字段（行 160）：`纯行动类...可在子任务描述显式标注 'audit_exempt: 行动类' 免除异质审计节点`。
- `template`（行 159 末）：`纯行动类(018四分法)可标注 audit_exempt 免除异质审计`。
- **核验**：`grep 'audit_exempt' ceremony-completion-guard.sh` → 0 结果。hook 完全不检查 `audit_exempt`。
- **含义**：mandate 声明 "某类任务可豁免异质审计"，但 hook 没有任何机制验证此豁免。若工位在子任务 metadata 里写了 `audit_exempt: true`，hook 仍会将该任务计为 ACTIVE_TASKS 并 block——hook 看不到 metadata 内部的 audit_exempt 标记。
- **反向风险**：若 Lead 错误地将一个不符合行动类定义的任务标为 `audit_exempt`，无任何机制在 Stop-Guard 层拦截——豁免滥用不可检测。
- **严重性**：MEDIUM。功能层面 hook 行为不变（任务有就 block），但 mandate 与 hook 能力不一致（声明了 hook 无法实现的豁免验证）。这是 016号 "no code = no enforce" 的又一实例。
- **修复建议**（二选一，no-patch-mentality 严格解倾向前者）：
  ① 删除 `exemption` 字段 + template 中的 audit_exempt 声明，或标注 "此豁免为协议层约定（不强制执行），Lead/工位自律遵守"——诚实说明无执行层。
  ② 在 check2 的任务扫描逻辑中增加：若任务 metadata 含 `audit_exempt` 且有效，从活跃任务统计中剥离异质审计任务不计入 ACTIVE_TASKS——但这需要 check2 理解 audit_exempt 的语义，增加复杂度。

**精化：#50 F1（fallback gemini-challenger 缺失）← 改进但未关闭**：
- #50 F1 指 fallback 仅列 `codex-challenger`，不含 `gemini-challenger`（canonical template 列两者）。
- 当前 fallback（hook 行 366）：`...（含 codex-challenger 异质审计=约束4 硬节点）...`——确认 gemini-challenger 仍不在 fallback。
- **改进点（#50 未注意）**：fallback 现在开头有 "完整 template/event_skill_map 见 .claude/team-topology.json + .claude/skills/sub-swarm-ceremony/SKILL.md"——fallback 明确指向规范来源，不再试图提供完整副本。这大幅缓解了 gemini-challenger 缺失的危害（降级路径的正确响应是"读规范文件"而非"用 fallback 内联指令执行"）。
- **裁决**：F1 仍属 CONCERN 级，但危害已降级（fallback = 最低限度紧急指针，不是替代 canonical 的执行指令）。不建议修复 gemini-challenger 进 fallback（会使 fallback 再次变大），建议保持当前设计并修正注释（同 R1 D-residual 建议）。

---

## 2. 与第一轮 (#45) 对照：回归检查

| 项 | #45 裁决 | 当前第二轮状态 | 变化 |
|----|---------|--------------|------|
| A：路由层 mandate | CONCERN（ceiling）| CONCERN（ceiling + H1 示例风险）| ⚠ 新增 H1 |
| B：check0 bypass | FAIL（task#40）| FAIL（task#40，未变）| = 不变 |
| C：mandate 空串 | CONCERN → resolved | RESOLVED（mandate 永不为空）✓ | ✅ 修复 |
| D：双源 drift | FAIL → 设计层解决 | 设计层 PASS（single-source + 规范指针）| ✅ 改进 |
| D-residual：注释"一行" | MEDIUM（hook 行 363）| MEDIUM（仍存在，gap 略收窄）| ⚠ 未修 |
| E：shell 注入 | PASS | PASS | = 不变 |
| H2：event_skill_map 2-step | — | NEW MEDIUM | 🆕 |
| H5：audit_exempt 无执行层 | — | NEW MEDIUM | 🆕 |

**回归结论：第二轮不引入 block 逻辑回归。** check0 bypass 是 task#40 设计性延续（非本轮引入），check1-5 全部 block 决策未变。新增 H1/H2/H5 均是架构性限制，不影响 check 的拦停有效性。

---

## 3. 综合裁决

| 维度 | 裁决 | 说明 |
|------|------|------|
| spawn 基因注入 | CONCERN（ceiling + H1）| 文本层正确，值计算依赖 Lead 运行时，示例坐标可能被误用 |
| 自动触发严格形式 | PASS（+ H2 CONCERN）| 无 teammate 自涌现假象；event_skill_map 单一源正确但需 2-step 读取 |
| check1-5 未削弱 | PASS | 全部 block 决策不变 |
| check0 bypass | FAIL 维持（task#40）| 已授权设计，非本轮新增回归 |
| 异质新发现 | H1/H2/H5 三项 MEDIUM | #50 同质审查未覆盖 |
| 第二轮是否引入回归 | **否** | block 逻辑无回归，新风险均为架构限制 |

**总裁决：第二轮扩展方向正确，无拦停回归；附三项异质新发现（H1/H2/H5，均 MEDIUM 级，功能不中断但有执行层虚假声明）。**

---

## 4. 结果包六要素

1. **结论**：第二轮 mandate 扩展总体 PASS（spawn 基因 + 自动触发严格形式逻辑正确，check1-5 block 决策不变）；三项异质新发现：H1（坐标示例混淆 MEDIUM）/ H2（event_skill_map 2-step 依赖 MEDIUM）/ H5（audit_exempt 无执行层 MEDIUM）；一项 R1 遗留未修（D-residual 注释"一行"不符）；check0 bypass 维持 task#40 授权设计。

2. **定义依据**：约束4（093号）= 异质审计硬节点；016号 no code = no enforce（A/H5 根因）；090号 no-patch-mentality = 声明与实际一致（H5：声明 audit_exempt 豁免但无 hook 执行）；073a/274（spawn 基因定义）；原则15（递归默认）；075（event_skill_map 单一源）。

3. **边界条件（翻转条件）**：
   - 若后续 DAG 追踪显示 topo_address 普遍为 "L0.41.6"（而非真实坐标）→ H1 从 MEDIUM 升为 FAIL（示例混淆已实例化）。
   - 若 Lead 在事件路由时跳过 Read topology 文件步骤 → H2 从 CONCERN 升为 FAIL（event_skill_map 查找失效）。
   - 若 task#40 check0 裁决被撤销 → check0 bypass FAIL 消失。
   - 若 `audit_exempt` 被明确标注为"协议层约定、无 hook 执行" → H5 降为 NOTE。

4. **下游推论**：若总裁决成立，则 #41 子 DAG 第二轮扩展完成了 spawn mandate 的结构丰富化（基因 + 递归判断 + 事件映射）；但完整执行合规仍依赖 Lead 运行时正确计算 topo_address（非 hook 强制）。三项 MEDIUM 发现应在 #52 结晶时诚实声明为 "已知执行层限制"，不可将 spawn 基因 / audit_exempt / event_skill_map 描述为"自动强制"。

5. **谱系引用**：
   - 093号（约束4 异质审计硬节点）、016号（no code = no enforce）、073a/274（spawn 基因）、原则15（真递归默认）、075（event_skill_map）、090号（严格性语法规则）、137号（正面格式机制化）、task#40 裁决（check0 放行口）。
   - 不确定 H5（audit_exempt 无 hook 执行层）是否有对应谱系条目，建议 #52 结晶节点或 genealogist 核查。

6. **影响声明**：本产出为只读审计，**未改动任何文件**。新增发现 H1/H2/H5 + D-residual 交 #52 结晶节点 + team-lead，本节点不修复（约束4 审计节点审、不改）。

---

## 5. 移交 #52 结晶节点的焦点清单

| 优先级 | 发现 | 建议动作 |
|--------|------|---------|
| HIGH（持续）| check0 bypass FAIL（task#40）| 结晶声明："Stop-Guard 蜂群持续性承诺有 context≥85% 例外" |
| MEDIUM | H1：topo_address 示例混淆 | template + genes.topo_address 增加"仅格式示例，Lead 须实际计算"声明 |
| MEDIUM | H2：event_skill_map 2-step | event_skill_map.description 增加"Lead 事件路由须另行 Read topology 文件"注意 |
| MEDIUM | H5：audit_exempt 无执行层 | 删除 exemption 字段或标注"协议层约定，无 hook 执行" |
| MEDIUM | D-residual（R1 延续）| hook 行 362-363 注释修正为如实描述当前 fallback 形态 |
| LOW | F1（#50）fallback gemini-challenger 缺失 | 维持现状（fallback 已有规范指针，不建议 inline 更多）|

---

## 6. Codex 调用元数据

```
模式：review（二轮/mandate 扩展）
API：不可用（No module named 'newchan'，094号降级记录）
降级协议：人工异质审计（同 #45 降级路径，已建立先例）
验证工具：bash -n（SYNTAX_OK） + python3 json.load（VALID）+ grep/python3 代码分析
认识论等级：L1（逻辑分析，非蜂群运行 L2 行为验证）
```
