# 收据 workers/uo-independent-review-0028/REVIEW.md

- 原件：`workers/uo-independent-review-0028/REVIEW.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`7ad3a946b06134bebe5f551015d7255c7d6c80b2977f268729eaf5ee122973c5`
- 字节数：11993；行数：102

## 关键行（行号: 原文）

```
1: # #1488 统一所有权预算补丁复审（0028）
4: - 返修基线：`b6969af4cc`（0026 评审对象）
5: - 依据：control/1488-uo-review-0024.md 评审项 1–7；0024/0026 REVIEW+RESULT；0027 REPORT/NOTES；`git diff b6969af4cc 4022f12079 -- rust/ .github/`；#1488 主控裁定（P2-A、fixed_radix :755/:775 追认）
8: ## 结论：**FAIL**
10: 0027 的方案方向正确：按物理 Ledger 分组，用收尾静止点 LedgerMark 做 attest 对拍，同父时拒绝保留池，keeper 池按 K 的物理 live 计费，source_permit 降为合同角色。P1-2、P3-a..d 均已解决。
12: 但 P1-1 的核心要求「无法证明映射时拒绝，不吸收」在真实 driver 重试路径上仍可绕过：`discard_after_error` 会把静止点读数 refresh 成当前 live，等于把来源不明的许可「洗白」进静止点，第二次分区就会把它吸收进 U 并照常冻结。这正是 0026 的 3000B 反例，只是多走了一次 discard。因此本轮判 FAIL。
14: ## 一、0026 各条状态
18: | P1-1 U 聚合残差 / 同父来源不明须拒 / 同父拒绝保留池 | **PARTIAL** | 同父拒绝保留池已落地（audit_owner.rs:885-890，request_audit.rs:128-134）→ 该子项 RESOLVED；直接分区时 attest 不符会拒（request_audit.rs:149-157 及 seal）；**但 driver 重试路径经 discard 后 refresh 吸收 stray（新 P1-α）**；U 仍为逐组残差，未拆到 v4:38 各项；E 侧（legacy lineage）对 L0 无映射，也无主控豁免 |
19: | P1-2 keeper 父≠候选父时按物理 live 计 | **RESOLVED** | keeper 池有 home 时 O_K 按 K 的物理 live 加 M_K 计（request_audit.rs:128-147，audit_owner.rs:1246-1288，cross_request_source_cache.rs:605-615）；driver 仍装不下时只丢可选池（driver.rs:333-341）；锁 audit_owner.rs:3492。无 Ledger 的池按合同额计，属已声明的设计限制 |
20: | P2-A 正文双计 | **PARTIAL** | 机制已落地：SourceClaim 只记合同额，C5 改为 claims==(nodes, body+digest) 并对拍多重集（audit_owner.rs:3285-3375，s_content.rs SourceClaim::on_body 要求 native_bound）；`attach_native_reservation` 遇已有 permit 时拒绝（重复 reserve 负控）。**但「111,334B 来源」的写法与实测不符（P2-β）** |
26: ## 二、Findings
28: ### P1
30: **P1-α（CONFIRMED，代码路径直读）discard_after_error 的 LedgerMark::refresh 在 driver 重试路径上吸收来源不明的许可**
39: - 结果：v4:25「无法证明映射时拒绝、不用 max 补差」被绕过，与 0026 P1-1 反例等价。锁 p1_2（audit_owner.rs:3492）的顺序是先 discard、再挂 stray、再直接分区，没有覆盖「先挂 stray、再经 driver 走 discard 重试」这个顺序。p1_1 锁（request_audit.rs:2771）只测直接分区。
42: **P1-1 剩余部分（并入 PARTIAL，不单列）**
47: ### P2
49: **P2-β P2-A「父账 live 少 111,334B」的口径与实测不符，16,640B 差额未说明**
52: - 实测（P24_SEAL_CENSUS）：
56: - 推测（未验证）：差额来自 NativeNode 去掉 `Option<NativeReservation>` 字段后元素变小，导致按 `hash_slots`（audit_retention.rs:135-144）计费的 index 许可同步缩小。这是正常的连带减少，但主控裁定要求「写明 111,334B 来源」，现在的写法把总降幅说成 111,334B，与物理数据不一致。
59: ### P3
66: ## 三、评审项 1–7 重做
68: 1. **反例 A–F**：PASS_WITH_FINDINGS。A/B/D/E/F 仍 PASS，adopt 族入口与事后预留均已退出生产；C（来源不明须拒）在直接分区时拒，但经 driver discard 重试时被吸收 → P1-α。
69: 2. **前轮 findings**：PARTIAL。见第一节：P1-2、P3-a..d RESOLVED；P1-1 PARTIAL；P2-A PARTIAL。
70: 3. **assert 保护**：PASS。删除的 assert 只在 p1_1 锁（0025 自加的锁，已改写为拒绝语义）和 s_content 测试助手中；fixed_radix（:755/:775 计入 M 已追认）、六 driver、128/1024、16/8MiB、R13 的断言均未动。
71: 4. **r14 恒等**：PASS_WITH_FINDINGS。C5 改为多重集逐张对拍，claims 计数正确；新增 `sealed_bytes==sealed`、`node_source_claims==nodes` 有效；`node_source_physical_permits==0` 恒真（P3-α）；111,334B 来源口径不实（P2-β）。
72: 5. **业务语义**：PASS_WITH_FINDINGS。SQL、业务错误顺序与 1024 路径未变。新增的生产行为全部在 native feature 内（生产 `s_session` 构建不编入）：
78: 6. **冷根重跑**：PASS。无票重跑原样返回 SQL 错误；新增残留复核分支 fail-closed（P3-δ）。
79: 7. **测试证据**：PASS。
82:    - CI run 39308507716：headSha = 4022f120…。job s_session 为 success（1 PASS / 50 NOT_LISTED）；job native 为 failure（47 PASS / 4 FAIL，exit 101，与本地 4F 文案相同）。locks.tsv 与锁清单一致；重新下载 artifact 与本地副本 cmp 一致。
85: ## 四、证据清单
```
