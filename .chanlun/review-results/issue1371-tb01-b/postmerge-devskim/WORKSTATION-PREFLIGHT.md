# #1371 合入后工位清理预检

快照：2026-09-12T17:05:18.173112+00:00。固定交付 `750f933bddffc8251a880555b9beae2cdce75690`（PR [#1449](https://github.com/xy7365527-lang/NewChanlun/pull/1449)）；PR/CI/main由根处理，本报告不预写合入或清理成功。

**结论：13 个本票工位中，4 个当前干净；另 9 个只有本票登记/旧报告在制品，已经逐字保全，没有发现产品在制品。当前票仍 OPEN，所有工位暂留；满足正式合入与关票收尾条件后，以下精确路径可以按条件清理。**

保全入口：[MANIFEST.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/POSTMERGE-WORKSTATION-PRESERVATION/MANIFEST.json)。仅保存8份旧native工位roster和原B的review-r6.json，每份含原字节、git diff、HEAD/状态及SHA。共9份原文件，275381 bytes；没有打包源码、数据库或其他票文件。

## 旧R6不丢失处置

原文件：[review-r6.json](/Users/silencehan/Projects/NewChanlun-1323-spec-publication/.sandcastle/worktrees/codex-1323-tb01-b-1371/.chanlun/review-results/issue1371-tb01-b/review-r6.json)，**261509 bytes**，SHA256 `e3e26cebd36c60ad68c6bc011e3c19fc2fad37a21abc3e00f313ed0e4d7218ff`。与固定750f提交的 [EVIDENCE-INDEX.json](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/native-codex-r7-r12/EVIDENCE-INDEX.json) `/files/104` 所指附件完整解码后逐字相等；附件位置为 [EVIDENCE-ATTACHMENTS.json](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/native-codex-r7-r12/EVIDENCE-ATTACHMENTS.json) `/artifacts/e3e26cebd36c60ad68c6bc011e3c19fc2fad37a21abc3e00f313ed0e4d7218ff`。

另外原样备份至 [review-r6.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/POSTMERGE-WORKSTATION-PRESERVATION/original-b-review-r6/review-r6.json)。原R6保持未完成名分，#1448同ID迁移义务不因该备份或R12通过消失。合入确认后，原B树里的这一份可按重复副本处置；不回写旧报告，不要求重审它，不删除D中的保全与恢复材料。

## 工位精确清单

| 工位路径 | HEAD | 当前状态 | 合入后处置 |
|---|---|---|---|
| `/Users/silencehan/Projects/NewChanlun-1323-spec-publication/.sandcastle/worktrees/codex-1323-tb01-b-1371` | `4922543038fbcfef1a4d711a306708ace3f62e88` | 未追踪旧R6，已保全且与入仓附件同字节 | 先确认这一个已保全登记/重复报告副本处置，再按干净工位移除 |
| `/Users/silencehan/Projects/NewChanlun-1371-codex-r12` | `750f933bddffc8251a880555b9beae2cdce75690` | 干净 | 正式关票收尾时按干净工位移除 |
| `/Users/silencehan/Projects/NewChanlun-1371-native-acceptance` | `8c868f968c5f937870a4305dca05a0e0162b5131` | 仅roster旧工位登记修改，已保全 | 先确认这一个已保全登记/重复报告副本处置，再按干净工位移除 |
| `/Users/silencehan/Projects/NewChanlun-1371-native-r10-acceptance` | `3ddb5c2b7f9f250861640d40d779d1a6a5fd8955` | 干净 | 正式关票收尾时按干净工位移除 |
| `/Users/silencehan/Projects/NewChanlun-1371-native-r11-acceptance` | `4922543038fbcfef1a4d711a306708ace3f62e88` | 干净 | 正式关票收尾时按干净工位移除 |
| `/Users/silencehan/Projects/NewChanlun-1371-native-r2-acceptance` | `0298cf429804c611d6ee5f691a1f36ee2a720fba` | 仅roster旧工位登记修改，已保全 | 先确认这一个已保全登记/重复报告副本处置，再按干净工位移除 |
| `/Users/silencehan/Projects/NewChanlun-1371-native-r3-acceptance` | `a9acf50ac3a700450108040ca1b2c019675df3c9` | 仅roster旧工位登记修改，已保全 | 先确认这一个已保全登记/重复报告副本处置，再按干净工位移除 |
| `/Users/silencehan/Projects/NewChanlun-1371-native-r4-acceptance` | `58dab231e2210e1ba1e9634c936807d8492e218b` | 仅roster旧工位登记修改，已保全 | 先确认这一个已保全登记/重复报告副本处置，再按干净工位移除 |
| `/Users/silencehan/Projects/NewChanlun-1371-native-r5-acceptance` | `8b726a57f98974c451ad0490066150afb9fe8e38` | 仅roster旧工位登记修改，已保全 | 先确认这一个已保全登记/重复报告副本处置，再按干净工位移除 |
| `/Users/silencehan/Projects/NewChanlun-1371-native-r6-acceptance` | `6eeed76e3e12f1ffc3fdc62e23e60f2a3084c37b` | 仅roster旧工位登记修改，已保全 | 先确认这一个已保全登记/重复报告副本处置，再按干净工位移除 |
| `/Users/silencehan/Projects/NewChanlun-1371-native-r7-acceptance` | `4a3da3787bdd6e783f3c7eee63dec544b7dc3e68` | 仅roster旧工位登记修改，已保全 | 先确认这一个已保全登记/重复报告副本处置，再按干净工位移除 |
| `/Users/silencehan/Projects/NewChanlun-1371-native-r8-acceptance` | `65fb5e72bbee1de6656ba4b902dd506ae526195f` | 仅roster旧工位登记修改，已保全 | 先确认这一个已保全登记/重复报告副本处置，再按干净工位移除 |
| `/Users/silencehan/Projects/NewChanlun-1371-native-r9-acceptance` | `667ce698eacd8c8f50ec5846f24c779b492010a8` | 干净 | 正式关票收尾时按干净工位移除 |

13个HEAD都已是固定750f的祖先，没有这些工位独占的未交付commit。原B另有ignored `rust/target/` 和 `s_session/tests/__pycache__/`，其余12个工位没有ignored条目；本次未读取或打包这些构建缓存。

两条本票本地分支是 `refs/heads/codex/1323-tb01-b-1371` 和 `refs/heads/codex/1371-codex-r12`。待根核合入后无独占提交，并先解除对应worktree，才作为同一关票收尾对象；远端PR分支是否自动删除由根发布流程处理，本工位未操作任何ref。

八份roster的差异均仅新增各自#1371验收工位登记。r6–r8同时写#1323，是父图归属，不是另一个未交付产品工作。原文件已有的#1370登记是历史上下文，未被这次修改。完整diff已保全，不把登记上的历史“running/待测”当作今天仍有活动测试的证明。

## 进程与旧准备

本次按本票路径及服务名筛选PID/argv，必要时核cwd，命中本票native/readonly/故障输送进程 **0** 个。端口18962监听查询 exit=1，输出为空。已留存的 [SERVICE-STOP-RECEIPT.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R12-GUI-PREP/SERVICE-STOP-RECEIPT.json) 与 [STOPPED.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R12-GUI-PREP/runs/gui-1/STOPPED.json) 记载服务退出、28请求以及先前无监听检查。无需停止任何本次已确认存活的服务；清理若推迟，应只重查相关PID/端口，禁止按进程名广杀。

旧准备目录 `/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/review-r7-prep` 及其 `host-v2` 是单票Prime旧入口。host-v2 PREPARATION仍写 `READY_FOR_APPROVED_RUN_NOT_EXECUTED`，属于当时准备状态；现已被用户选择Codex原生工作替代。只保留历史，不调用run/resume或重新完善。其与D整体证据一起保留，不作为需要恢复的服务。

## 票体、报告入仓和逐提交声明

- 本次只读 [#1371](https://github.com/xy7365527-lang/NewChanlun/issues/1371)：OPEN，8项AC仍未勾选，评论1条；不能据未勾选否定已发生验收，也不能在未完成main/CI/工位收尾前宣称关票成功。
- 固定750f尾提交新增21份本票报告/附件文件，逐一与FINAL-DELIVERY登记的bytes/SHA和Git对象吻合；没有产品或测试变化。报告实体、独评、根验收、GUI与必要附件已真实tracked。EVIDENCE-INDEX有423条，去重附件285份。本次只新增旧R6字节核对，其余沿已完成验收复用。
- 仓内 [COMMIT-DECLARATIONS.json](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/native-codex-r7-r12/COMMIT-DECLARATIONS.json)逐项覆盖前21提交；仓外 [FINAL-DELIVERY.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/FINAL-DELIVERY.json) `/last_commit/declaration` 明确第22项 `750f933bddffc8251a880555b9beae2cdce75690` 为纯报告豁免。两者合起来覆盖22项，不能声称仓内声明本身已有22行。正式resolution应现场引用全22项或明确“前21项见仓内清单；750f为21文件纯报告豁免”。
- 已入仓根验收仍为历史 `PASS_B_BOUNDED_PREMERGE`，R12 FINAL-CHECK为PASS、候选e5；末尾21份报告与该产品面完全一致。待主线落地后在新resolution写实际main SHA/CI，不回写冻结报告里的“尚未发布/尚待批准”历史文字。
- 不要求重新评审或重跑已通过检查。本票只到B分配子域；父#1323、后续C和#1448等不能随本次工位清理关掉。

## 必须保护的范围

- 用户主仓 `/Users/silencehan/Projects/NewChanlun`：本次HEAD `6df8d1921c72c84da90987f8f6c02ebc11fe6aac`，有21处跟踪更改和333条未追踪记录。禁止以收尾为由reset/clean/checkout覆盖；本次只读状态，没有读取或备份其在制品内容。
- `/Users/silencehan/Projects/NewChanlun-1323-spec-publication` 是父图工位，不属于本票可删除集合。原B子路径嵌套其下，不能递归删除父目录。
- 除上述13条以外的全部66个已注册worktree均排除；详细路径在伴随JSON `other_worktrees_outside_scope`，其他研究/实施票不作旁带清理。
- 整个仓外 `/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z` 必须保留，尤其本次PRESERVATION、review-package-r7、reports、recovered、native/GUI原件与旧准备。此预检不授权删除整包；月度报告归档另按世代宪法处理。

## 清理条件与规则

[世代宪法](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/docs/agents/generation-constitution.md:20)明确：票仍开或存在未提交在制品即“在跑”；票关、工作区干净且无未合入commit才是“残骸”。[交付纪律](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/docs/agents/delivery-discipline.md:15)要求关票收尾处理对应工位。当前阶段只是完成可审阅的保全与候选清单，不把两条规则变成提前删除在跑工位或要求重复验证的理由。

1. 实际main已包含被批准的750f交付且PR/主线CI事实由根接回，交付内容无新增改变。
2. 关票收尾时再次只读核工位HEAD/状态；若多出本快照以外文件/commit/进程立即从清理集合剔除，不能广泛force。
3. 九份指定在制品已在PRESERVATION逐字保全并可核SHA；旧R6另已在750f附件中。先明确历史登记/重复报告副本的处置，再清理对应工位；不将历史登记夹带回本次已批准交付。
4. 按世代宪法在跑/残骸和交付纪律关票清理一并收尾：当前OPEN时全部登记保留，本预检不把尚未关票清理预写为已清。

本工位仅写本预检两文件与根指定的九文件保全目录，没有删除/改写工作树、改ref、提交、GH写入、测试或服务操作。

完整机器记录：[POSTMERGE-WORKSTATION-PREFLIGHT.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/POSTMERGE-WORKSTATION-PREFLIGHT.json)。
