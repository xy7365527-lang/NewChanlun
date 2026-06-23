# #62 异质审计报告：#55 反 idle 生命周期机制化

**日期**：2026-06-23  
**工位**：c-producer-codex-audit2（codex-challenger 工位）  
**审计模式**：review（094号降级：Codex CLI 不可用 → 人工异质审计）  
**目标文件**：`.claude/hooks/ceremony-completion-guard.sh`（check2.5）、`.claude/team-topology.json`（anti_idle 字段 + template 末尾条款）  
**审计前置**：读取 `idle-lifecycle-mechanize-20260623.md`（#55 产出报告）作为上下文  
**关联任务**：#55（被审对象）、#62（本任务）

---

## 总判决：PASS（含2个 MEDIUM 发现）

核心机制正确：check2.5 idle 检测逻辑可靠，不削弱 check1-5 拦停，anti_idle 注入到位。
发现2个异质观点（同质审查未涉及）：H1（inherited bug 扩散）、H2（doc-only 字段分歧风险）。

---

## A. check2.5 逻辑正确性

**判决：PASS**

- **流向正确**：check2 在 `ACTIVE_TASKS > 0` 时 `exit 0`（阻断）；`ACTIVE_TASKS == 0` 时不 exit，自然落入 check2.5。check2.5 只在 all tasks done 时运行——符合设计意图。
- **检测条件精确**：`alive = isActive is True AND name ∉ STRUCTURAL`（严格 Python bool 检查，不误判 null/string）；`idle = alive ∩ has_completed ∩ ¬has_active`——三条件缺一不可，不误判新 spawn 未分配任务的工位（无 completed 记录 → 不在 has_completed → 不被 flag）。
- **结构工位双重排除**：`name ∈ STRUCTURAL` OR `agentType ∈ STRUCTURAL` 两路过滤，与 check1.5 required 集一致，避免与 check1.5 振荡。
- **非 Lead session 豁免**：check2.5 包裹在 `[ -n "$LEAD_TEAM" ] && [ -n "$LEAD_TASK_DIR" ]` 门控中，非 Lead session（如本工位）直接跳过——这是**正确设计**（check2.5 面向 Lead stop，不面向 teammate stop；teammate 只需声明 ready-for-shutdown，Lead 消费后 shutdown）。

---

## B. check1-5 不削弱

**判决：PASS**

- check2.5 是**新增**独立块（行 420-497），位于 check2 末尾 `fi`（行 418）之后，check3 开头（行 499）之前。
- 不修改 check1/check1.5/check2/check3/check4/check5 的任何条件或 exit 路径。
- check2.5 只增加 block 条件（idle 时），不减少。

---

## C. anti_idle template 注入正确性

**判决：PASS（含 H2 注意项）**

- `template` 末尾追加了完整反 idle 条款（`SendMessage parent_callback + ready-for-shutdown + 不 idle`），check2 的 SPAWN_MANDATE 注入路径消费 template → 注入到被 spawn 工位 prompt。
- 137号正面格式机制化：条款以正面命令式写出，不依赖否定性禁令。

---

## 异质发现（同质审查未充分质询的角度）

### H1（MEDIUM）：check2.5 继承 R1 共享计数器 bug

**位置**：行 487 `echo "$((COUNT + 1)):$PRE_ACTIVE_TASKS" > "$COUNTER"`

**现象**：check2.5 使用与 check3/check4/check5 相同的 `.chanlun/.stop-guard-counter` 文件写入计数器。该文件在多 session 场景下被多个 session 互相覆盖（R1 审计第45号已识别此 bug）。

**推论**：check2.5 的 145号熔断（`COUNT ≥ 3 AND PRE_ACTIVE_TASKS == LAST_ACTIVE`）在以下场景会静默失效：
- 另一个 session 在 check2.5 block 期间写入计数器，覆盖 `COUNT`，导致连续计数被打断 → 熔断永不触发
- 或另一个 session 写入 `COUNT:M`（M>0），Lead session 读出的 `LAST_ACTIVE=M`，而 Lead 自己的 `PRE_ACTIVE_TASKS=0`（所有任务完成），条件 `0==M` 不满足 → 熔断永不触发

**严重性**：MEDIUM——不影响检测正确性，但影响死锁保护的可靠性。若 Lead 无法 shutdown 某 idle 工位（工位不响应），Lead 会被无限 block，熔断失效意味着无自动解除路径。

**修复方向**：同 R1 建议——使用 session 隔离的计数器文件（`.chanlun/.stop-guard-counter-{SESSION_ID}`）。

---

### H2（LOW）：`anti_idle` 字段是文档单源，非代码执行对象

**位置**：`team-topology.json` → `spawn_mandate.anti_idle` 字段

**现象**：check2 的 SPAWN_MANDATE 注入逻辑读取 `spawn_mandate.template` 字段，不直接读取 `spawn_mandate.anti_idle` 字段。`anti_idle` 是独立的文档字段，说明机制来源和边界条件，不参与代码路径。

**016号 no-code-no-enforce 评估**：**不违反**——反 idle 条款通过 `template` 字段注入，代码执行对象是 template。`anti_idle` 字段是辅助说明（单源文档），不是被绕过的执行对象。

**分歧风险**：若未来维护者更新 `anti_idle` 字段内容但忘记同步更新 `template` 末尾条款，两者可能语义分歧。当前（本审计时）两者内容一致。

**严重性**：LOW——当前无危害，标注为注意项供维护参考。

---

## D. 与同质审查 #50 对比（异质视角补充）

| 维度 | 同质 #50 覆盖 | 本轮异质补充 |
|------|-------------|-------------|
| check2.5 条件逻辑 | ✓ 覆盖 | 无新发现 |
| 结构工位排除 | ✓ 覆盖 | 无新发现 |
| check1-5 不削弱 | ✓ 覆盖 | 无新发现 |
| 熔断 145号兼容 | 部分（声明注解） | **H1：继承 bug 扩散，熔断可靠性隐患** |
| anti_idle 字段语义 | 文档性覆盖 | **H2：doc-only 字段分歧风险量化** |
| 非 Lead session 豁免 | 未明确质询 | 确认为正确设计（非 bug） |
| 计数器多 session 竞争 | 未涉及 | H1 新扩展 |

---

## 结果包（六要素）

**1. 结论**：#55 反 idle 机制化实现正确，check2.5 独立检测不削弱 check1-5，anti_idle 注入完整。异质视角发现 H1（共享计数器继承 bug，MEDIUM）+ H2（anti_idle 字段 doc-only 分歧风险，LOW）。

**2. 定义依据**：
- 016号（no-code-no-enforce）：anti_idle 字段通过 template 间接执行——代码路径存在，不违反
- 069号（RTAS）：check2.5 检测 alive business 工位的任务生命周期终结，符合 RTAS 节点存在论
- 145号（智能熔断）：check2.5 使用相同计数器格式，但跨 session 竞争使熔断可靠性降低
- 137号（正面格式）：反 idle 条款以正面命令式注入，符合正面格式机制化

**3. 边界条件**：
- H1 翻转：计数器改为 session 隔离 → 熔断恢复可靠性
- H2 翻转：anti_idle 字段被维护者误认为执行对象并修改而不同步 template → 分歧

**4. 下游推论**：
- check2.5 正确添加：ceremony 终止态更干净（business pty 不泄漏）
- H1 不修复：多 session 场景 Lead 可能被 idle 工位永久 block（需手动干预）
- 与 #41 spawn mandate 闭环成功：spawn → 工作 → 递归 → 汇报 → 声明退出 → Lead shutdown 全生命周期机制化

**5. 谱系引用**：
- R1 第45号审计已识别共享计数器 bug（H1 inherited）
- 016号/069号/137号/145号：均在已结算谱系中
- 无新谱系冲突

**6. 影响声明**：
- 本审计只读，不修改任何文件
- H1 是已识别结构性 bug（非 #55 引入，#55 沿用现有计数器写法），修复需独立任务
- H2 为 LOW 注意项，无需立即行动

---

**审计完成**：#62 ready-for-shutdown
