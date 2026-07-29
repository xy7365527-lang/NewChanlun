# #641 N3 实装收尾评审 — Spec 轴（2026-07-29）

工作面 `/tmp/wt-641n` @ `ticket-641-n3`，diff = `039cf86974...HEAD`（3 commit，1977+/4-）。
规格源 = #641 票体 Acceptance 1–5 + #636 Resolution 与两条 #641 裁定（地板条款
comment-5121572134、两套取舍 comment-5121683...「A 修复轮」三项）。

## 结论：**FAIL**（4 项规格缺口，其中 2 项为已明令的修复轮条款未落地）

## (a) 规格要求但缺失 / 部分

**A1｜地板条款未实装（阻断级）** — `chain_cert/mod.rs:479-485`
裁定原文：「链必须**至少一条有效链段**才能 Closed；链头独活（全下级证伪/缺失、链段集合为空）=
**永远 Open**……『全链段谓词判过』不得在空集上真空成立」，且「`head_only_survivor_closes_by_
vacuous_segment_condition` 测试所钉行为**翻转为不可 Closed**」。
实装 `all_segments = edges.iter().all(is_segment)` 对空 `edges` 真空为真，`Closed` 分支无
`!edges.is_empty()` 合取。`tests.rs:470-476` 的文档注释反向声明「按裁定字面落地，不自行补
floor 规则」——该测试未翻转，仍在钉旧行为。裁定明列适用面为 #641 两套实装，非留 fog 项
（留 fog 的只有 `CloseFloor_at` 三型完整口径）。

**A2｜`extends` 未查簿（修复轮③）** — `chain_cert/mod.rs:491`
裁定：「`extends` 改查簿命中才写（实测 12/12 指向不存在链）」。实装仍无条件写
`key.proper_prefix()`，文档自认「不保证该 key 曾作为对象在簿中物化过」。

**A3｜CONTEXT.md 词表未收编** — Acceptance 4「命名独立（与 NestCertificate 区分），CONTEXT.md
词表收编」。命名独立**已达成**（`TowerChainCertificate`，模块头对照 ADR-0005）；但 diff 未触
CONTEXT.md，全文 grep 无 `chain_cert` / `TowerChainCertificate` / 链证书条目（对照 N1/N2 已收
「背驰段候选事件」`CONTEXT.md:123`、「Sub 边」`CONTEXT.md:119`）。**部分达成**。

## (b) scope creep

未发现。`TowerCache::candidate_streams()`（`classifier/mod.rs:1229`）只读取数、
`P123_CHAIN_DUMP` env-gated 只写不判（`p123_fast_replay.rs:20-135`）、电池
`ISSUE641_CHAIN*` 三行摘要，均在 Acceptance 3 授权面内；nest 对照侧零触碰。
Hasse 覆盖边 + 全部极大路径枚举不丢分支，与工位碰撞裁定「与『谱系照实』一致」的认定相符，
非 creep。

## (c) 看似实装但错了

**C1｜「双路径逐字节一致」测试是自比** — `classifier/mod.rs:4442-4451`
Acceptance 1 要求「全量/增量双路径产出逐字节一致」。测试里 `replay` 与 `prefix_incremental`
调用**同一函数、同一参数**（`chain_book_over_prefixes(&segments[..n], …)`，仅 cache 实例不同），
比的是同一条增量路径重跑两次；真正的增量簿 `incremental`（4442 行）算出后只参与非空断言，
从未进入 `assert_eq!`。全量路径 `classify_with_tower_events`（fresh，N1 双路径对照见 4231-4234）
在链侧完全未接。锁的是确定性，不是双路径一致性。

**C2｜链头 `Absent` 被判 Invalidated，与「无缺席即失效」自相矛盾** —
`chain_cert/mod.rs:476,479`
`advance` 文档明写「链……**没有『缺席即失效』这条路径**——链的死活只由裁定②的两个条件（谓词
判不过 / 链头 Invalidated）决定」。但 `head_alive` 把 `Absent`（查无，模块自述在终态窗口投影
驱动下真实可达，`mod.rs:108-115`）与 `Falsified` 同归一路判 `Invalidated`，且终态不复活。
裁定②只授权「链头 `Invalidated`」判死，查无≠证伪（模块自己在 `ChainNodeStatus` 上把两者刻意
分列并单列计数）。因果簿驱动下不可达，终态窗口投影驱动下会永久错杀。

## 已核对通过（不构成缺口）

节点身份键沿用 `CandidateKey` 且模块内零构造（禁伪造中间级证书）；skip 与 adjacent 同权
（`is_segment` 只看谓词，`mod.rs:164`）；判不过留为事实边；证伪中间节点被跨过、由两侧存活端点
重判（`crossed_nodes` + `crossed_by_edge` 探针）；缺/断分档（`SkippedLevel`）；append-only +
终态不复活 + 同 as_of 零 Delta（`apply`/`make_revision` + 对应单测）；相切边界
（`touching_endpoints_still_form_a_segment`）；44 课小转大留 fog 照实声明；纯产出零消费。

## 建议处置

A1/A2 为已裁明令，须在关票前落；C1 须把全量路径真正接上再宣称 Acceptance 1；A3 一行词条即可；
C2 建议将 `Absent` 头改判 `Open`（或经裁定确认后成文）。
