# #1539 FU-17 41F 返修稿独审（41K）

## 结论：PASS，可提交用户 P 批准

40Y 的 B1–B4 均已实质闭合，R1–R8 全部 PASS。41F 保留三模式固定完整输入下的六面 H2，以 AT-13/C07.6 作为真实外效的附加约束；责任减少前件不再任选；成交、command/period 费用与账户查询已分别落到正确路径；只读可重算物化不再被误判为第二权威。问表可供用户对同一版本作决定。

下文三项是非阻断文书订正，不改变推荐答案或合同，无需另开研究或重复独审。主控发布时一并交作者订正。当前状态仍为 P / needs-approval；本结论只覆盖送批准备，不代替用户批准、实现验收或 #1279 完整验收。

## 范围与依据

- 输入：`/Volumes/AgentStorage/fu17-repair41f/` 的 draft、附件 A/B、archaeology.md/json、shadow-comment、review-response、REPORT；对照 `/Volumes/AgentStorage/fu17-review40y/REPORT.md`、review.json 和 40R 原件。
- 仓内证据基线：`/Volumes/AgentStorage/fu17-repair40r/frozen/`，冻结 main `9de18e606b2c0330162a83e2a030898b46c3690f`。以下 ADR/源码行号均指该冻结树，不声称它是当前最新 main。
- SPEC 原文：`/Volumes/AgentStorage/fu17-repair40r/gh/body1340.md`；#1411 原文与依赖快照：41F 的 `gh/`。直接回查决定 B1–B4 的原文，而非采用作者的“已闭合”自评。
- 独立性：GPT 家族非作者工位 `/root/fu17_review41k`，未参与 41F 起草；具体模型路由以父代理派发记录为准，不以旧稿作者的模型标注代替当前作者身份。
- 本轮 live 只读确认 #1539 为 OPEN，含 needs-approval；blocking 仍为 1540、1551、1535、1362、1366、1367、1365、1525，尚无 1425。查询使用 `env -u GH_TOKEN -u GITHUB_TOKEN -u GH_DEBUG gh`，退出码 0。

## B1–B4 逐项闭合

| 项 | 裁决 | 修订位置（相对 41F） | 原文与判断 |
|---|---|---|---|
| B1 H2 射程 | PASS，闭合 | draft.md:89–95、154、182–186、224–225；archaeology.md:240–256；shadow-comment.md:5、13 | ADR 0056:63/:128 保留三模式同模式 H1/H2；ADR 0038:47–56/:80–81/:89 定义六面；SPEC:347/:425–426/:436 保留完整输入与真实杀停。41F 明列输入修订、政策模型、时钟、种子、数值语义、故障调度，真实模式用受控场所夹具，各模式用自身恢复方法满足同一 H2，AT-13/C07.6 不替代它。附件与问表同义，无需改 ADR 0056。 |
| B2 减少前件 | PASS，闭合 | draft.md:81–87、150；archaeology.md:187–205；shadow-comment.md:5、12 | SPEC:295 的证据及实际参与账回执、:312 的“旧单不可再执行且历史累计完整”，INTERFACE-CONTRACTS:38 的“缺任何前件保原潜量”均保留。已撤旧单但迟到成交未齐的反例仍等待；adapter 只能说明如何证明，不能取消任何所需前件。既批合同已从用户待决项中剥离。 |
| B3 原件类别 | PASS，闭合 | draft.md:40–41、82、99–108、115、141、156–161、230–231；archaeology.md:341–356；shadow-comment.md:5、12、16 | ADR 0056:125 明定账户查询进 E；ADR 0032:33 明定 fill/command/period 费用及 du=0；tb03c_per_leg.rs:28–31 和 tb03c_allocation.rs:127 只支持成交投影要求。41F 共享来源/身份/因果外壳，成交及其具名前件单列，command/period 由 E 按既裁政策归属，账户查询不推进 F、不产生分配、不写 B。X 不获归属写权，原件级 allocation_id 与逐重 allocation_id 仍分开。 |
| B4 只读物化 | PASS，闭合 | draft.md:119–130、163；archaeology.md:304–312；shadow-comment.md:15 | ADR 0051:310–311 区分权威根链与可重算投影，允许物化并要求审计等于重放。41F 禁独立写映射与反写，不禁只读可重算物化；当前不交付属于实施范围。报告、草稿、问表不再互相冲突。 |

B1 只确认验收合同未被缩窄，没有宣告真实模式的任何杀停点已通过。#1411 若出现六面差异，仍须按其现有 AC 判 FAIL，不能用不重发约束冲销差异。

## R1–R8

| 规则 | 裁决 | 依据与复用范围 |
|---|---|---|
| R1 | PASS | 新增关键引文已回查冻结 ADR、SPEC、接口合同与源码；旧引文核验复用 40Y。引用区间的一个末行笔误见 N1，不改变论据。未把“1082 个行号存在”当语义证明。 |
| R2 | PASS | B1–B4 的反例已进入正文、附件和问表；三模式 H2、合取与回执、三类原件、合法物化均同义。未见改变既批前件或增加 X 归属写者。 |
| R3 | PASS | draft.md:202–218 为具体命令、测试、边与状态检查；新增 :209–211 对应此次修正。机械文书检查与下游行为验收分开，:236 的建议测试不伪称本轮已跑。七个 Rust 测试与真正恢复测试留给原获准执行面。 |
| R4 | PASS | 保留 #1323 六 Destination、#1279 完整验收、资金隔离/恢复，不把此票文书 PASS 当整体交付。账户权限归 #1540，X 实体规则归 #1330，FU16 recover_ns 归 #1769；无数值、资金或场所权限代填。 |
| R5 | PASS | 缺证保潜量、缺跳报缺项；恢复方法按预先固定 evidence_class 分述，不以严格失败触发宽松替代。三模式用同一 H2，未新增降档兜底。 |
| R6 | PASS | draft.md:222–235 补齐 #1411/#1366 与 #1400/#1425 消费范围。#1411:7/:11/:13 已有等价与不重发 AC，:24 明定父汇总非本片 blocker，不需把文书同步额外变成实施前置。41F 快照确认 #1366 被1539阻塞、1540→1382→1400；本轮 live 确认1539→1540和八条既有边。1539→1425仍是待用户批准后新增，见 N2。 |
| R7 | PASS | 本轮 live 确认 OPEN / needs-approval，draft.md:241 保留两轮 BLOCK 转 P 与用户 /graph-approve 门。shadow-comment.md:1 的独审结果占位由主控发布时填写，不构成批准。 |
| R8 | PASS | 40Y 已有换词复搜和14/6/4计数证据可复用；本轮新增反例原文已进入推理。JSON 对照40R仅Q4/Q9/Q10/Q12/Q13改变，新增18命中、4查询，分类A8/B5/C2；逐点question/citation/reasoning/recommended/conflict字符串全部见于对应Markdown。没有用机械计数取代语义核验。 |

## 三项非阻断文书订正

1. **N1 引用区间末行**：draft.md:40、108、280（以及 archaeology.md:350–351 的同款摘述）把单价 `unit_price` 包进 `tb03c_per_leg.rs:28–30`；冻结源码单价实际在 :31。最小修复：凡同时列出 manifest_hash、reported_quantity、unit_price 的出处写成 :28–31。draft.md:280 的核实命令本已读到31，事实内容正确，只是标签少一行。
2. **N2 新增边状态**：REPORT.md:25、review-response.md:20 写“#1425（边已有）”，与 draft.md:222/:217、REPORT.md:43 的待新增口径及本轮 live 不符。最小修复统一为“沿用40R拟新增边；批准落地时接 #1539 blocks #1425”。不更改当前任何边。此处是状态措辞订正，草稿已明确真正的后续动作。
3. **N3 推理不要过度概括**：draft.md:108、archaeology.md:351 把账户查询与 command/period 费用一概说成“没有单笔发令 manifest”。command 费用可有命令因果锚，现有依据不证明其必然没有 manifest。最小修复：“这些类别不都以单笔成交 manifest 为前件，不能一律套成交量/单价投影；command 费用按 command_id 归属，period 费用按既裁坐标。”推荐值 :104/:159 和问表 :16 已正确，不改选择。

## 复用证据与未运行项

复用 `/Volumes/AgentStorage/fu17-review40y/REPORT.md:93–103` 已确认的修复与证据：ADR 0038/0039 append-only 落点、simulation 工作态重跑而不删持久行、真实模式保外效、allocation_id 消费点及 fill_ref 绑定解释、不预占0058。复用40Y同目录 `r8-q3-maozhang.txt`、`r8-q4-position.txt`、`r8-q12-crosswalk.txt` 的14/6/4计数，以及其七份 `r8-*-alt*.txt` 换词证据、`spec-q9-h2.txt`、`r1-consumers.txt`。基线及相关未改部分一致，不重复扩测。

本轮未运行 Rust、CI、恢复实验或本机探针；未改仓库、作者稿、票/标签/边、服务或 FU16 工位。只产出本目录 REPORT.md 与 review.json。应用、恢复、隔离和下游成功实例仍按原有实施/验收门执行。

## 下一步

主控可采纳此独审结果；作者将 N1–N3 作同义文书订正，主控在同版影子问表填入“41K：PASS”，提交 Q4/Q12 两项 C、Q9/Q10/Q11/Q13/Q15 五项 B，以及 ADR 0038/0039 append-only 修订授权供用户批准。用户批准后才落正本、接1425边；所有六 Destination 和 #1279 完整验收保持原门槛。
