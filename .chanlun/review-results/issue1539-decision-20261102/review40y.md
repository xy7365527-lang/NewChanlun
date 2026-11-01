# #1539 FU-17 返修稿独审（40Y）

## 结论：BLOCK，问表暂不能送批

旧两轮指出的恢复修订落点、主要漏引/行号、X 归属双写与 ADR 编号问题已实质修正；不撤销这些进展。本轮仍有四项影响合同的缺陷：H2 射程缩窄遗漏 ADR 0056，累计证据被解释为任取其一，所有原件被压成成交投影，以及只读物化被误判为第二权威。先由原作者修稿，主控再验收；不能把这些事实/合同矛盾交给用户凭一声“全批”解决。

## 范围与证据

- 输入：`/Volumes/AgentStorage/fu17-repair40r/` 的 draft、archaeology、shadow-comment、review-response、REPORT、frozen 与 gh 原件；旧轮次来自 `/Volumes/AgentStorage/decisions/1539/`。
- 语义审查基线：冻结 main `9de18e606b2c0330162a83e2a030898b46c3690f`；只读现场核实 origin/main 相同，工作目录 HEAD 是 `c24b0a870f856d61a0d6b1a47f5a96d59d1129cb`，不混作同一基线。
- 作者：claude-opus-5-5（作者报告标注）；审查：父代理指定的 GPT/Astra 家族非作者工位；未参与起草。
- GitHub live 只读确认 #1539 为 OPEN、带 needs-approval，blocking 为 1540、1551、1535、1362、1366、1367、1365、1525。取证命令清除 GH_TOKEN/GITHUB_TOKEN/GH_DEBUG，输出保存在本目录。
- 未运行 Rust、CI 或恢复实验，未改草稿/正本/票/边/标签，未触碰 FU16 System 实施。此处验的是 P 草稿，不是实现通过。
- 使用 Prove It Works 的决定：不接受作者“114 条命中/976 个行号检查通过”作为语义结论，而回查 SPEC、ADR 和真实消费点。旧审 PASS 不覆盖本轮发现的反例。

## R1–R8

| 规则 | 结论 | 依据 |
|---|---|---|
| R1 | PASS | 本轮回原文核对关键引文、旧审点名行号与原件级 allocation_id 消费位置，所查摘引成立；未将作者 976 个行号存在性检查等同语义正确。R2 中的问题是从真引文推出了过宽结论。 |
| R2 | BLOCK | B1–B4：H2 射程与 ADR 0056 同模式合同未对齐；C06.2 斜线不能推翻 C07.8 明确合取；所有原件只准成交投影排除账户/非 fill 费用；只读物化被误判为第二权威。 |
| R3 | PASS | draft.md:195-208 各项为命令、测试名、文件存在或票状态，输出条件二值。本轮只审文书可执行性，依授权未运行 Rust/CI；通过不代表这些验收已经执行，也不补足 B1 的验收范围。 |
| R4 | PASS | 工作仍在唯一真账、恢复与 crosswalk 的 Destination 内；现稿已是 P，未借 N-1 自动落地，资金/量档/cap/θ 和场所权限未被代填。错误合同推论另在 R2 阻断。 |
| R5 | PASS | 未发现新增“严格失败再走宽松判据”的 if-else 兜底。per_leg/netted 由冻结事实选定；缺证保潜量与缺跳报缺项无宽松替代。B2 是证据前件被错误解释，不伪称它已实现运行时降档。 |
| R6 | BLOCK | live 核实八条 blocking 边与草稿一致，但 H2 改动漏 ADR 0056:63/:128 和 #1411 的验收消费；Q13 的账户/period 费用路径未形成完整消费映射。需补影响表并说明复用传递边或新增边的理由，不机械给每个受影响票加边。 |
| R7 | PASS | 两轮 BLOCK 后保持 P/needs-approval 正确；live #1539 OPEN，needs-approval 存在。本轮不回退为 T，不清旧轮次。 |
| R8 | BLOCK | 已独立复跑毛账 14 行、物理持仓/场所净仓/账实 6 行、crosswalk 4 行，旧行号与计数缺陷修正成立；对全部 B/C 点做了换词复搜。新发现 ADR 0056 同模式 H2 与物化投影反例未进入关键推理，已列 C07.8/账户查询/费用类别也未被正确消费，因此考古仍未支撑待批推荐。 |

## 必须返修的四项

### B1：Q10 把 H2 的适用域缩到 simulation，却遗漏 ADR 0056 的同模式合同

**位置**：`/Volumes/AgentStorage/fu17-repair40r/draft.md:91,178,214`；其他同步位置： shadow-comment.md:9。

**原合同证据**（以下路径相对输入根目录）：

- frozen/docs/adr/0056-three-mode-shared-core-adapter-boundary.md:63 明定同模式沿用 ADR 0038 H1/H2，:128 将此交给 #1411。
- frozen/docs/adr/0038-dual-run-determinism-recovery-seek-asknown.md:81,89 定义六面恢复等价 H2；gh/body1340.md:347,426,436 保留固定完整输入、各面硬判和真正杀停重启。

**影响**：保留真实外效是正确修订，但不推出真实模式免做固定输入下的同模式恢复等价。仅保留 ADR 0051 根链幂等和 AT-13 不等价于六面 H2；“本修订不增删验收项”与实际适用域收窄不符。

**最小修复**：保留并明确三模式在固定输入/时钟/调度和受控场所夹具下的 H2；把真实在途外效的 AT-13 写为相应恢复约束，不替代六面 oracle。若确需改变 H2 射程，显式列为 P 合同变更，补 ADR 0056:63/:128 的同步修订及 #1411 消费映射，不再声称验收未改。

### B2：Q9 由斜线推出任取其一，遗漏已批 C07.8 的明确合取

**位置**：`/Volumes/AgentStorage/fu17-repair40r/draft.md:81-84,143`；其他同步位置： archaeology.md:Q9； shadow-comment.md:8。

**原合同证据**（以下路径相对输入根目录）：

- gh/body1340.md:295 C06.2 说不可再执行/累计完整性的证据及参与账持久回执。
- gh/body1340.md:312 C07.8 明定非原子 amend 路径：等待旧单不可再执行且历史累计完整，再以族/版本 CAS 唯一转移剩余量。
- frozen/.chanlun/review-results/issue1339-r2-spec-20260909/view/spec/INTERFACE-CONTRACTS.md:38 缺任何前件保原潜量。

**影响**：撤回“所有情况都要求 Σ逐笔＝累计且终态”是正确的；但不能反向将不可再执行和累计完整变成可互换证据。旧单已撤、迟到成交尚未收齐的反例中，adapter 宣告撤单终态只能证明不能再执行，不能证明历史累计完整。

**最小修复**：删“不可再执行或累计完整性之一”及从斜线推 OR 的推理。按具体责任/动作列出所需证据，明确保留 C07.8 非原子转移的两项前件和 C06.2 实际参与账回执；adapter 能力声明只能说明如何证明，不能选择取消前件。这是既批合同，不向用户重问。

### B3：Q13 将所有场所原件强制为成交投影，排除了账户查询与非 fill 费用

**位置**：`/Volumes/AgentStorage/fu17-repair40r/draft.md:80,97-100,134,149-154`；其他同步位置： shadow-comment.md:8,12。

**原合同证据**（以下路径相对输入根目录）：

- frozen/docs/adr/0056-three-mode-shared-core-adapter-boundary.md:125 账户身份、订单、持仓、回报前沿查询结果以场所 adapter 原件进 E。
- frozen/docs/adr/0032-fill-allocation-policy.md:33 费用 cause 有 fill/command/period，du=0 合法。
- frozen/rust/src/economic_session/tb03c_per_leg.rs:8-74 的成交形要求 manifest_hash、reported_quantity、unit_price 等；tb03c_allocation.rs:126-151 按成交 manifest 核验。

**影响**：无单笔发令 manifest 的账户持仓快照/period 费用是既有合法输入，不能为满足全局字段规则伪造“所发命令”、成交量或归属。Q4 要账户回读只作对账，Q9 又把账户回读全经成交 F 函数归一，范围也相撞。已有投影消费点能支持成交形槽位设计，不能证明覆盖所有原件类别。

**最小修复**：明确共享的是来源/身份/因果外壳，按既有事实类别限定 payload：命令相关成交及其具名费用/更正才用冻结 manifest 投影；账户查询不推进成交 F、不覆写 B，command/period 费用保留既裁因果路径。同步 Q9、Q13、裁定表、问表及 #1400/#1425 消费范围；不新增 X 归属写者。

### B4：Q12 把只读物化也判为第二写者，错误排除合法候选

**位置**：`/Volumes/AgentStorage/fu17-repair40r/draft.md:118,123,156`；其他同步位置： shadow-comment.md:11； REPORT.md:43。

**原合同证据**（以下路径相对输入根目录）：

- frozen/docs/adr/0051-per-bar-domain-event-identity-lifecycle-contract.md:310-311 区分权威根链与可重算投影，允许投影物化且审计必须等于重放。
- 作者 REPORT.md:43 自己承认未来物化汇总可以是可重算投影；draft.md:118 却以第二写者为由一概排除汇总表。

**影响**：独立修改映射真相的汇总表应禁止；只读、可重建、不得反写的汇总索引并不自动获得第二权威。现稿把一种合法实现当违规排除，与 ADR 0041 及其自己的报告不一致。

**最小修复**：将禁令收窄为禁止第二权威写入口，写清逐跳根链仍唯一、汇总只能是可重算只读投影。若当前不交付物化层，明确是实施范围；若本票确要裁存储/读策略，则保留合法候选并依 ADR 0041 给版本化默认值与代价。

## 问表怎样恢复可用

保持现有 A 类 8 点不重问，保持 P 与原两轮记录。Q9 先恢复 C07.8/C06.2 的既有证据前件；Q13 先分清成交、账户查询与 command/period 费用；Q10 先对齐 ADR 0056 的 H2 射程与受影响实施票。上述都是作者必须查清、写清的合同问题。Q12 删去“只读物化必然双写”的错误排除后，才按 ADR 0041 呈现真正需要选择的部分。

`shadow-comment.md:8-12` 必须随修稿同步，尤其其 Q9 省略正文的 OR，不能靠这项省略掩盖正文缺陷。用户批复需针对同一版本的问表、草稿与修订附件。

两个文书小项一并修：`draft.md:171` 的原件目录仍指旧 decisions/1539，落地应归档返修包或改准确指针（作者已识别）；`REPORT.md:37-45` 将 Q10 放在“两条 C 决定”叙述里，而草稿/考古/问表列 B，应统一分类。四空间/四跳列出五个节点的措辞宜改成“四跳关系”，不要借措辞改变映射合同。

## 已确认的旧问题修复

- 恢复规则实际给出 ADR 0038/0039 的 append-only 修订原文，ADR 0030:48 只作同向先例；不再只在新 ADR 单方面改旧正本。
- simulation 回退解释为工作态重跑，已持久 append-only 行不删；experiment/production 保留已持久 X/E/B 外效、不自动重发，方向成立。
- Q13 不再让 X 复制归属字段；当前原件 allocation_id 作为派生种子、具名前件身份与 E 派生的逐重分配 id 区分。tb03c_corrections.rs:125 比的是 fill_ref 与原 fill 的槽，不是要求当前更正事件和原 fill 共享 external_fact_id；此点不报错。
- 主要旧行号、毛账命中数及 ADR 0033:31/0030:48/0039:289 的漏引已补。三项独立复跑得到 14/6/4，与返修稿一致。
- 不预占 0058，保留两轮 BLOCK 后的 P/needs-approval 历史，正确。

## 可复查证据与交棒

本目录的 `review.json` 是机器可读 R1–R8 与 B1–B4。查询原始输出：`r8-q3-maozhang.txt`、`r8-q4-position.txt`、`r8-q12-crosswalk.txt`；换词证据：`r8-q9-alternative.txt`、`r8-h2-alternative.txt`、`r8-q13-alternative.txt`、`r8-q12-alternative.txt`、`r8-q4-alt.txt`、`r8-q11-alt.txt`、`r8-q15-alt.txt`；`spec-q9-h2.txt` 给出已批 C07.8 与恢复验收原文，`r1-consumers.txt` 保存 allocation_id 字符串消费位置。查询按冻结树执行，live 票状态另存，不混用。

下一动作：主控把 B1–B4 交回原作者，在原返修工作区修草稿、考古与问表；无需回退 FU16 System 线，也不必为既有合同再开用户数值问题。本轮完成条件是报告与 JSON 落盘可读，最终采纳归主控。
