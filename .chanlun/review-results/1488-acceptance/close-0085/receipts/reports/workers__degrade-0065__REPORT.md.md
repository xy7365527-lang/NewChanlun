# 收据 workers/degrade-0065/REPORT.md

- 原件：`workers/degrade-0065/REPORT.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`3d64e320822d696e406b76658d14502932ec2568464a326f37edbc9a89aacea6`
- 字节数：19417；行数：171

## 关键行（行号: 原文）

```
1: # #1488 degrade-0065 报告：v4 L13 降级修复 + testonly 双路对拍
3: - 工位 `budget-handoff-fix-v2-checkout`，分支 `codex/1488-budget-full-chain-fix-v2-20261018`；起点 `727b3bfb45`，终点 `78523300ad`。只做本分支 append-only 提交，未 push，未合入 main。
7: ## 0. 结论
11: | P1：候选 journal/索引预算不足 → 放弃候选、无候选发布并成功回复 | 已实现（只在 `s_capture_native_testonly` 门内）。锁、负控、红证齐全；真正的尾检失败仍然 fail-closed |
13: | 全锁表 | `1a01e55943` 上 **87/87 PASS**（原 86 条加新锁 1 条） |
14: | S86（testonly native，帽 268,435,456） | **86/86 Committed**。与 S1 r8a 做 `compare_s86.py --mode cross-binary` 对拍：**INVARIANT 31/31 相等，VERDICT PASS**。CONDITIONAL 2/5 相等，3 项差异逐项解释见 §5 |
15: | MIN12（testonly native，帽 268,435,456） | **52/52 Committed**；与 accept-s2 oracle 的**逐位**对拍无法成立（oracle 由 v2 生成，见 §7）。改为两条口径：① native 与同二进制 ordinary 臂、以及 **oracle 同一二进制 main `6badaa6049`（sha `c944bd6b…`）的 v3 臂**做三方比较，52 代 qcut3 摘要、counts、rows、回执**逐位全等**（PASS×2）；② 对 oracle 逐代比 counts，52/52 有差异，三臂差异完全相同，
17: | P2 / P3 / P5 | 未实现。目标运行不依赖它们，理由见 §4 |
20: ## 1. P1 设计（v4 L13）
45: ## 2. 生产不变证明
51: | 生产 release sha256 | `01b31fda…` | `4da7731d…` |
56: - 未 strip 时的差异只出现在 `.strtab` 本地符号的 `.llvm.<hash>` 后缀（crate 哈希）。代码节和数据节逐字节相同。中间点 `ff5612fea2` 也做过同样核对，结论相同。
58: - testonly release 二进制：`testonly-1660d9d32d` sha256 `06abcef0d4739ccb42807ec601ba5e32afe287ac4d3cf89be9c099d919a29c94`。
60: ## 3. 既有缺陷：pager 诊断探针永久撤销 Carry 租约（`1660d9d32d`）
64: - 修复：`configure_pager_probe` 中只替换一行，改调用 gated 尾部的 `clear_candidates_for_pager_probe()`（行号中性：与 727b 对比，731 行之后唯一的差异行是 1027）。新函数先清候选，再执行 `reopen_idle_lease()`，条件与 P1 续租相同（无候选、无资格、未退役、autocommit、close 成功）。诊断关闭时不经过这条路径。
69: ## 4. P2 / P3 / P4 / P5 的必要性
75: - **P1** 把 ① 改成降级成功，这是目标运行 86/86、52/52 的必要条件，已经实现。
77: - **P2（精确索引扩容）/ P3（j.cap 与 L0/A 对齐）**：二者只能推迟 ① 的触发代数、提高保留率，不改变“成功与否”。目标运行在没有 P2/P3 的情况下已经全部成功，与基线对拍通过，因此不属于“为达成目标所必需”。未实现，以免在 #1612 生产开关裁定前扩大 testonly 面。
79: - 代价是保留率：放弃之后，由于冷根首跑超帽，候选无法再出生，保留率停在首次放弃前（S86 19/86、MIN12 15/52）。要提高这个数字，需要 P2/P3 加 P4 的“保留上一个已知良好候选”或有界部分候选。这留给后续，与 #1612 一起裁定。
81: ## 5. S86 对拍（`s86-nat2`，testonly-1660d9d32d，native，帽 268,435,456）
84: - `compare_s86.py s86-nat2 s1/r8a --mode cross-binary` 结果：**VERDICT PASS**，INVARIANT 31/31，CONDITIONAL 2/5。报告存于 `~/nc-remote/1488-accept/d65/s86-nat2-vs-r8a.json`，G: 有副本。
85: - INVARIANT 全部相等，包括：init.json；86 条回复帧逐位；逐代公开根 manifest/snapshot/delta_root；逐代 FinancialBasis 根（独立重算）；basis-ref；全部业务表；公开根集合；可达闭包 344,419 个节点。末代 snapshot 根 `7508b621…`，basis 根 `sc1-c195df59…`，与基线相同。
86: - CONDITIONAL 有 3 项差异，均为跨二进制允许的差异：
88:   - `binary_sha256`：两侧二进制不同。
92: ## 6. 指标
106: | 运行 | 全部：合计 s / 均值 / p50 / 最大 | 保留段 | 放弃后 | 宿主负载 均值（范围） |
118: ## 7. MIN12 对拍
128: **逐位口径（`compare_arms.py`）**：nat2↔ord2、nat2↔main1 都是 **VERDICT PASS**。逐条回执 publication，以及逐代 qcut3 摘要、counts、structure_cut、rule_revision、canonical 行 sha，在 52 条/52 代上全部相等。三臂末代摘要都是 `qcut3-93911ed1d9574e…`。结论：native 降级路径（含 op-0015 放弃）的公开结果与普通路径相同，也与 oracle 同一二进制的 v3 输出相同。结果见 `.chanlun/review-results/1488
132: - v3 投影行是内容引用（`{root, extent, format, kind}`），catalog 也是引用。摘要是 `qcut3-` 内容树根，与 oracle 的“canonical 文件 sha256”不是同一函数，所以 digest 不可比。
137: ## 8. 跑法与驱动层改动（只动 `.chanlun/` 驱动，不动产品）
143: ## 9. 待裁与张力
145: 1. **#1612**：本次 P1 和 pager 修复都只在 testonly 门内，没有加生产开关。生产路径仍然是“候选预算不足 → StaleWriter”，等 #1612 裁定后才能放开。
146: 2. **保留率**：放弃之后不再保留候选（§4）。如果验收要求跨请求复用覆盖全程，需要另开 P2/P3/P4 扩展（“保留上一个已知良好候选”或有界部分候选）。
147: 3. **MIN12 oracle 可比性**：accept-s2 oracle 由 main v2（s-session/2，`/api/v2/snapshot`，rule `…+segment-second/2`）生成；native testonly 只存在于 v3。v3 的 Q3 摘要是 `qcut3-` 内容树根，投影行是内容引用，rule_revision 也不同，所以无法与 oracle 逐位对拍。本报告用语义口径（逐代 counts）对 oracle，再用两臂与 main 对照臂补足逐位口径（§7）。是否另出 v3 oracle，请主控裁定。
148: 4. 既有红：`content_publication::tests::native_publication_begin_and_consumers_share_parent_scope` 在 `727b3bfb45` 上同样红，不在本票范围。
151: ## 10. 提交
153: - `4017955224` fix(#1488): 0065 P1 testonly publication 候选预算不足降级为无候选发布（v4 L13），附锁与负控
```
