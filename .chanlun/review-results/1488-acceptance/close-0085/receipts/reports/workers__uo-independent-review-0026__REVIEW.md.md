# 收据 workers/uo-independent-review-0026/REVIEW.md

- 原件：`workers/uo-independent-review-0026/REVIEW.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`c2c04f11bdf04575b9ac1b69879e6ffe0a0eef00f2db1a9deb3e208b87de530d`
- 字节数：13953；行数：102

## 关键行（行号: 原文）

```
1: # #1488 统一所有权预算补丁独立复审（0026）
7: ## 结论：**FAIL**
9: 0024 的 P2-1..4、P3-1..4 基本已解。但 P1-1 只是**部分**解决，而且新的 OldPoolPartition 引出一个新 P1（keeper 池不在当前候选父账上时，物理超额不进入 T）。另有一个需主控裁定的 P2：C5 把节点正文在父账内双计写成了期望。
13: ## 0024 findings 逐条状态
17: | P1-1 E/O 交集 max 补差、无逐项映射 | **PARTIAL** | 见下文 §P1-1 |
18: | P2-1 生产 adopt_native / Budget::new(retained) | **RESOLVED**（残余 P3-a） | `audit_owner/native.rs:1513` 覆写为 `#[cfg(test)]`，生产 Ledger 落到 trait 默认 `s_content/capture_allocation/scratch.rs:132`，恒拒；`Budget::new` 为 cfg(test)，生产只有 `fresh`；`adopt`(native.rs:793)、`adopt_codec_vec`(:845)、`MoveIn`/`move_in`(:11
19: | P2-2 material journal 事后同额补签 | **RESOLVED**（残余 P3-b） | 生产改为 `Recording::start_on(…, &candidate.resources, candidate.limit)`（audit_owner.rs:2658–2667），在 journal 分配前从候选父账取票（effects.rs:431/441），`attach_parent` 改为 cfg(test)；`with_material_access` 不再调用 `new_empty_with_fee(j.bytes)` |
20: | P2-3 rebirth 峰值漏计旧块 | **RESOLVED** | `rebirth_vec` 在新块前做 `coexist = reserve(vec_bytes(cap))`，旧块释放后才 drop（native.rs:879–886）；`rebirth_codec_vec` 同构（:936–955）。保守方向 |
21: | P2-4 池内重复计费不可检 / 负控只测 helper | **RESOLVED**（引出新 P2-A、P3-c） | 逐许可普查 C1–C5 接在 r14 真实封存路径上（audit_owner.rs:3194、3436–3540）。`p2_4_seal_census_flags_duplicate_permit_on_real_seal_path` 通过 `SEAL_FAULT` 在真实 `build_bounded_counts_admitted` 中注入重复许可：差额恒等式照样通过，普查拒绝（C5）。CI 显示 PASS |
22: | P3-1 冷根失败首跑残留 | **RESOLVED**（残余 P3-d） | audit_owner.rs:1540–1557 记 `base`/`residue`，残余非零时调用 `cache::clear_handles()`；锁 `p3_1_cold_root_audit_failure_leaves_no_permit_on_destroyed_root` PASS |
23: | P3-2 :775 裁定 | **RESOLVED** | 主控已在 #1488 评论中追认 |
25: | P3-4 BootstrapGrant 域 | **RESOLVED** | `BootstrapGrant::from_freeze(FreezeSeal{domain,limit})`；原 `debug_assert_eq!` 改为生产路径的 domain/limit 拒绝（request_audit.rs 约 1948），`take_bootstrap_grant` 也做同样过滤；锁 `bootstrap_grant_from_other_freeze_domain_is_rejected` PASS |
29: ## P1
31: ### P1-1（0024 遗留，PARTIAL）：U 是一笔残差，不是逐项映射；E 仍不对 L0 映射
48: ### P1-2（新，PLAUSIBLE，静态推导加实测读数）：keeper 池所在 Ledger 不是当前候选父账时，只按合同额计，物理超额不进入 T
54: - 实测读数（同一 p2_4 fixture，日志 `/tmp/nc-remote/20261020-004846-2d613c16ff.log`）：封存后池在父账上的存活许可为 527048B；合同额 `retained_source_cost` 为 194817B。扣除 P2-A 所述的 803 张、111334B 双计来源许可后，仍有约 22 万 B 以上的真实物理分配加上 M，不在 O、U、E 任一项中。
64: ## P2
66: ### P2-A（新，需主控裁定）：C5 把节点正文在父账内的双计硬编码为期望
70: C5 的期望是：析构时，每个节点都有一张“没有真实释放支撑”的来源许可，金额为正文 + digest/tag/证书。也就是说，同一份节点正文在父账里既有 body Native 许可 x，又有 source_permit x+97。在 r14 中共 803 张、111334B，背后没有对应的真实分配。v4 要求“parent 物理 live 只计一次 / 不把 parent 物理 live 在 L0 重加”。0025 REPORT 的待决项 2 也自承这一点需要主控裁定。普查把这个现状写成通过条件之后，未来若去掉双计，锁会变红；若保留双计，就等于默认接受了 L0 中的重复。必须由主控二选一，不
74: ## P3
78: - **P3-c**：C5 的“无释放支撑”通过 allocator `released_bytes` 的交错来判定，属于启发式；多线程或并发释放时可能误判（测试线程为 4，当前 PASS）。
83: ## 评审项 1–7（按 0024 control 重做）
85: 1. **反例 A–F**：A/B/D/E/F 仍然闭合；adopt、adopt_native、adopt_codec_vec、MoveIn、事后预留、未分配余额都已退出生产（全仓 rg，cfg 标记与非测试 cargo check 双重证据）。**C 仍为 PARTIAL**（P1-1），并带出 P1-2。结论：**FAIL**。
86: 2. **上轮 P1/P2**：P2-1..4 已 RESOLVED，P1-1 为 PARTIAL。结论：**PARTIAL**。
87: 3. **原断言保护**：diff 中只有两处删除 assert，都是加强：96B 字面常量改为 Layout 推导；`debug_assert` 改为生产路径的 domain/limit 拒绝。六 driver、128B probe、1024、16/8MiB、R13 的断言都没有改动；r13 和 construction_tests 只把冻结入口改为 partition（setup 层），r14 原断言仅重排格式，另追加普查。结论：**PASS**。
88: 4. **r14 恒等式**：同父、无追认、无残留成立；逐许可普查已接到真实路径，负控有效。但普查期望中包含正文双计（P2-A）。结论：**PASS_WITH_FINDINGS**。
89: 5. **业务语义**：`audit_owner::startup`（audit_owner.rs:351–392）与 `state.rs` 零 diff；Cargo/features/workflows 零 diff。codec、集合、首错与 COMMIT 顺序、Recover、资源 cap 的逻辑都没有改动。新增的生产行为有 3 处：冷根残余时 `clear_handles`、journal 错误类别（P3-b）、driver discard 后重新分区（与 P1-2 相关）。结论：**PASS_WITH_FINDINGS**。
90: 6. **冷根失败后无票重跑**：重跑仍是 `audit_required_in_parent(mode, context, None)?`，原样返回 SQL 错误，顺序不变，1024 成功回复不变；新增的 `clear_handles` 只影响缓存复用（P3-d）。结论：**PASS**。
94:    - CI 39290457765：headSha=`b6969af4cc…`。artifact-1（`s_session,s_capture_native_testonly`）的 locks.tsv 有 46 项，与 `git show b6969af4cc:.github/1488-uo-locks.txt` 去掉注释后的 46 项完全一致，42 PASS / 4 FAIL(101)，与远端日志一致。artifact-0（无 native feature）为 45 NOT_LISTED / 1 PASS。本地 `workers/uo-ci-runs/39290457765` 副本与重新
96:    - 结论：**PASS**。
98: ## 建议返修
100: 1. P1-2：分区时 keeper 必须按其所在 Ledger 的物理 live 只计一次。父账不同或候选已丢弃时，要么把该 Ledger 的 live 作为独立的 L0 分区，要么拒绝保留池。补一个“discard 后 keeper 存活”的锁。
101: 2. P1-1：把 U 拆到 v4:38 清单中的各字段；不可映射的残差应拒绝，而不是吸收。E 对 L0 做逐项映射，或由主控书面豁免反例 C 的 E 侧。
102: 3. P2-A：由主控裁定正文双计；若不允许，修改 C5 期望并去掉双计。
```
