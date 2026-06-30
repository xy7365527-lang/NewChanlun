# codex 严格解法：acceptance_id 补绑（SUPERSEDE vs GOAL_AMEND）2026-06-30

异质源 codex CLI(gpt-5.5,xhigh,52986 tokens)。编排者裁A=路1 SUPERSEDE,要"跟codex找严格解法"。原始/tmp/codex_ans_supersede.txt

## codex严格裁决：不用裸SUPERSEDE,新增GOAL_AMEND(ACCEPTANCE_ID_BINDING)

核心论断：补id是**表示层/寻址层修正**,非goal取代。SUPERSEDE语义保留给目标外延/acceptance谓词变化。用SUPERSEDE补id=制造假goal更替+goal alias+历史转移复杂度。

### 裁决四点
1. **不用SUPERSEDE**：它会制造假的goal更替。
2. **base_head保持fab1f08a不变**(不重锚)。GOAL_AMEND单独记recorded_head表示'补id发生在现在'。
3. **历史进度slot继承**：同goal_id同acceptance slot继续存在。slot_uid=goal_set_event_id+ordinal+acceptance_hash。旧CHECK_PASS解析到slot,后续slot绑acc-n,进度自然继承。
4. **审计**：amend必须append-only,带event_id+reason+acceptance_vector_hash+每项ordinal/hash,重放幂等。

### GOAL_AMEND事件结构
```jsonl
{"type":"GOAL_AMEND","event_id":"evt-amend-001","goal_id":"g-sigma-complete-l2-nautilus","amendment_kind":"ACCEPTANCE_ID_BINDING","definition_event_id":"<原GOAL_SET event_id>","definition_base_head":"fab1f08a","recorded_head":"<current HEAD>","reason":"bind stable acceptance ids without changing goal semantics","acceptance_vector_hash":"sha256:<原acceptance数组去id的hash>","bindings":[{"ordinal":0,"acceptance_hash":"sha256:<h0>","acceptance_id":"acc-1"},...]}
```

### reducer改动点(5)
1. GOAL_SET时创建immutable acceptance slot: slot_uid=goal_set_event_id+ordinal+acceptance_hash。
2. CHECK_PASS解析:有acceptance_id按(goal_id,acceptance_id)找slot;无id仅历史兼容按(goal_id,check)精确匹配唯一slot(0或多个=invalid)。
3. pass状态记到slot不记文本:passed_slots.add((goal_id,slot_uid))。
4. GOAL_AMEND校验:definition_event_id存在+acceptance_vector_hash等于原始去id canonical hash+每ordinal+hash命中唯一slot+acceptance_id goal内唯一+重复同amend内容no-op+同id不同绑定invalid。
5. amend后新的no-id CHECK_PASS应拒绝或hard warning,fallback只服务历史重放。

## 与编排者裁决A的关系
编排者裁'A=路1 SUPERSEDE'+'跟codex找严格解法'。codex严格化结论:A路径(补稳定id)正确,但**严格形式是GOAL_AMEND而非裸SUPERSEDE**。codex:'若短期必须走SUPERSEDE,必须扩展成IDENTITY_MIGRATION带acceptance_map,但那已等价于GOAL_AMEND,只多制造goal alias+历史转移复杂度'。即GOAL_AMEND=A裁决的严格实现形式。

## 575号关系
GOAL_AMEND保持goal_id不变=不触发goal身份变更,可能不升575(575是goal定义层变更/clean-slate重建)。但新增事件类型(7→8种已含GOAL_RESUME,本次再+GOAL_AMEND=9种)是契约扩展,需genealogist评估。

## 认识论 L0(事件模型设计,codex实读MEMORY+events.jsonl历史)。
