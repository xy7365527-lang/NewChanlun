# 收据 workers/arch-0063/ARCH.md

- 原件：`workers/arch-0063/ARCH.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`93bb3644cb667f4d9b6534f8a04923b976e2fc499123c8c23067975666a26438`
- 字节数：23353；行数：207

## 关键行（行号: 原文）

```
1: # #1612 增量升格：架构改动面（arch-0063，只读分析）
9: ## 0. 前提事实（后文反复引用）
20: | F8 | S4 锁 `v3::driver::tests::s4_generation_incremental_matches_cold_full_six_stages` 在 a89996a487 上 PASS：28 代，8 项逐位比对，N1–N6 负控全部变红。但 g28 增量 3750ms，完整路径 2658ms，**目前增量比完整更慢**；§3.1 N3h 显示 removed = 上代全部、added = 本代全部 | S4 §2–§4 |
22: | F10 | 0059：testonly 下 fail-closed 卡在 Begin（`audit_owner.rs:428/531`）；S86@256MiB 只跑到 19/86，MIN12 15/52。根因：journal 帽 `limit/8`（`audit_owner.rs:3133`）、扩容估算 `s_content.rs:3268-3310`（`:3301` `growth > cap`）、`native.rs:1476`、stale `:649-730` | 0059 REPORT §2.3–§4，P1–P5 见 :134-147 |
26: ## ① 改动面是否只在 S 写者生成管线内部
28: **结论：主体在 S 写者内部。** 不碰 schema，不碰 rule_revision，不碰 Q/c2 协议，不碰财务。逐位锁保证公开根和回执不变。触到边界的有 **4 处**，都不是公共查询合同，但都需要 ADR 级裁定；另有 **1 处（P6）是 spec 级**。
30: ### ①.1 按边界面逐项判断
35: | 内容寻址身份：对象/节点摘要 | **不触（以锁为条件）** | F2：同一个 `generate_impl` 产出同一批对象；S4 的 8 项逐位比对覆盖对象摘要（F8）。**注意**：增量和冷算持久化下来的*不可达*节点集合不同（S4 §2），所以「节点全集」只能定为 CONDITIONAL，详见 ④ |
36: | 公开根、回执 | **不触（以锁为条件）** | S4 锁逐位比较公开根和回执；compare_s86 把它们列为 INVARIANT（accept-s1 REPORT:48-58） |
40: | 恢复（recover/epoch） | **触** | 生产已有 recover（`audit_owner.rs:955-1011`），testonly 只在 `:975` 多带 handoff。但 recover 事件必须落在绑定的 clock plan 内（`s_session/README.md:109`）。若生产保持 0059 的 fail-closed 语义，就要求 clock plan 预置 recover 事件，这会触到协议输入；改为降级（0059 P1：超预算 `Ok(None)`，放弃保留，审计和回复照常成功，即 v4 L13）可以避开。另有 #1612 审查 R5：冷启
45: ### ①.2 PLAN 方案点逐项判断
57: | P1「身份摘要排除每代字段」 | 内部，**但疑似误诊** | 见 ③.2：layout/1 下的全量置换强推断来自中段插入，不是每代字段 |
65: ## ② 切片清单
69: ### (i) 升格开关与资格、资源、恢复口径
75: | i-c 降级修复（0059 P1–P5 中属于开关的部分） | P1：超预算 → `Ok(None)` 放弃保留，不再 StaleWriter（v4 L13）；P2：Begin 不卡死（`audit_owner.rs:428/531`）；P3：journal 帽 `limit/8` 改为按实际需求（`audit_owner.rs:3133`）；P4：扩容估算（`s_content.rs:3268-3310`）；P5：`native.rs:1476`、stale `:649-730` 收口 | `audit_owner.rs`、`s_content.rs`、`audit_owner/nati
76: | i-d 恢复与冷启动同一性（R5） | 采用 R5 的方案 (b)：完整生成 = `generate_impl(delta=None)`，是唯一的冷启动实现（F2），来源审计 `previous=None` 时走同一代码。object_plan 和 stage_indexes 的增量/完整两份实现要么合并，要么把「冷启动 + k 步增量 ≡ 冷算」写进锁。recover 采用降级口径（i-c P1），从而不要求 clock plan 预置 recover 事件 | `generation.rs:411,658`、`prepared.rs:653-680,1065-1068`、`audit_
81: ### (ii) 六阶段 Δ_R 化（按 PLAN §2）
88: | prepare_source_audit ②③ | 持久台账 | 新表 | 2–3k | 2–3 | **P6 spec** | 等裁定 |
90: | assemble_manifest ④ | 清单论证 | 小改动 + spec | <200 | 1（spec） | **P6** | 等裁定 |
96: ### (iii) 候选保留有界（PLAN §6、0059 P1–P5）
102: | iii-3 | 超额降级 `Ok(None)` | 与 i-c P1 是同一处代码 | 并入 i-c | 0 | — |
107: ### (iv) 每请求完整 state::audit（#1481 方案三）
109: - **不属于 #1612 本次范围**，归 #1481（票面 :16-17 U-A/U-B，:25-27 Δ 步与三层对拍，:32 读侧影响清单，并明确「不在本票改 Q/c2 协议」）。
116: ## ③ 与 #1491 有界布局（ADR 0037、CC-007）的交织
118: ### ③.1 段 B 改什么（ADR 0037）
121: ### ③.2 关键推断：S4 的全量置换来自 layout/1
125: - **推断**（未跑验证）：S4 的 O(N) 增量开销主要来自 layout/1 的错位，而不是 PLAN P1 假设的「身份摘要里含每代字段」。出生序可以从根上消除它。
126: - **便宜探针**（≈0.5 WU，只读）：在 S4 的 28 代上统计「错位一位后摘要相等」的比例。比例高，说明 P1 在 layout/1 上是白做；比例低，说明 P1 的诊断成立。
128: ### ③.3 分类
135: | PLAN P1 身份摘要 | 先跑探针 | 若探针确认错位 → 等段 B | **高风险白做** |
145: ### ③.4 建议顺序
146: 1. **S0 探针**（③.2）＋ **i-c 降级修复**（在 testonly 下做，0059 P1–P5）。两者并行。
149: 4. 在出生序上做 **generate → object_plan → (assemble ①–③ ∥ stage_indexes)**；P1 是否做由 S0 结论决定。
155: ## ④ 切换期对拍保障
157: ### ④.1 增量 ≡ 全量逐位锁扩展到生产构建
163: ### ④.2 S86 / MIN12 双路对拍
164: - **S86**：`compare_s86.py` 三档（accept-s1 REPORT:48-58）。增量对冷算时，INVARIANT（公开根、Basis 根、回执、非节点表、可达闭包）必须逐位相同；`s_content_nodes` 全集归 **CONDITIONAL**，因为增量和冷算留下的不可达节点不同（S4 §2）。所以「全库指纹」要按**可达闭包**定义，不按表全集定义。
167: ### ④.3 灰度与一键回退
172: ### ④.4 完整路径作为离线对照臂（#799 合规表述）
173: > **开关开启时，生产准入只由增量路径的判据决定；完整路径只作离线对照臂，对拍不一致时报告并阻断灰度推进，但永远不在请求内替增量兜底，也不在增量失败时被调用来出具回复。** 增量出错时执行 `discard_runtime_state` 并 stale，下一请求从同一个 `generate_impl(delta=None)` 冷启动（F2、F3），这是增量路径自身的冷启动，不是第二条判据。
179: ## 总评（一页）
189: | **合计（#1612 范围）** | **≈8.4–13.4k** | **≈16.7k** | **≈20–26** | 1–2 份 ADR + 1 份 spec |
190: | (iv) #1481（不在范围） | U-A 0.3–0.5k + U-B spec 级 | — | 2–4 | spec |
195: 1. **白做风险**：在 layout/1 上做 generate、assemble、stage_indexes 和 P1 的 Δ 化，会被段 B 的出生序改写（③.2，推断，S0 探针可在 0.5 WU 内证实或证伪）。
204: > - 先在 testonly 下修 0059 P1 和 P3（`audit_owner.rs:3133` 的 `limit/8`、`s_content.rs:3301`、超额 `Ok(None)` 降级），目标是 MIN12@256MiB 跑满 52/52（现在是 15/52）、S86 跑满 86/86（现在是 19/86）。
```
