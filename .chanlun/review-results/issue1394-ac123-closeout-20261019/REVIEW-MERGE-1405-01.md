# REVIEW-MERGE-1405-01：#1394 合入 #1405 后的衔接审查

- 被审：`codex/1394-ac123-closeout-20261019` @ `93f386baeb`（worktree `/Volumes/AgentStorage/issue1394-b46-closeout-wt`，只读，工作区干净）
- 基线：origin/main `5baeb030a6`；合并基 `a54d1ced81`；#1405 链 `8f6fdda1ad..d63d31483b`
- 审查者：独立审查代理（非作者）。只做静态核对，加上 node/python 轻量测试；本机禁 cargo，Rust 编译与测试结果以作者提供的为准。

## 结论

没有 HIGH 或 MEDIUM 问题。冲突解法没有丢失 main 上 #1405 的行为；三项衔接（q3-typed 承载、RULE_REVISION 闭合、bsp 白名单重锚）全部成立；新负控确实走到「不得让其他 slot 插队」分支，对照组也证明 alpha 单独可以装下。另有四条 LOW 级别的记录项，不阻断。

## A. 冲突解法与 #1405 语义

**前提核实。** 对 #1405 涉及的 31 个非 review-results 文件逐个执行 `git log a54d1ced81..origin/main -- <file>`，提交全部来自那 9 个 #1405 提交，没有别的写者。所以作者「冲突处取我方」的依据成立。另外，`git range-diff 0d4dc64473..5d99234d16 8f6fdda1ad^..d63d31483b` 显示：preintegrate 当初合入的 #1405 旧链（`5d99234d16`）和 main 上那 9 个提交逐个相等（diff 都是 4886 行）。也就是说，适配所依据的 #1405 内容与 main 现行版本一致。

**结果核实。** 31 个文件在 HEAD 与 `dccb69793c`（preintegrate）之间的差异都是 0 行，说明合并后的版本就是 preintegrate 版。其中 stroke.rs、parser/mod.rs、env_registry.rs、stroke_tb02v_tests.rs、tb02v 夹具、tb02a/tb02b oracle、tb02v_runtime.py、tb02b_harness.py、tb02b_verify.py 等与 origin/main 逐字相同。

**逐文件对比补丁。** 把「#1405 在 main 上的补丁 a54d→main」与「#1394 侧实际施加的补丁 b46ba29699→HEAD」去掉行号后对比，有 20 个文件的补丁逐行相同。另外 11 个有出入的文件，我逐个核了意图：

| 文件 | 差异性质 | 判定 |
|---|---|---|
| tb02a_facts.rs | main 在全量路径里删掉「同角色极值竞争三组窗口请求」，组载荷标签改为 `labels`，步骤原因过滤掉 `equal_extreme_identity`，domain 升级。HEAD 把这些拆成 `step_extra_reasons`、`group_reasons`、`group_payload` 三个共用助手，全量路径（:480-595）与增量路径（:700-888）都过滤 `EQUAL_EXTREME_IDENTITY`；增量路径的窗口请求块也一并删除（:809）；`KNOWLEDGE_DOMAIN` 改取 `tb02b::CURRENT_CONTRACT.knowledge_domain`，值相同。`grep windows(3)` 全仓零命中。 | 重构，行为等价 |
| tb02b.rs | main 用 `SCHEMA_REVISION` 常量和 `policy_version()` 拼接；HEAD 改成具名 `BiContract`（CURRENT 或 LEGACY），载荷取值相同（`s-new-bi/2`、`RULE`、`standard_raw_gap_3;same-price/keep-earlier+extreme-region/1`）。policy 字面量由测试 `named_bi_revisions_never_fallback_or_relabel_history` 锁定为与 `format!("standard_raw_gap_3;{SAME_PRICE_POLICY}")` 相等。 | 重构，加锁 |
| s_structure_session.rs | `RULE_REVISION` 与 main 逐字相同；#1373 选根门的删除在 HEAD 中成立（`advance_core` 中已无该门，注释保留）；另新增 V3 和 LEGACY_V3 常量（见 B(b)）。 | 等价 |
| bsp.rs | 见 B(c)。 | 正确重锚 |
| index.html | `logicalSlot` 的 `s-new-bi/1`、`s-new-bi/2` 两代在 HEAD 的对应位置（:1350）上等价。 | 等价 |
| tb01c-client.js | 见 B(a)。 | 迁移到 q3-typed |
| s_tb02_contract.py / s_query_integrity.py | 多出 `new-bi/2`（v3 A/B，13 轴）一行；未知规则从 main 的「`.get` 兜底，退到全轴或 None」改为 `raise ValueError("cut规则修订未声明")`；空会话也先验规则。 | 只收紧，不放宽，没有丢掉 main 行为 |
| test_tb02b_contract.cjs / .py、test_tb02c_verifier.py | main 新增的用例全部保留：cjs 改为 async 或 `Q3.validateState` 执行点，拒绝原因原文相同。只有 main 的 `_published_axes('unrecognized-rule') == AXES` 一条按上面的收紧改为期望 raise，其余都是新增用例。 | 保留并扩充 |

**自动合并的同族文件。** v3 producer（`content_producer.rs`）同样退役了 `shape_window_has_competing_extremes` 门（:1936）、步骤原因过滤（:894）、组载荷 `labels`（:2333）与 domain；`s_session_v3/generation.rs:644` 同样过滤；`source_pending.rs` 的 `equal_is_evidence` 按具名合同精确区分 /1 与 /2；`q3-consumer.js` 的 rule→axes 表与 `RULE_CONTRACTS` 绑定（:1373）和 main 的 `validateTB02Candidate` 绑定段逐条对应（段、笔、CC-054 domain 三处判断相同）。main 上不含 `s_session_v3/` 和 `q3-*`，这些文件不存在合并语义丢失的问题。

**非 #1405 的 main 改动。** 1d4d3dccbc 带入的其余文件（recursive_t、replay_compare、trading 等）在 HEAD 与 origin/main 之间零差异；有差异的只有 ci.yml、Cargo.toml、Cargo.lock、lib.rs，全是 #1394 自己的新增，main 的内容（含 60 分钟超时等）都没有被删。

## B. 三项衔接

**(a) q3-typed.js 承载 s-new-bi/2 校验：成立。** 把 main `tb01c-client.js:408-500` 运行段与 `q3-typed.js:8-22, 423-510` 逐条对照：`KNOWLEDGE_DOMAINS`、`BI_VERSIONS`、`SAME_PRICE_KEEP_EVIDENCE`、`RETIRED_SAME_PRICE_WAITING` 字面量相同；`RULE_CONTRACTS` 在 main 五条的基础上多了 `new-bi/2`，对应 v3。`validateBiPayload` 中以下各项都对上了：版本三元组、v2 退役等待态、端点 v1/v2 字段分叉、根集与首末 raw 闭合（`last-first >= count-1`）、条件中 v2 的 `raw_between` 必填与向量不得为 null、异型 evidence=[]、同型 v2 只允许 KEEP/REPLACE、比较器闭合与保留锚、EQ 证据、change_event 的 before 按原件代际校验而 after 跟随本代、slot 绑定。CC-054 domain 两代集合（:167）以及会话规则到合同代际的绑定（q3-consumer.js:1373）也都在。`node s_session/tests/build_tb01c_browser.cjs --check` 返回 `status: passed`，生成段与五个输入一致。

**(b) RULE_REVISION 闭合：成立。**
- `RULE_REVISION = "s2-axis-quantifiers+new-bi/2+segment-second/1"`，与 main 逐字相同，没有另造新串，也没有带 segment-second/2。
- `V3_RULE_REVISION = "s2-axis-quantifiers+new-bi/2"`，由 v3 写者使用（`v3/mod.rs:452`）；`require_current_writer_rule` 拒绝在旧规则上续写（:121-126）。
- `LEGACY_V3_RULE_REVISION = "...new-bi/1"` 只出现在读取和审计映射（`tb02b::contract_for_rule`、`CatalogAxisTable::for_rule_revision`）以及测试中。
- 映射表闭合情况：Rust `contract_for_rule`、Py `RULE_CONTRACTS` 和 `_published_axes`、JS `RULE_CONTRACTS` 与 rule→axes 四张表对 `{new-bi/1, new-bi/2, new-bi/1+segment-first/1, new-bi/1+segment-second/1, new-bi/2+segment-second/1}` 的取值一致：笔合同 /1 对 /1、/2 对 /2；13 轴对应 A/B，17 轴对应 segment-second。Rust 目录表不含 segment-first/1，但目录表只在写者侧按当前修订调用，不影响读取路径。
- v3 改为 new-bi/2 有依据：`tb02b::payload` 恒按 `CURRENT_CONTRACT` 发出 `s-new-bi/2`，而三种语言的合同表都把 `new-bi/1` 绑到 `s-new-bi/1`。如果 v3 留在 /1，自己写出的笔对象会被自己的读者以「新笔合同与cut规则版本不符」拒绝。
- 不构成 #799 宽严两档：各表都按规则修订**精确匹配**，未知规则一律报「未声明」（Py 和 Rust 报错，JS 得到 null schema 后对笔和段对象拒绝）；/1 与 /2 不接在同一个 if-else 上互相兜底，旧规则只读不写。13 轴和 17 轴的分表已在 `tb02a_facts.rs:911-916` 登记，并写明了退场条件。

**(c) bsp 白名单：成立。** 在 HEAD 的 `rust/src` 上用测试里同一条正则 `\.(find|any|filter)\(\s*\|[a-zA-Z_]+\|[^)]*\.source_index\s*==`（排除 bsp.rs）做 grep，得到 23 个命中点，与 `WHITELIST` 的 23 条 `diff` 后完全相等，既没有遗漏的命中点，也没有过期条目。#1394 的唯一相关点是 `tb02a_facts.rs:556`（:550-558，按组锚 find `InclusionGroupFact`，不是 BSP 绑定）。b46 当初已把 main 的 231/241/267 三处收成一处，原来登记在 564，删掉 #1405 的窗口请求块后上移 8 行到 556，注释写明了理由。

## C. 新负控 `waiting_slot_blocks_other_slot_reserve_without_queue_jump`

**分支命中。** `finance.rs:910-925` 的 Reserve 分支依次检查：domain 是否为 E、slot 是否存在、`request_set_hash` 是否匹配，然后才是「已有 AwaitingAllocationPolicy 的 slot 就报插队」。测试为 slot-b 用自己的 applicants 计算 hash（夹具 `request` 按 slot_id 查找，:258-259），并补了 `Reserve-slot-b` 时钟（:201-205）。断言是 `err.contains("不得让其他 slot 插队")`：如果先在时钟、证据或 hash 上失败，报错原文不同，测试会变红，不可能误判为通过。这里只能确认「分支走到了」；「测试为绿」依赖作者提供的 cargo 结果。

**对照组。** 夹具中 alpha 需 2×14+2=30，beta 需 3×12+4=40（:90-92），既有责任 20，cap 55。slot（beta）合计 60，超 cap，进入等待；slot-b（alpha）合计 50。对照 `g` 同一 cap、没有等待 slot 时，slot-b 的结果是 Reserved、恰好一条 alpha grant、责任总额 50，证明负控中的拒绝来自等待门，与 alpha 是否装得下无关。负控还断言 E 镜像整体不变、slots 只有 `slot`、alpha 停在 Prepared，并做了 `audit`。

**夹具改动不影响单 slot 行为。** `build` 改为调用 `build_slots(cap, fault, &[("slot", applicants)])`：plans 的 `slot_id` 仍为 "slot"，applicants 的顺序、`applicant_list`、`request_hash` 和 slots 数组形状与原来相同；`Reserve-slot-b` 时钟只在 `slot_spec.len() > 1` 时加入；Reserve 的 proofs 从「全体 applicants」改为「该 slot 的 applicants」，单 slot 时两者是同一集合；非 Reserve 的 witness 不变。删除的 `applicants` 字段没有其他读者。

## D. 记录项（LOW，不阻断）

1. **L-1** 负控里 `write(...).is_err()` 没有断言拒绝原因（这条路径缺 witness，很可能先以 MissingDependency 失败）。它只是旁证，关键断言在 `persist` 那一条。写法与既有 `commit_while_slot_awaits_policy_is_rejected_without_winner` 一致。
2. **L-2** `step_as` 仍固定使用 `witness(op)`，也就是 slot="slot"。双 slot 夹具如果以后经 `step_as` 走 Reserve slot-b，会得到错误的 proofs。现有用例没有这样用。
3. **L-3** REVIEW-AC123-01 L2 的另一半「E 尚未 Reserve 就发 B Commit」仍没有直接测试；本提交只补了插队这一半，这一点与作者的描述相符。
4. **L-4** `content_producer.rs:3748` 中 `group_facts_competing` 的文档注释仍写着「不给分型选根」，但它现在只是覆盖口径断言（:4306-4309），注释已经过时。

## 已执行的验证

- `node s_session/tests/build_tb01c_browser.cjs --check`：passed
- `node --test s_session/tests/test_tb02b_contract.cjs`：9/9 通过
- `python3 -m unittest test_tb02b_contract test_tb02c_verifier`（s_session/tests 下）：19/19 通过
- `node --test s_session/tests/test_q3_v3_validate_once.cjs`：1/1 通过
- bsp 白名单 grep 对拍：23 条，与白名单相等
- 未执行：任何 cargo 编译或测试（本机禁止），Rust 侧的通过状态以作者提供的结果为准。

## 追加：v3 冻结锁重冻（`720c0c5337`）

被审对象是 `720c0c5337`，它只改了两个测试文件：`pending_source_tests.rs` 的三个生成根，以及 `source_audit.rs` 的 full_some 读取轨迹（reads 与 F 各加 120，read/budget 两个摘要更新）。旧值都以注释保留，前缀与被删行一致。本节仍是静态核对，没有跑 cargo。

结论：**这次重冻正当，没有发现它掩盖 preintegrate 缺陷的路径**。两类锁的变化都能由「v3 规则升 new-bi/2 + #1405 内容变化」直接解释；`199caedfd0` 对这四个夹具不可达，`dccb69793c` 只改审计侧，不可能改变生成根。不过「+120」目前**没有可核的来源**，提交注释自己也写了「120 未逐节点分解」，这条记为 LOW-5，附上一个便宜的核验办法。

### (1) 重冻是否正当

**生成根必然变化，与内容无关。** v3 对象身份按六字段配方计算：`kind`、`fact_key`、`payload`、`input_refs`、`profile_id`、`rule_revision`（`content_producer.rs:1053-1069`）。`rule_revision` 是构造器持有的会话值，在 v3 中取 `V3_RULE_REVISION`。它从 `new-bi/1` 改为 `new-bi/2` 后，每个 TB-02 对象的 `obj3` 身份都会变，而生成状态里含有这些对象，所以只要规则修订变了，任何夹具的根都会变。这类冻结锁原本就分不清「规则修订带来的必然变化」和「内容缺陷」，判断时必须看下面的可达性分析。

**#1405 内容变化在这些夹具上的落点：**
- 两个「前三根同OHLC」夹具（乱序和顺序，冻结在 step=2）：两根相同的 K 线落进同一组，`high_sources` 和 `low_sources` 都有多个根。按 #1405，组载荷 `waiting_reasons` 新增证据标签 `equal_extreme_identity`，组请求和步骤请求不再带这条原因，CC-054 的 domain 改为 `established_direction_with_extreme_root_sets`。这些都是 main 上 #1405 的既定行为（见 A 节 tb02a_facts.rs 一行）。
- 「较早坐标修订」（step=4）：最终 a(0) 为 8/17/6/12，b(10) 为 8/16/6/12，a 包含 b，两根的低点同为 6，同样出现多根组和证据标签。
- 三个夹具的 `expected_groups` 断言（1/2/3）没有改，审计照常通过，说明分组和接收行为不变，变的只是发布原件和身份。

**`199caedfd0`（成员区间改到终点极值区末根）不可达：**
- 改动只在 `content_bi.rs` 遍历 `stroke` 的循环内（:1303-1371）。pending_source 的三个夹具最多 4 根 K 线、3 个组，连一笔都成不了，进不了这个循环。
- full_some 的夹具 `tb02b_oracle_rows()`（`s_structure_session.rs:5443`）共 14 根 K 线，只有 b2/b3 一处包含合并（向上合并：高点 12 只来自 b2，低点 9 只来自 b3）。其余高低点没有并列相等的情况，所以每个端点的 `extreme_roots` 都只有一个根，`last_extreme_root == first_extreme_root`，改动不生效。这与锁中 `groups=13`（14 根合并 1 处）、`objects=76` 不变一致。

**`dccb69793c`（source_pending 版本化）不影响生成根：**
- 它只改了 `source_pending.rs`（审计）和 `source_audit.rs`（把 rule 传给审计并新增测试），生成和发布路径一行没动，所以三个生成根的变化与它无关。
- 它会影响审计侧的读取轨迹，见 (2)。语义上它按具名合同精确区分 /1 和 /2：/2 只把 `equal_extreme_identity` 当证据，请求必须与「去掉证据标签后的真实等待原因」逐项相等，长度也要相等；`initial_direction_unsettled` 和 `direction_not_yet_established` 仍按等待处理；/1 仍要求原件整体相等。这与 main 上 #1405 的语义一致，不是放宽，也不构成 #799 的兜底。它附带的 `tb1405_request_reason_versions_keep_legacy_and_real_waits` 等负例在 5090 上没有出现在失败名单里。

### (2) +120 有无可核来源

目前没有可核来源。读数是在合并后实测的，提交没有做分解。静态上能找到两个候选来源：

1. **`dccb69793c` 的 /2 请求核对路径（可能性大）。** /1 路径对非空 reasons 只做一次 `request == evidence` 引用比较，不产生节点读。/2 路径（`request_reasons`）先查 extent，再 `elements(request)`，并对 evidence 逐项 `text_value` 后逐项比较。full_some 夹具在 /2 下走这条路径，凡是 waiting_count>0 的 CC-004 请求都会多出几次读，而且每次读对应一次 F，正好解释「reads 与 F 同加 120、计数不变」。
2. **v2 新笔载荷形状。** 端点从 `raw_position` 改为 `raw_first`/`raw_last`，pair 和 same_kind 多了 `evidence` 字段。但身份配方对 payload 只取 `Edge(digest)`（`content_producer.rs:1066`），审计对新笔对象只读语义头和 slot（`source_audit.rs:1654-1675`），所以这一项大概率不贡献读次数，只改变摘要。

**建议核验（不阻断，可在 5090 上做）：** 在 `813fc75fa5`（199 和 dccb 之前、已含 v3 new-bi/2）、`199caedfd0`、`dccb69793c` 三点各跑一次 `full_some_audit_keeps_frozen_read_and_budget_trace`，同时跑三个 pending_source 用例，记录 `Some full:` 和 `root=` 的 eprintln 输出。按上面的分析，预期是：
- 三点的 pending 根都等于新冻结值，说明 199 和 dccb 不影响发布；
- 813 与 199 的 full_some 读数相同，说明 199 不可达；
- 199 到 dccb 的读数差 = 120，说明 +120 全部来自版本化请求核对。

如果实测与预期不符，就说明有静态分析没覆盖到的路径，需要重审。得到结果后，把三点的读数写进重冻注释，替换「120 未逐节点分解」。

### 记录项

- **LOW-5** `source_audit.rs` 重冻注释中的 +120 没有来源；按 `docs/agents/stat-provenance.md` 的统计口径，应补上面的三点读数。不影响正确性：计数断言 `(76,13,11,14,0,0)` 没有变，审计通过，变化幅度约 0.07%。
- 关于 windows-5090 上 `v3::financial_session` 的 22 条失败：CI 本就 `--skip v3::financial_session`，并由 ci-macos.yml 在 macOS 上执行（见 A 节 ci.yml 一行）。作者称合并前后都失败，判为环境性，这个判断说得通；但本审查没有独立复现，仍以 macOS 作业的结果为准。

VERDICT: PASS
