# ADR 0025：LAV 旁路排序的证据与失败安全契约

**日期**：2026-08-21
**状态**：Accepted
**裁定链**：上游事实研究 [#1144](https://github.com/xy7365527-lang/NewChanlun/issues/1144) → 唯一职责与接缝 [#1145](https://github.com/xy7365527-lang/NewChanlun/issues/1145) → 本契约裁定 [#1148](https://github.com/xy7365527-lang/NewChanlun/issues/1148)
**范围**：纯架构契约。本 ADR 不裁 class、API、队列、存储后端或部署拓扑，不安装或运行 LAV，也不运行历史 pilot；数值、样本与 pilot 门槛归 [#1149](https://github.com/xy7365527-lang/NewChanlun/issues/1149)。

## 背景

[#1144](https://github.com/xy7365527-lang/NewChanlun/issues/1144) 已查明：上游 `llm-as-a-verifier` 不执行代码、不复核测试、不提供可失败关闭的 correctness gate；固定源码还存在 `cache=None` 路径丢分、错误折成平局、ambient `.env`、endpoint/key fallback 与全局缓存等不能直接带入本仓的行为。因此名字里的 verifier 不构成权威能力。

[#1145](https://github.com/xy7365527-lang/NewChanlun/issues/1145) 随后只保留一个可撤销接缝：LAV **不是 pstack**，在 NewChanlun 的唯一名分是实现后／评审前的非权威**旁路排序器（Shadow Ranker）**。它只消费通过确定性过滤并冻结的候选证据包，不进入 Spec、Implement 或 Code Review，不改变任何工作流状态。

本 ADR 裁定的是这条旁路在进入历史离线 shadow pilot 设计前必须满足的证据、输入、输出、隔离、缓存、失败、审计与删除契约。它不把 LAV 扶正为 verifier，也不预先批准 pilot。

## 一、角色、接缝与权威顺序

1. **唯一角色**：LAV 仅是 Shadow Ranker。它不是 pstack，不承担 Pick／Graft／Verify，不是 verifier、judge、reviewer 或 gate。
2. **唯一接缝**：`post-implement / pre-review`。候选实现完成、冻结的 Gate Profile 所要求的确定性闸门全部通过、Candidate Evidence Bundle 被封存后，才可选地调用 LAV；其后仍进入 Lead 与标准 Code Review 的现役路径。
3. **权威关系**：确定性闸门先于 LAV 且不可被它推翻；`{Lead、标准 Code Review、CI、Lean／形式化、真实行为证据}` 全部高于 LAV。此集合内部的既有权责不由本 ADR 重排；本 ADR 只裁它们都不会被排序覆盖。
4. **状态零写权**：LAV 不得改变 map、ticket、branch、merge、test、review、gate 或 Lead 决策状态。Lead 可忽略排序、另选候选或拒绝全部候选。
5. **Matt 不阻塞**：LAV 永不阻塞现役 Matt 工作流。截止时间到而没有 `RANKED` 时，流程照常继续到 Lead／标准 Code Review；迟到结果只作历史记录，不得重开、回滚或改变已经推进的状态。

这是一条可缺席的研究旁路，不是权威链中的新环节。

## 二、Candidate Evidence Bundle 的内容与身份

### 2.1 规范表示、完整性与内容身份

- `Candidate Evidence Bundle = RFC 8785 canonical UTF-8 manifest + SHA-256 content-addressed blobs`。manifest 是按 RFC 8785（JCS）规范化的 UTF-8 JSON 字节串；证据对象是以 SHA-256 内容寻址的不可变 blobs。
- `bundle_id = SHA-256(canonical_manifest_bytes)`。`bundle_id` 不写回 manifest，禁止自引用；`comparison_set_id` 也**不在 bundle manifest 内**。文件路径、目录名、生成时间或封存时间均不得充当身份。
- manifest 对每个 blob 只引用其 SHA-256 摘要及契约所需角色、媒体类型和字节数；同一摘要必须解析到同一字节内容。任一 manifest 字段或 blob 字节变化都生成新摘要与新 bundle，不得原地补件。
- Bundle 在候选进程终止、worktree 冻结且证据封存器原子发布 `state = SEALED` 后才完整。`DRAFT`、半包和事后追加都不能进入比较；“已封存”只表示契约完整，不表示正确、通过评审或获准采用。
- Bundle 同时承载位于 **Lead-only evidence plane** 的 raw evidence（Bundle Sealer 仅可为确定性封存校验读取）与单独生成的 canonical redacted `ranking_view`；LAV 的可见面只有后者。

### 2.2 精确最小 manifest schema

以下字段名、层级与数组元素是已确认的精确最小 schema；实现不得把必填字段降成语义表、改名，或用隐藏 sidecar 补足：

```json
{
  "schema_version": 1,
  "candidate_id": "...",

  "task": {
    "input_digest": "...",
    "spec_digest": "...",
    "gate_profile_digest": "..."
  },

  "candidate_slot_policy_digest": "...",
  "comparison_context_digest": "...",

  "source": {
    "repository_digest": "...",
    "base_commit_digest": "...",
    "candidate_tree_digest": "...",
    "diff_digest": "..."
  },

  "generation": {
    "run_id": "...",
    "provider_id": "...",
    "owner_vendor": "...",
    "owner_family": "...",
    "model_id": "...",
    "prompt_digest": "...",
    "toolchain_digest": "...",
    "runtime_image_digest": "...",
    "dependency_lock_digest": "..."
  },

  "gates": [
    {
      "gate_id": "...",
      "command_digest": "...",
      "status": "PASS",
      "exit_code": 0,
      "stdout_blob": "...",
      "stderr_blob": "..."
    }
  ],

  "artifacts": [
    {
      "role": "diff",
      "media_type": "text/x-diff",
      "bytes": 1234,
      "sha256": "..."
    }
  ],

  "ranking_view_digest": "...",
  "redaction_policy_digest": "...",

  "sealing": {
    "sealer_digest": "...",
    "seal_policy_digest": "...",
    "redaction_policy_digest": "..."
  },

  "state": "SEALED"
}
```

`comparison_context_digest` 的唯一构造式是对下列对象的 JCS canonical UTF-8 bytes 求 SHA-256；四个输入严格是本 bundle 的 `task.input_digest`、`task.spec_digest`、`task.gate_profile_digest` 与冻结的 `candidate_slot_policy_digest`：

```json
{
  "task": {
    "input_digest": "...",
    "spec_digest": "...",
    "gate_profile_digest": "..."
  },
  "candidate_slot_policy_digest": "..."
}
```

`set_policy_digest`、成员身份、成员顺序、成员数量、bundle／view／set ID 均不得进入这条构造式。

字段不变量：

1. `schema_version` 固定 schema 解释；`candidate_id` 是比较上下文里的不透明槽位标识，不携带候选质量或顺序，并在一个 Comparison Set 内唯一。
2. `candidate_slot_policy_digest` 固定候选槽位的预分配、唯一性与不透明性政策；该政策在候选生成前冻结，同一次比较的所有 bundle 必须逐字共享此摘要，且它不得由任一 `candidate_id`、bundle／view／set ID、成员身份、顺序或数量派生。
3. `comparison_context_digest` 是每份 Candidate Evidence Bundle manifest 的必填字段，必须严格按上列唯一构造式重算。同一次比较的所有 bundle 必须逐字共享它及其四个输入；`source` 四摘要分别固定仓库内容域、基线提交、候选树与实际 diff。
4. `generation.provider_id` 是实际调用服务，`generation.owner_vendor` 与 `generation.owner_family` 是实际模型所有者与家族；`generation.model_id` 是**实际精确模型 ID**（包括服务实际调用的精确快照身份），不得用营销别名、聚合器名、浮动 `latest` 或仅家族名代替。
5. Gate Profile 的每个 required gate 必须逐条出现且 `status = PASS`；`command_digest`、真实 `exit_code` 及 stdout/stderr blob 摘要缺一不可。每个被引用 blob（包括 stdout、stderr、raw evidence 与 ranking view）都必须能在 `artifacts` 中按角色、媒体类型、字节数与 SHA-256 对上。
6. 顶层与 `sealing` 内的两个 `redaction_policy_digest` 必须逐字相同；`sealer_digest` 固定封存器实现，`seal_policy_digest` 固定封存规则。三者均参与 `bundle_id`，不能在封存后替换。
7. manifest 不含时间戳、绝对路径、机器名、用户名、PID 或 mutable URL；这些 operational 数据只可进入外层 Run Record。manifest、receipts 与 artifacts 不保存或要求 hidden chain-of-thought、模型私有推理或“思维过程”。可审计证据只来自显式输入／输出、observable tool call/result、确定性 receipts、固定 provenance 与可重放 artifacts。

## 三、证据封存器与 Evidence Store 写权

**证据封存器（Bundle Sealer）**是一个确定性的非模型角色，也是唯一具有 Candidate Evidence Store 的 bundle／Comparison Set **创建与封存发布写权**、能把 bundle 原子发布为 `SEALED`，并在成员 bundle 全部 `SEALED` 后原子发布 Comparison Set Manifest 的角色。

封存前它必须：

1. 等候 candidate 进程终止并冻结 worktree，再从冻结面收集证据；candidate 自报“测试通过”不算 gate evidence；
2. 重新执行 required gate，或从确定性来源重新核验其真实 command、exit、stdout/stderr 与 receipt；封存器不得调用模型或解释测试结果；
3. 重算 JCS manifest、每个 blob、source/base/candidate tree/diff 与 ranking view 的摘要、长度和内容寻址关系，并按 2.2 的唯一构造式重算 `comparison_context_digest`；
4. 验证 frozen Gate Profile 的 required gate 完整且逐项 `PASS`；
5. 验证 ranking view 已按 manifest 中同一份 `redaction_policy_digest` 生成且安全；
6. 验证 `candidate_slot_policy_digest`、`sealing.sealer_digest`、`sealing.seal_policy_digest`、两个 redaction policy digest 与本次实际封存输入逐字一致；
7. 把 manifest、所引用 blobs 与可见状态作为一个原子结果发布，不允许半包或事后补件。

Candidate、LAV、reviewer 与 Lead 都没有 Candidate Evidence Store 的创建、补写、改写或发布权，也不能把 bundle 标成 `SEALED`；Lead 对 raw evidence 的只读权不等于发布权。LAV 的 Evidence Store 只读挂载必须由访问控制收窄到 `ranking_view`，不得借“只读”取得 raw plane。第十一节确定性 GC 的删除窄权不构成封存发布写权。

若 Bundle Sealer 自身发生执行、存储、`fsync` 或原子发布故障，bundle 封存失败时**不产 bundle**，集合封存失败时不产 Comparison Set Manifest；外层 supervisor 必须使用预留 `run_id` 发布 `status = INFRA_FAILURE`、`failure_stage = BUNDLE_SEAL` 或 `COMPARISON_SET_SEAL` 的不可变 Run Record。若 Sealer 的确定性校验因 schema、digest、同域或绑定不成立而拒绝，则走 `INVALID_INPUT`；无法安全生成 canonical redacted `ranking_view` 则走 `SKIPPED`。三类都不发布对应 bundle／set，也不得静默消失或互相伪装。

首轮本地历史 pilot 的可信边界依靠本 ADR 的隔离、内容寻址、重新校验与原子发布，**不需要签名密钥**。任何跨主机导出在另行裁定签名方案之前都不成立；不得把“以后可能签名”冒充当前已有保证。

## 四、Gate Profile 与 Comparison Set

### 4.1 Gate Profile 准入

1. **先冻结**：任务在生成候选前冻结 Gate Profile；它完整列出 required／optional gates、receipt 契约与版本，封存或排序后不得追改。
2. **逐项全 PASS**：只有 required gates 全部 `PASS` 的候选可进入排序。`FAIL / ERROR / TIMEOUT / SKIP / UNKNOWN` 候选被确定性过滤排除，永不进入 LAV，也不能由 LAV 救回；本契约不强迫运行 Gate Profile 未要求的无关全仓测试。
3. **同一输入域**：进入同一比较的 bundle 必须共享 `task.input_digest`、`task.spec_digest`、`task.gate_profile_digest`、`candidate_slot_policy_digest` 与由这四项唯一导出的 `comparison_context_digest`，并通过下节的 Comparison Set Manifest 形成封闭成员集。任一摘要、候选槽位或 view 绑定不一致都是 `INVALID_INPUT`，不得自动拆组或凑组。
4. **至少两个候选**：确定性过滤与封存后 `N < 2` 时状态为 `SKIPPED`，不调用排序模型。

### 4.2 独立的 canonical Comparison Set Manifest

Bundle 不反向引用集合。全部成员 bundle `SEALED` 后，Bundle Sealer 才能另行构造并原子发布以下 RFC 8785 canonical UTF-8 manifest：

```json
{
  "schema_version": 1,
  "comparison_context_digest": "...",
  "member_bundle_ids": ["...", "..."],
  "ranking_view_digests": ["...", "..."],
  "member_count": 2,
  "set_policy_digest": "..."
}
```

精确语义如下：

- Comparison Set Manifest 的 `comparison_context_digest` 是成员 bundle 内同名必填字段的逐字重复，不是集合层重新定义的摘要。它的构造式仍且只能是 2.2 的三个 `task` 摘要加 `candidate_slot_policy_digest`；`set_policy_digest` 不得参与。
- `set_policy_digest` 是与 `candidate_slot_policy_digest` 分离的集合政策输入，固定成员准入、有序 membership、`member_count` 与其他 set policy；实际有序成员、对应 views 与数量另由本 manifest 的两个数组和 `member_count` 固定。数组顺序只服务规范身份，不表达任何偏好，更不得在失败时变成默认排名。
- `member_bundle_ids` 与 `ranking_view_digests` 是等长、按位置一一对应的有序数组；`member_count` 必须等于两者长度且至少为 2。bundle ID、view digest 与从成员 bundle 解出的 `candidate_id` 均不得重复；每个 view digest 必须等于对应 bundle 的 `ranking_view_digest`，该 view 内的 opaque `candidate_id` 也必须与对应 bundle 的 `candidate_id` 逐字相同。由此每个成员位置形成 bundle ↔ view ↔ candidate 三者唯一双射。
- Sealer 必须对**每一个**成员 bundle，从该 bundle 自身的三个 `task` 摘要与 `candidate_slot_policy_digest` 重算 `comparison_context_digest`，验证重算值等于该 bundle 的同名字段，再验证所有成员的同名字段逐字相同且等于 Comparison Set Manifest 重复的值；任一步不成立都不发布集合并走 `INVALID_INPUT`。
- `comparison_set_id = SHA-256(canonical_comparison_set_manifest_bytes)`，且不写回 manifest。成员、顺序、view、数量或 policy 任一变化都生成新的 manifest 和 set ID；发布后的 membership 永不可变。
- Bundle 的 `comparison_context_digest` 不含 `set_policy_digest`、成员身份／顺序／数量或任何 bundle／view／set ID，bundle 本身也不含 `comparison_set_id`；Comparison Set Manifest 只引用既成 `bundle_id`、重复其非集合派生的上下文摘要，再由完整 manifest 生成 `comparison_set_id`。因此身份图仍是 `bundle_id → Comparison Set Manifest → comparison_set_id` 的单向闭包，不存在 bundle/set 哈希循环。
- `RANKED` 的 coverage 必须恰好覆盖 manifest 全部成员 bundle 中解析出的唯一 `candidate_id`，不能遗漏、增加或只排容易的子集。

## 五、给 LAV 的唯一数据面：canonical redacted ranking_view

LAV（包括其模型、进程与工具）只能看到 canonical、已脱敏的 `ranking_view`。Bundle 内 raw diff、source、logs、tool evidence 与完整 provenance 留在 Lead-only evidence plane；manifest 中的 raw blob 引用不赋予 LAV 解析、路径或读取权，只有证据封存器可为确定性校验读取。

`ranking_view` 及其执行环境中一律不得出现或可解析得到：

- credentials、tokens、keys、cookies 或任何 secret；
- environment 内容或环境变量转储；
- absolute paths、host names、user names；
- URLs、images、local-file references，或任何 URL／图片／路径解析能力；
- hidden answers、历史 ground truth、`LeadBaselineDecision`、Lead outcome；
- unrestricted logs、raw tool transcripts、其他候选的排名／身份信息，或可借此还原上述内容的旁路引用。

脱敏不是字符串遮盖后继续发送原对象，而是只发布策略允许的 canonical projection。`ranking_view` 使用 JCS UTF-8 JSON、有独立 SHA-256、绑定 `redaction_policy_digest`，并与 bundle 一起封存后不可修改。若不能构造安全且满足固定 ranking-view schema 的视图，整个 LAV 接缝为 `SKIPPED`；不得回退到 raw evidence、删减策略或人工临时放行。

每份 `ranking_view` 必须含一个且仅一个顶层 opaque `candidate_id`，供 LAV 返回相对排序；Sealer 必须验证它与所属 bundle manifest 的 `candidate_id` 逐字相同。`candidate_id` 是比较上下文预先分配的槽位，不得由 `bundle_id`、`ranking_view_digest` 或 `comparison_set_id` 派生，也不得携带模型、候选质量或输入顺序信息。

## 六、输出契约与失败状态

### 6.1 唯一合法的排名输出

`RANKED` 的业务输出只允许：

- 对 opaque `candidate_id` 的**有序 tie groups**：组间有序，组内明确同列；
- 恰好覆盖 Comparison Set 全部且每个 ID 只出现一次；
- 至少两个非空 rank levels。全部候选落入单一 tie group 不构成排名；
- 固定输入、provider、endpoint、model、source、dependency、runtime、parser、prompt、criteria 与 policy provenance；
- 完整的实际 usage 和 request receipts。

不确定性只由状态语义表达，禁止输出 scalar score、confidence、概率、星级或把它们藏进 metadata；禁止任何 correctness、pass/fail、approve/reject、verifier、judge 或 gate 主张，**包括 advisory／“仅供参考”版本**。LAV 也不得输出 `ALL_BAD` 或借排名暗示“全部可接受”。

### 6.2 状态枚举

`ShadowRankingRun.status` 只能是：

| 状态 | 契约语义 |
|---|---|
| `RANKED` | 完整集合形成符合 6.1、至少含两个 rank levels 的有序 tie groups |
| `UNKNOWN` | 执行与审计输入完整，但冻结 criteria 下不能形成非平凡稳定排序；**all-tie、偏好循环或不稳定聚合均在此态** |
| `INFRA_FAILURE` | sealer、provider、transport、process、deadline、parser、cache、usage、request receipt 或运行中 budget 等基础设施未满足契约 |
| `INVALID_INPUT` | bundle、set、摘要、pin、同域条件、恢复身份或输入 schema 不成立 |
| `SKIPPED` | 命中本 ADR 明定的自动跳过条件，未调用或未继续排序模型 |

只有 `RANKED` 可以含 `ranked_groups`；其余四态必须完全省略该字段。禁止 partial ranking、failure-as-tie、默认输入顺序、随机顺序、以缺失候选补平局，或把失败转换成任何排名。usage 缺失必须如实记为缺失并阻止 `RANKED`，不得按零请求、零 token 或零金额记账。

## 七、外部 provider 的隔离、pin 与模型身份

任何外部 provider 调用必须同时满足：

1. **空执行面**：empty HOME、empty cwd、env-clean；仅显式 allowlist 的非敏感运行变量可见，禁止自动加载 `.env` 或接受 ambient `OPENAI_BASE_URL` 一类覆盖。
2. **先验 endpoint allowlist**：在加载任何 secret 前先验证 endpoint；未命中即停止。
3. **endpoint-key 一一绑定**：每个允许 endpoint 只绑定一把明确授权的 key，每把 key 也只绑定该 endpoint；不得 key fallback、endpoint fallback、provider fallback 或读取 ambient secret。
4. **最小 secret 暴露**：secret 仅以 least-privilege mount 或 FD 注入调用进程，不进入参数、文件、日志、cache、manifest、view、request receipt 或 response artifact。
5. **egress allowlist**：网络只允许固定 provider endpoint 所需目的地；不得任意 DNS／HTTP 出网。
6. **精确 pin**：provider endpoint、实际 `provider_id`、`owner_vendor`、`owner_family`、`model_id`、LAV source revision、dependencies、runtime、parser、prompt、criteria 与相关 policies 全部固定并进入 run provenance／identity；其中 `model_id` 必须是服务实际调用的精确模型 ID，浮动 latest、版本范围或服务端自动换模均不成立。
7. **零宿主挂载**：不得挂载 repo、HOME、GitHub、SSH、cloud credentials 或宿主配置；只读输入只含固定运行物与脱敏 `ranking_view`，不得给模型或工具 images、URL fetch、local path resolution 能力。

`provider_id` 记录实际 API／托管服务，`owner_vendor` 与 `owner_family` 记录实际被调用模型的所有者和家族，`model_id` 记录其实际精确模型 ID；聚合器、代理网关或托管 provider 不能把自己的名称冒充模型 owner。无法证明实际 owner／family／exact model 时身份不完整，不得调用或声称 `CROSS_VENDOR`。

任何隔离条件无法满足时都不得调用 provider，更不得降级到较宽权限。

## 八、run-local cache、request receipts、BudgetEnvelope 与重试

### 8.1 Fresh run-local cache

- 每个进入 cache 构造／排序执行阶段的 run（包括 Execution Rerun）必须且只能创建一份**全新、run-local、内容寻址**的工作 cache；在该阶段之前终结的 `SKIPPED`／`INVALID_INPUT`／sealer failure run 依 9.1 省略 `run_cache_id`，不得伪造空 cache。已创建的 cache 在 run 结束后不得供另一个 run 复用。
- `run_cache_id` 必须覆盖全部语义输入：`comparison_set_id`、有序 member bundle/view digests、provider endpoint、`provider_id`／`owner_vendor`／`owner_family`／`model_id`、`independence_class`、LAV source／dependencies／runtime／parser、prompt／criteria、repeat／direction／job 参数、candidate-slot／redaction／seal／set policies 与冻结的 `BudgetEnvelope` digest。秘密值本身不进 identity。
- crash resume 只允许回到**同一个 `run_id`**，且完整 `run_cache_id` 与所有内容摘要逐字一致；任何缺项或不一致均为 `INVALID_INPUT`，不得猜测、部分恢复或自动新开语义不同的 cache。
- cache manifest 必须记录全部 jobs、输入／输出 digest 与状态，并原子封存为 Audit Replay 证据；以后只能校验，不能作为另一个 run 的工作 cache。
- 禁止走上游 `cache=None` bug path，也禁止继承上游 global cache、跨 run cache 命中或“相似输入可复用”语义。

### 8.2 BudgetEnvelope 与 request receipt

- 每个 run 在任何 provider 请求前冻结一份 `BudgetEnvelope`，覆盖请求数、并发、deadline、输入／输出 token、金额及输入／输出／view 大小等硬上限；具体数值归 #1149，但缺失 envelope 不等于无上限。
- 初始 precheck 必须证明**完整 planned comparisons 的最坏情况**可装入 envelope；缺失或装不下即 `SKIPPED`，不得发出第一条请求。run 已开始后，在**每一条请求及每次 retry 之前**再次按剩余工作的最坏情况检查剩余请求、token、金额和 deadline；装不下即 `INFRA_FAILURE`，不得缩题求成。
- 每次 provider request（首发和 retry）各自产生一份不可变 request receipt。它始终绑定 `request_id`、job、attempt ordinal、endpoint 与四项模型身份、request digest、开始时间、deadline、终态、failure stage／reason／observability，以及请求前后 BudgetEnvelope 余额；retry 还必须绑定 `retry_of_request_id`，首发省略该字段。请求终结后还必须记录结束时间。收到 provider response bytes 时，`stored_response_digest` 必填；实际 usage 可观察时，usage 必填。连接中断、deadline 或更早阶段失败而未形成 response／usage 时，只能省略尚未形成的字段并由 stage、reason 与 observability 明确解释，禁止空串、零值或虚构占位。缺 receipt，或成功请求缺 response／usage，都会阻止 `RANKED`。
- request receipt 是一个 run 内的请求级证据，不是新的 Run Record；一个 run 可以有多份 request receipts，但最终恰有一个不可变 `ShadowRankingRun`。

### 8.3 Retry 转移

- 只允许对已分类的 transient failure（连接中断、408、429、明确允许的部分 5xx），在同一 endpoint、同一 `provider_id`、同一 `owner_vendor`／`owner_family`／`model_id`、同一 prompt、criteria 与全部 pins 下有限重试；`Retry-After`、剩余 deadline 与 BudgetEnvelope 必须同时允许。
- 401／403、schema、digest、redaction、parser、cache mismatch、usage／receipt 缺失、nontransient 或 budget failure 均不重试。
- 禁止 provider／model／endpoint／prompt／criteria failover；也不得静默降低 thinking、缩短输出、删除 criteria 或减少 comparisons。换任一语义输入都是新 run，不是 retry。
- 任一 planned job 在允许重试后仍失败，整个 run 为 `INFRA_FAILURE`；不得用另一 provider 填洞或输出部分排名。
- retry 次数、backoff、并发、deadline、请求、token 与金额硬上限的具体值归 #1149；“数值待定”不允许无界执行。

## 九、原子 Run Record、状态机、截止时间与关系字段

### 9.1 ShadowRankingRun 最小内容

一次 orchestration attempt 在 Bundle Sealer 开始前就预留唯一 `run_id`，并最终恰好发布一份不可变 `ShadowRankingRun`。Run Record 至少记录：

| 分组 | 必填内容 |
|---|---|
| 输入身份 | `run_id`；存在时的 `comparison_set_id`、有序 bundle/view refs；封存前失败只记录已经存在的身份，不伪造占位值 |
| ranker 身份 | `ranker.provider_id`、`ranker.owner_vendor`、`ranker.owner_family`、`ranker.model_id`（实际精确模型 ID）、endpoint digest、`independence_class` |
| 语义 pins | source、dependency、runtime、parser、prompt、criteria、candidate-slot／redaction／seal／set policy digests |
| 资源与请求 | `BudgetEnvelope` digest、`run_cache_id`、全部 request receipt digests、完整 usage 或明确缺失 |
| 关系 | 可选且互斥的 `replays_run_id`、`supersedes` |
| 结果 | 五态之一、coverage、机器可判读的 failure／skip stage 与 reason；仅 `RANKED` 可有 `ranked_groups` |

上表按终结阶段判必填：`run_id`、status、stage 与 reason 永远存在；某字段在终结前已经形成就必须记录。若 attempt 在该字段可能形成前终结（例如 sealer 失败时尚无 `comparison_set_id`／`run_cache_id`，或 `NO_INDEPENDENT_RANKER` 时尚无 ranker 四元组），只能按 schema 省略并由 stage/reason 解释，禁止写空串、零值或虚构占位。只要发出过 provider request，完整 set、ranker 四元组、pins、BudgetEnvelope、cache、receipts 与 usage 字段便全部必填。

### 9.2 原子发布

所有内容先写 staging，逐项校验 schema、长度和 digest，`fsync` 后原子 rename 到内容寻址目录；不完整 staging 不得可见。正常执行者只产 staging，外层 supervisor 负责以 compare-and-swap 方式发布最终 Run Record，保证每个 `run_id` 恰有一个终态。所有 Run Records 与 artifacts append-only，不得覆盖、编辑或“修正”；`latest` 索引若存在也只是可重建目录，不是权威事实。

### 9.3 完整状态机

1. **预留**：先冻结输入 pins、可用时的预定 ranker 四元组、BudgetEnvelope、deadline 并预留 `run_id`。
2. **封存／预检**：契约明定的非可用条件走 `SKIPPED`；摘要、schema 或同域不成立走 `INVALID_INPUT`；sealer 执行失败走 `INFRA_FAILURE` 且不产 bundle。
3. **请求**：首发与 retry 均留独立 receipt，但仍属同一 run；provider、parser、cache、usage、receipt 或运行中 budget 失败均走 `INFRA_FAILURE`。
4. **语义收口**：完整执行后 all-tie、偏好循环或不稳定结果走 `UNKNOWN`；只有覆盖全集且至少两个 rank levels 才走 `RANKED`。
5. **进程死亡**：若正常发布前进程死亡，supervisor 用预留 `run_id` 原子发布最小 `INFRA_FAILURE` Run Record；静默消失不成立。
6. **deadline**：到 deadline 仍无最终记录时，supervisor 原子发布 `INFRA_FAILURE / DEADLINE_EXCEEDED`，Matt 立即旁路继续。正常发布与 deadline 发布竞争同一终态槽，只能有一个成功。
7. **late**：deadline 终态之后到达的进程结果不得覆盖或追加到 Run Record；只可另存 append-only `LateResultRecord`，必须含 `run_id`、`late = true`、到达时间及所见 response/artifact digests，标明 historical-only，且不得含 `ranked_groups`。它不能改变候选、Code Review、ticket、branch 或 merge。

新 run 一律使用新 `run_id`。`supersedes` 只表示显式的**语义替代**，例如输入或 policy 已变的新评估；它不删除旧记录，也不是 replay。Execution Rerun 必须使用 `replays_run_id` 且省略 `supersedes`；二者不得同时出现。

## 十、Replay、历史盲法与独立性

### 10.1 Audit Replay 与 Execution Rerun

- **Audit Replay**：不调用 provider。它必须按 Run Record 的终结阶段，重算当时应已形成的全部 bundle／set manifests、blobs、ranking views、Gate Profile receipts、cache manifest 与 request receipts，并验证按 9.1 本就尚未形成的对象确实没有被伪造。凡该 run 存有 provider response bytes，Audit Replay 必须读取其**原样保存的 bytes**，用 Run Record 固定的 `parser_digest` 重新解析，重建 status 与 canonical ranking/failure output，并与当时发布的 bytes **逐字节一致**。它还必须证明所有实际发布 artifact 未改动。
- Audit Replay 的结果另存 append-only audit receipt，不编辑原 Run Record。任一应存在对象缺失、意外多出对象、digest、receipt、重解析或 byte comparison 失败，原 Run Record 即不可审计、不可用于 pilot 结论；不得退化成“尽量相似”。按阶段从未形成的对象不是删除；曾形成但已按第十一节删除的证据返回 `EVIDENCE_EXPIRED`，两者不得混同或伪装成功。
- **Execution Rerun**：重新调用相同 provider／endpoint／精确 model，并使用同一 comparison set、bundles、views、prompt、criteria、source、dependency、runtime、parser 与 policies；它必须生成新 `run_id`、以 `replays_run_id` 指向旧 run、创建全新 run-local cache、重新计费和留 request receipts，且不得覆写旧 artifact。外部非确定性允许新排名不同，差异本身是校准数据，不是 Audit Replay 失败。
- 若原 exact model snapshot 已不可调用，就不能冒充同身份 Execution Rerun；只能显式建立语义已变、使用 `supersedes` 而非 `replays_run_id` 的新评估，并照实记录实际身份。

### 10.2 历史 pilot 的盲法

- `LeadBaselineDecision` 必须在任何 LAV 结果揭示给 Lead／比较者之前先行封存，且封存后不可改写；它记录 `selected_candidate_id` 或 `REJECT_ALL`、理由与确定性证据 refs。
- LAV 永远看不到 ground truth、hidden answers、Lead baseline、最终 merge 候选或最终 outcome。Lead 的候选选择、Code Review 与 merge 不等待 LAV。
- `ShadowComparison` 是 baseline 与 LAV run 之后生成的独立记录；它只能引用二者并比较，不能回写、解释性修改或替换 `LeadBaselineDecision`／`ShadowRankingRun`。揭示后的新观察只可写 post-shadow annotation。

本 ADR 只立将来 pilot 的盲法契约，不创建 baseline、不做 comparison、不运行 pilot；未来 live workflow 是否提前展示 LAV 必须另行裁定。

### 10.3 Independence class 的机械互斥判定

Candidate Evidence Bundle 的 `generation` 始终必须记录四元组 `generation.provider_id`、`generation.owner_vendor`、`generation.owner_family`、`generation.model_id`；凡已选定 ranker 的 ShadowRankingRun，其 `ranker` 也必须记录同名叶键组成的四元组 `ranker.provider_id`、`ranker.owner_vendor`、`ranker.owner_family`、`ranker.model_id`。两处 `model_id` 都是实际精确模型 ID。仅在 ranker 选定前以 `SKIPPED: NO_INDEPENDENT_RANKER` 终结的 run 可依 9.1 的阶段规则省略 ranker 四元组；只要选定 ranker，尤其只要发出 provider request，四项就全部必填。`provider_id` 不参与“模型是否同源”的伪装；分类比较的是实际 model owner 身份。

令 `g = Candidate Evidence Bundle.generation`、`r = ShadowRankingRun.ranker`，严格按以下 first-match 优先级且只命中一类：

1. `SAME_MODEL`：`g.owner_vendor = r.owner_vendor` 且 `g.model_id = r.model_id`；
2. 否则 `SAME_FAMILY`：`g.owner_vendor = r.owner_vendor` 且 `g.owner_family = r.owner_family`；
3. 否则 `SAME_VENDOR`：`g.owner_vendor = r.owner_vendor`；
4. 否则 `CROSS_VENDOR`。

即 `SAME_MODEL > SAME_FAMILY > SAME_VENDOR > CROSS_VENDOR`。一次 set 有多个 generator 时，run 的 `independence_class` 取所有 generator/ranker 配对中优先级最高、也就是**最不独立**的一类；只有 ranker 与每个 generator 都跨 owner vendor，run 才是 `CROSS_VENDOR`。

聚合器或托管平台即使 `provider_id` 不同，也不能改变底层 `owner_vendor`／`owner_family`／`model_id`；身份无法核实时不得判成 `CROSS_VENDOR`。前三类只作 common-mode controls，不能支持采用结论；**只有 `CROSS_VENDOR` 可以支持未来 GO**。没有可用的 independent ranker 时为 `SKIPPED: NO_INDEPENDENT_RANKER`，不得回退同源后冒充独立验证。具体 arms 与样本量归 #1149。

## 十一、删除与证据过期

删除由 Evidence Store 的**确定性 GC**独占执行；LAV、candidate、reviewer 与 Lead 均无删除权。GC 只能按冻结 retention policy、legal hold 与活跃审计引用机械判断，不能调用模型或作内容判断。这是对 Bundle Sealer 封存发布独占权的唯一窄例外：GC 只能删除本节指定的敏感对象并在 append-only 审计面追加 `DeletionReceipt`，不得创建、补写、改写或重新发布 bundle、Comparison Set Manifest、Run Record 或其他 immutable record。

每个被删除 bundle 必须在 append-only 审计面追加且只追加一份以下精确 `DeletionReceipt`：

```json
{
  "schema_version": 1,
  "deleted_bundle_id": "...",
  "affected_run_ids": ["..."],
  "deleted_blob_digests": ["..."],
  "retention_policy_digest": "...",
  "deletion_reason": "TTL_EXPIRED",
  "deleted_at": "...",
  "deleter_digest": "..."
}
```

`affected_run_ids` 与 `deleted_blob_digests` 必须去重并按固定字节序排序；`deleter_digest` 固定 GC 实现，`retention_policy_digest` 固定实际生效政策。精确 reason 枚举、TTL、legal hold 与删除批次归 #1149，但上述字段不得省略、改名或由 sidecar 补足。

GC 删除 ranking views、Lead-only raw evidence、provider responses 与 run-local caches；immutable bundle manifest、Run Record、Lead baseline、ShadowComparison 与其 ID／digest／状态／策略引用不原地修改。Tombstone 只保留上列删除事实，**不得**保留源码、日志、prompt 正文、模型输出、secret、路径、用户／主机信息或其他敏感 metadata；LeadDecision 也只能保留旧 bundle/run digest 引用，不能复制被删内容。

未到 retention 期限、处于 legal hold 或仍有活跃审计引用时不得删除。删除后，任何需要被删内容的读取、Audit Replay 或证据解析都返回 `EVIDENCE_EXPIRED`；不得用空内容、零 usage、缺失 blob 或摘要占位冒充成功重放。

## 十二、自动跳过与相邻失败的精确边界

以下条件不启动或不继续 LAV，统一产生带机器 reason 的 `SKIPPED` Run Record，且不影响 Matt：

1. required gates 全 PASS 且 `SEALED` 的合格候选少于两个；
2. 任一成员无法生成契约完整且安全的 canonical redacted `ranking_view`，或发现 ground-truth／Lead outcome leakage；
3. 没有可用的 `CROSS_VENDOR` independent ranker；
4. provider endpoint/key 不在预先 allowlist，或隔离执行面在加载 secret 前判定不可用；
5. `BudgetEnvelope` 缺失，或初始完整计划的最坏情况装不下；
6. 输入、输出或 view 尺寸超过由 #1149 固定的硬上限。

确定性 gate failure 候选只被排除，永不进入 LAV；bundle/set/schema/digest/pin/同域身份不一致走 `INVALID_INPUT`；sealer 或启动后的 provider／process／deadline／parser／cache／usage／receipt／budget 失败走 `INFRA_FAILURE`；完整执行但 all-tie／cycle／不稳定走 `UNKNOWN`。这些状态都不得伪装成 skip 后的默认排名。

## 十三、后果与代价

- **失败安全**：LAV 的缺席、失败、迟到或语义不确定都不能放行候选、救回 gate failure 或卡住 Matt。
- **证据可核**：bundle 与 set 的非循环内容身份、唯一 Sealer 封存发布权、GC 仅删除敏感对象／追加 `DeletionReceipt` 的窄权、原子 Run Record、request receipts 与非成功态留档，使“排了什么、没排成什么、花了多少”都可追溯。
- **数据最小化优先**：LAV 看不到 raw diff/source/log/tool evidence，排序信息量会下降；无法安全投影时接受 `SKIPPED`，不以扩大出境面换覆盖率。
- **可用性让位于同一语义**：禁止 fallback、跨 run cache、单层全并列与部分排名会增加 `INFRA_FAILURE`／`UNKNOWN`，这是避免静默换题与 failure-as-tie 的明确代价。
- **跨厂商才有决策价值**：same-source 实验只能校准共因失效，不能支持 GO；独立 model owner 不可用时接受无排名。
- **删除会终止完整 replay**：敏感内容可删除，但删除后只能靠无敏感正文的 `DeletionReceipt` 证明发生过删除，不能继续声称证据可重建。
- **没有当前生产收益**：本 ADR 不安装、不运行、不接活流程；在 #1149 裁完且后续另有实施授权前，它只是一份契约。

## 十四、明确留给 #1149 的数值与样本决策

[#1149](https://github.com/xy7365527-lang/NewChanlun/issues/1149) 必须另行裁定，且本 ADR 不预填：

1. 历史样本单位、任务清单、样本量、分层方法、每任务候选数，以及 calibration／test／injection sets 的隔离比例；
2. pilot 采用的具体 generation/ranker provider、owner vendor/family、exact model 与 `independence_class` arms；精确 source/dependency/runtime/parser/prompt/criteria pins 的取值；
3. `candidate_slot_policy_digest` 所绑定的具体候选槽位预分配、唯一性与不透明性政策，`set_policy_digest` 所绑定的有序 membership、成员准入／计数与其他 set policy，以及单 run 与全 pilot 的 concurrency、request、retry、backoff、timeout、deadline、token、金额、输入／输出／view 大小与 cache 容量硬上限；
4. 指标及计算口径：相对选择准确率／regret、tie、历史 ground truth 下的 all-bad 分层、与 Lead／CI 分歧、prompt-injection 成功、infra failure、P50／P95 latency、请求／token／金额；其中 all-bad 只可由独立 `ShadowComparison` 评估，LAV 输出仍禁止 `ALL_BAD`；
5. GO／NO-GO／inconclusive 的样本门槛、置信规则与停止线；只有 `CROSS_VENDOR` 样本可支持 GO；
6. 精确 retention 天数、各类 artifact 的保留／删除期限、`deletion_reason` 枚举、legal hold、活跃审计引用、删除批次、pilot 结束后的删除证明与退出时点。

跨主机导出的签名算法、密钥治理与信任根不在首轮本地 pilot 范围；如未来需要跨主机，必须再作明确签名裁定，不能由 #1149 的数值选择顺带带过。
