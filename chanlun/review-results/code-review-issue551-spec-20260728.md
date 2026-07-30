# #551 Spec 轴评审（新上下文，只读）

范围 = `git diff 20a14b9dd9...HEAD`（4 文件 / +1605）。规格源 = SPEC #547 全文 + #551 票体验收 + 裁定(i) 甲口径两锁 + 归口三项。独立实跑：`cargo test --release --lib theta_v0::classifier::tests::` = **37 passed / 0 failed / 2 ignored**。

## 结论：**PASS**（无阻断项；1 条 MED 建议收口）

---

## (a) 规格要求但缺失 / 部分

**A1【MED】oracle probe 非真空只覆盖 16 路计数中的 7 路，且报告把未断言的计数器列为证据。**
全仓仅 `birth_unresolved` / `to_provisional` / `first_provable_written` / `observed_pinned` / `invalidated_absent` / `terminal_block` / `idempotent_skip` 被断言非零（`cand_event.rs:1055-1066`、`mod.rs:4696`）。`growth_revision` 与 `invalidated_shrink` 反而只被断言 **==0**（`cand_event.rs:1060,1068`），`birth_provisional` 断言 ==0（`:1059`）；`birth_confirmed`/`to_unresolved`/`to_confirmed`/`payload_revision`/`first_provable_pinned`/`confirmed_pinned` 零断言。
> 票体验收：「oracle probe 覆盖计数：转移/修订/钟路径真触发（防真空绿）」。

而实施报告 §一 行1 称「`birth_provisional`/`birth_unresolved`/`birth_confirmed` 三路探针」、行8 称「`first_provable_pinned` 探针」、§六 红线③ 称「探针 `invalidated_shrink` 计量」——三处所指计数器均无任何断言。违 SPEC #547:96「090：任何测不出/未覆盖项照实登记，禁外推冒充实测」。
（被指路径本身另有直接断言兜底：同状态生长修订见 `mod.rs:2814-2823`；故判「部分」而非「缺失」。）

**A2【LOW】归口三项之一 `truncate-key` 无逐条结案登记。** #550 Spec 评审 LOW-7（级别 truncate 后同 key 永不再生）归口本票，报告 §四/§八 通篇未提。机制上已被 `mod.rs:4493`/`:4643` 两锁覆盖（`invalidate_unseen` 对全级别 key 生效），仅簿记缺一条。

**A3【LOW】转移表禁止边 ∅→Invalidated 无机器锁。** `cand_event.rs:236,250` 对该分支静默 `{}`，无构造性拒绝。
> SPEC #547:59「∅→Provisional/Unresolved/Confirmed」。

## (b) 规格未要求的行为（scope creep）

**B1【LOW】`CandidateKey.rule_version`（`cand_event.rs:27,47`）** 是 SPEC #547:50-51 冻结键形状之外的新分量；且为 `const`、生产永不递增 ⟹「规则版本变 → 新 key」仅由手改字段的单测（`cand_event.rs:1084`）证成，**生产不可达**，却未进 §四 090 登记（与 P→C 不可达的登记口径不一致）。字段为票体验收所必需，只登记不判失。

**B2【LOW】`signal.rs:258` 抽出 `first_structural_gates`** 超出 SPEC #547:42 声明的既有文件改动面（`classify_impl` 接线 + 通道透传 + `TowerCache`）。四态所必需、判定单源合 audit §6，**逐位等价核实成立**（新序仅把 `extreme` 检查后移，纯函数无副作用）。核验限度：其 bit-exact 旁证 `extract_signals_bit_exact_digest_guard` 基线即红，left=`16618955402698307653` 与报告所记一致但无绿锁。

## (c) 看似实装但错了的

**无。** 逐条复核成立：空 L0 当场失效（`mod.rs:1701-1708`，`clear()` 不触 `candidate_book`，`as_of` 与全量路径同式 `:2429`）；`invalidate_unseen` 跳过终态 ⟹ 重复塌空仍 Delta=∅；`merge_episode_leg`（`cand_event.rs:556`）extreme 取析取 + 首证钟取首个全谓词成立腿，与 037:20 等价性注记自洽，且修掉了破 US8 幂等的真缺陷；甲口径两锁（`mod.rs:4408`/`:4493`）的无条件域（非终态）/ 计量域（终态）分解与裁决文本逐条对应，非真空前提 `live_checked > 0` 到位；`nest` 三件 / `CandDeltaEvent` / `NestCandidateEvent` / typed 链 diff = 0；消费方零接线。

---

**建议**：补 `growth_revision` / `invalidated_shrink` / `birth_confirmed` 三路非零断言，并订正报告 §一 行1、行8 与 §六 的证据列（A1）；A2/B1 各补一行登记。三项均不阻断关票。
