# Goal 契约与事件 Schema（D′ 唯一真相源）

## events.jsonl（append-only 事件源）
每行一个 JSON 事件。事件类型（11 种）。所有事件可选 `idempotency_key`（#8/655：重试安全——
同 key+同 payload[去 ts] → noop 返回既有事件不重复 append；同 key+异 payload → raise；
不进 reducer 语义，纯写入去重锚）：

| event | 必填字段 | 语义 |
|-------|---------|------|
| GOAL_SET | goal_id, description, acceptance[], base_head, ts | 设定 active goal（acceptance 每项必须 falsifiable=true；可选 acceptance[].id 稳定身份）。**同 goal_id 不可重复 GOAL_SET**（防重复 active/歧义） |
| GOAL_DRAFT | goal_id, description, acceptance[], base_head, ts | **非 active 占位**（#2 两阶段）：裸 /goal 写 skeleton acceptance 落这里，reducer **不计入 active 集**（不进 current_goal/不触发 AMBIGUOUS）。Lead 补全真实可证伪 acceptance 后写正式 GOAL_SET（同 goal_id 转正，跨类型推进合法），或先 GOAL_AMEND ACCEPTANCE_REPLACE 补全再转正。同 goal_id 不可重复 GOAL_DRAFT |
| GOAL_AMEND | goal_id, amendment_kind, reason, ts + kind 子字段 | 验收层修正（goal_id/base_head 不变）。**ACCEPTANCE_ID_BINDING**：给无 id 的 acceptance slot 补绑稳定 id（+acceptance_vector_hash 防篡改 +bindings）。**ACCEPTANCE_REPLACE**（#3）：整体替换 acceptance vector（DRAFT 转正/验收改写；+old_acceptance_vector_hash 匹配旧 vector 防并发改写 +acceptance 新 vector）。reducer 重建 vector，旧 CHECK_PASS 失配（改验收=重置闭合，正确语义） |
| GOAL_RESUME | goal_id, note, ts | session 恢复审计事件（**纯审计，禁带 base_head**——见下「base_head 不可变」） |
| DECOMPOSE | goal_id, sub_goals[]（{id, desc, blocked_by[]}）, ts | 分解为 sub-goal 树（containment 树 + blocked_by 执行 DAG） |
| EVIDENCE | sub_goal_id, artifact, ts | 工位产出证据（可选 evidence_id 稳定身份，供 CHECK_PASS 机器溯源） |
| CHECK_PASS | sub_goal_id, check, ts + **来源**（见下「CHECK_PASS 来源」） | 某验收项通过 |
| CHECK_FAIL | sub_goal_id, check, reason, ts（可选 acceptance_id, evidence_ids） | **撤销** CHECK_PASS（658 修复）：把验收项标回 not-passed + contested=true。reducer「最后写者胜」据此重开 goal（防争议验收假闭合）。匹配键与 CHECK_PASS 对称（acceptance_id 优先，否则 check 文本） |
| BLOCKED | sub_goal_id, blocker, ts | 阻塞（定义冲突/缺数据/需编排者裁决） |
| SUPERSEDE | old_goal_id, new_goal_id, ts | goal 被替代 |
| CLOSED | goal_id, ts | goal 闭合（所有 acceptance CHECK_PASS） |

### base_head 不可变（650 裁决，codex 议题二 verdict=B）
`GOAL_SET.base_head` 是**不可变历史锚**——goal 在某代码世界被定义的事实（GOAL_SET 时刻的 git sha）。
- 永不被 RESUME 改写。`base_head≠current_head` 是**正确的降级信号**（base_head_stale=True）：旧 EVIDENCE 可能未覆盖当前 HEAD、验收语义可能被代码漂移污染。
- GOAL_RESUME 是**纯审计事件**（goal_id+note+ts），不影响 base_head/acceptance/闭合——把它当 base_head 重锚 = 抹掉漂移预警（A 立场，不自洽，已否决）。
- 确需移基线 → 显式高权限 GOAL_REBASE（reason+old_base+new_base+author），非热启动自动。当前 SCHEMA 不含 GOAL_REBASE（YAGNI，需要时再加）。

### CHECK_PASS 来源（650 裁决，codex 验收闭合）
EVIDENCE=材料，CHECK_PASS=裁决。CHECK_PASS 必带来源，禁裸写（无来源的 CHECK_PASS=声明膨胀）：

| method | 附加必填字段 | 语义 |
|--------|------------|------|
| auto | command, verifier | 自动验证：command=可重跑命令，verifier=验证器标识 |
| manual | judge, rationale, evidence_ids（非空） | 人工裁决：judge=裁决者，rationale=理由，evidence_ids 指向 EVIDENCE.evidence_id |

- GOAL_SET.acceptance[].id 是 acceptance 稳定身份（可选；同一 GOAL_SET 内须唯一）。
- CHECK_PASS 可带 `acceptance_id`：匹配 goal acceptance 的 `acceptance[].id`（稳定身份）。
- **两键互斥**：CHECK_PASS 带 `acceptance_id` 时只按 `(sub_goal_id, acceptance_id)` 稳定身份匹配，**不** fallback check 文本；`acceptance_id` 缺失时才按 `(sub_goal_id, check)` 文本匹配（reader 历史有效域——历史 CHECK_PASS 无 id）。两键都 sub_goal_id 作用域隔离（goal acceptance 须 CHECK_PASS.sub_goal_id=goal_id）。

## current.yaml（reducer 输出的 projection，非手写真相源）
```yaml
goal_id: <id>
description: <一句话>
acceptance:
  - check: <可证伪验收项>
    falsifiable: true
    passed: <bool>
base_head: <git sha at GOAL_SET>          # 不可变
base_head_stale: <bool>                    # base_head≠git_head 的降级信号
status: active | blocked | closed
ready_workstations: [<sub_goal_id>...]
```

## reader/writer 有效域分层（formalization-validity-domain + codex 议题二问题1）
- **reader（goal_reducer.py）**：有效域=全部历史 events.jsonl（含手搓退化事件、schema 外叙事类型）。宽容读历史不崩——但 schema 外/writer 永不产生的事件**不得**作为当前状态机的关键语义输入（GOAL_RESUME 重锚 base_head = 非法边界，已删）。
- **writer（goal_events.py）**：有效域=仅新事件。严格守新写——退化/非法/schema 外事件 raise ValueError。
两者有效域不同是分工不是矛盾：reader 有效域更大（读历史），writer 有效域更小（守新写）。

## 分层防污染（575号）
goal events ≠ genealogy。只有 goal 定义/验收/分解原则**语义变更**才升格 genealogy（用 goal_event_id 反向引用）。

## 治理事件归属（650 实施附带裁决）
DECISION/ADJUDICATION/CEREMONY/ESCALATE 等治理/叙事类型**不进 SCHEMA 的 8 种**——它们是编排者裁决/escalation 文件的人读记录，归 EVIDENCE（artifact 自由文本）+ escalation 文件。writer 拒绝它们当结构事件 append（防退化类型扩散）；reader 宽容跳过历史中已存在的此类裸 append。
