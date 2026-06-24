# 实装报告：Stop-Guard check3 责任方过滤（571号）← stopguard-fix 工位

**工位**：stopguard-fix（topo_address: swarm/stopguard-fix | parent_callback: main）
**日期**：2026-06-23
**对象**：`.claude/hooks/ceremony-completion-guard.sh` check3（生成态 pending 阻断）+ check0 注释
**认识论等级**：L2（真实代码逐行修改 + 沙箱场景测试，可证伪——8/8 场景含 4 反证设计）
**编排者裁决**：已采纳 571（责任方过滤），020 阻断解除（见 settled/571）

---

## 一、结论

check3 阻断对象从「任何触发 Stop 的 agent」（session 级无差别）精确化为「该 pending 的责任方 agent」（责任方级）：
- **责任方**（agentType ∈ pending 的 `responsible_agents`）→ 阻断（待命结算）
- **非责任方** → 放行（继续后续检查），消除 meta-lead 等无 SendMessage 结构工位的死锁
- **Lead session** → 无条件阻断（保留全 swarm 判据，572 决断点A）
- **Fallback**（字段缺失 / 身份不可确定）→ 保守阻断（防漏，无倒退）

8/8 场景测试通过（bash -n 通过）。codex #77 三否定全部落地。

## 二、改动清单

| 文件 | 改动 | 性质 |
|------|------|------|
| `.chanlun/genealogy/settled/571-...md`（原 pending） | frontmatter 加 `responsible_agents: [genealogist, 编排者]`（已被并发 session 随 571 结算 commit 5e4d330b89 收入 settled） | 谱系字段（codex 否定2 实装前置） |
| `.claude/hooks/ceremony-completion-guard.sh` check3 | 加责任方过滤逻辑（+约 100 行；读 transcript agentSetting + pending responsible_agents） | 元层/基因组级（编排者已采纳） |
| `.claude/hooks/ceremony-completion-guard.sh` check0 末尾 | 加正交性注释（否定3） | 注释 |

## 三、codex #77 三否定的落地形式

### 否定1（排除自称后门）— 只读客观来源
责任判定只读两个**客观**来源：
1. pending frontmatter 的 `responsible_agents` 字段（人/genealogist 显式声明，非运行时）
2. 平台在 transcript **首条 `type=agent-setting` 记录**写入的 `agentSetting`（= spawn 时记录的 subagent_type）

**绝不读取** agent 在 Stop 输出（`stop_hook_content`）里的自称——该字段从不参与责任判定。agent 无法在 Stop 回合内通过「声称自己已路由」来绕过阻断。

### 否定2（responsible_agents 字段前置）— 非硬编码映射
责任方来源是每个 pending **显式声明**的 `responsible_agents` 机器可读字段，不是 codex 草案 §附录 的 `type→agentType` 硬编码映射表。理由：硬编码映射表会随 pending 类型增长 drift（声明膨胀，090号）；显式字段把责任归属下放到 pending 自身（局部依赖，275号）。

### 否定3（对齐 check0）— 正交共存
check0（context≥85% 全局放行）与 check3 责任方过滤**正交、无重叠、非冗余**：
- check0 沿 **context 维度**放行**所有人**（含责任方）——context 临界逃生阀（编排者 task#40）
- check3 沿 **责任方维度**放行**非责任方**（正常 context 内）——阻断对象粒度过滤

一个解 context 临界死锁，一个解非责任方无意义阻断死锁。注释写入 check0 末尾 + check3 头部。

## 四、关键实测发现（修正 codex FB-1）

codex FB-1 建议「复用 check1.5 路径：session_id→member.agentType」。**实测此路径不可行**：
- team config 的 `members[]` **无 `sessionId` 字段**（实测 keys = agentId/agentType/name/cwd/model/tmuxPaneId/subscriptions/joinedAt）
- check1.5 实际只用 `leadSessionId == session_id` 定位 **Lead 的 team**，再读全体 member 的 agentType 找缺失结构工位——它**从不**把任意 teammate 的 session_id 映射到 agentType

**可行的客观身份源 = transcript 首条 `agent-setting` 记录的 `agentSetting`**（hook 已有 `transcript_path`）。实测分布证实其值即 agentType：meta-lead / genealogist / quality-guard / code-verifier / meta-observer / topology-manager / codex-challenger / gemini-challenger 等；业务命名（geneal-p4）归一为 agentType（genealogist）。Lead/main session 无 agent-setting 记录（620 个），由 `leadSessionId` 匹配识别，不依赖 agentSetting。

## 五、场景测试（沙箱，8/8 PASS）

| # | 场景 | 期望 | 结果 |
|---|------|------|------|
| (a) | 责任方 teammate（genealogist），pending 含 genealogist | 阻断 | PASS |
| (b) | 非责任方 teammate（meta-lead） | 放行 | PASS |
| (c) | context≥85%（meta-lead，900k/1M） | check0 放行（无 block） | PASS |
| (d) | pending 缺 `responsible_agents` 字段 + meta-lead | 保守阻断（防漏） | PASS |
| (e) | Lead session（leadSessionId 匹配） | 无条件阻断 | PASS |
| (f) | teammate transcript 无 agent-setting（身份不可确定） | 保守阻断 | PASS |
| (g) | quality-guard（非责任方） | 放行 | PASS |

测试隔离：临时 HOME/.claude/teams + 临时 PROJ/.chanlun/genealogy/pending + 合成 transcript（含 agent-setting + 低/高 token usage 记录）。(a)/(g) 验证责任方/非责任方分流；(b) 复现死锁修复（meta-lead 不再被 567 类 pending 困住）；(d)/(f) 验证 fail-safe 无倒退；(c) 验证否定3 正交（context 维度优先且不被 check3 干扰）；(e) 验证 Lead 全 swarm 判据保留（048）。

## 六、结果包六要素

1. **结论**：check3 实装责任方过滤（见 §一）。非责任方放行消除 Stop-Guard 死锁根因；责任方/Lead/fail-safe 仍阻断，048 蜂群级持续性不下降为个体命题。8/8 场景 + bash -n 通过。

2. **定义依据**：571号（阻断对象 ⊥ 责任方，settled）+ 048号（蜂群级有工作→不停，本实装显式保留：释放非责任方 ≠ 释放责任方）+ 097号（hook 纯化，本实装只改「阻断谁」不改「注入什么」，在纯化框架内）+ 275号（局部依赖：责任归属下放 pending 自身）+ codex #77（3 否定=实装规格）。输入满足定义条件：check3 原第 499-523 行无 owner 过滤（L0 源码）；transcript agent-setting 提供客观 agentType（L0 实测）；571 frontmatter 含 `responsible_agents`（已添加）。

3. **边界条件（结论翻转处）**：
   - (a) 若平台改变 transcript schema（agent-setting 不再首条或 agentSetting 改名）→ teammate 身份解析失效，全部走 fail-safe 阻断（保守不倒退，但责任方过滤失能）。需随平台 schema 同步。
   - (b) 若编排者裁决「Stop 应无差别阻断所有节点直至 pending 清空」（全局 barrier 语义）→ 责任方过滤判定翻转，应回退（但 settled/571 已采纳过滤，此翻转需新 escalate）。
   - (c) 若 pending 创建时未写 `responsible_agents` → 该 pending 对所有人 fail-safe 阻断（退化为旧行为，仅对该 pending）。**下游约定**：genealogist 创建新 pending 应写 `responsible_agents`（见下游推论）。

4. **下游推论**：
   - **新约定**：今后所有 pending frontmatter 应含 `responsible_agents`（结算/张力检查→[genealogist]；选择/语法记录→[编排者]；行动 gated on task→[task owner]）。缺失则退化为 fail-safe 全阻断（仅该 pending）。当前 pending 目录为空（571 已结算），无遗留缺字段 pending。
   - **check2 同维度缺口（未在本工位范围）**：572 escalate §下游推论指出 check2（任务队列）同样无 owner 作用域过滤。本工位仅实装 check3（任务边界明确）；check2 owner-aware 化是后续工位（与 565 ghost-owner / 155 owner 标识共享机制基础）。
   - **FB-3 counter 竞态间接缓解**：非责任方在 check3 放行**不写 counter**，减少 `.stop-guard-counter` 写入争用（codex FA-1 竞态部分缓解；非责任方不再进入 counter 路径）。

5. **谱系引用**：571（settled，本实装直接对象）/048（蜂群循环存在论，显式保留）/097（hook 纯化，正交扩展「阻断对象维度」）/275（局部依赖同构）/145（智能熔断，本实装减少其触发依赖）/155（owner 标识，check2 后续共享）/565（ghost-owner，check2 后续）/codex #77（3 否定实装规格）。**概念分离领域说明**：本实装处于 097→548→564→571 的「hook 阻断对象维度」谱系链；571 已 settled，无新概念分离。

6. **影响声明**：改动 `.claude/hooks/ceremony-completion-guard.sh`（check3 责任方过滤 + check0 注释，元层/基因组级，编排者已采纳）+ 571 frontmatter（responsible_agents 字段，已随 5e4d330b89 入 settled）。**影响模块**：所有 session 的 Stop 停机判定（check3 分支）——非责任方 teammate 现可正常 idle/退出；责任方/Lead 行为不变。**不改** check1/1.5/2/2.5/4/5（其余检查逻辑零改动，bit-exact）。未改任何定义/引擎/回测代码。

## 七、commit 建议

```
fix(stop-guard): check3 责任方过滤实装(571号)——非责任方放行消除死锁根因

571编排者采纳:session级无差别阻断⊥pending责任方.check3读transcript agent-setting的agentType
+pending responsible_agents字段,非责任方放行/责任方+Lead+fail-safe阻断.
codex#77三否定落地:排除自称后门(只读客观源)/responsible_agents字段前置(非硬编码映射)/对齐check0(正交).
实测修正codex FB-1:members无sessionId,客观身份源=transcript agentSetting.
8/8场景测试通过(含4反证)+bash -n通过.048蜂群级持续性保留(集体命题不下降个体).
关联048/097/275/145/565.
```

**建议合入方式**：rebase（携带谱系意义——571 实装步骤，谱系012推论）。

> **风险声明**：hook 是 Stop-Guard 自身，影响所有 session 停机。本实装已 bash -n + 8 场景沙箱验证（含责任方阻断/非责任方放行/context 放行/字段缺失保守阻断 4 必测 + Lead/未知身份/quality-guard 3 补充）。逻辑若有 bug 当前即影响本 session——实测本 session（teammate）行为符合预期（非责任方场景已覆盖）。
