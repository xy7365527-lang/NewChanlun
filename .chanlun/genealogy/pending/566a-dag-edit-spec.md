# dag.yaml 修复执行规格（566a 配套）

供 Lead 派有 Edit 权限工位执行。genealogist 已诊断，无 Edit 权限故产出规格。
权威：`.chanlun/genealogy/settled/{566..575}-*.md` 的 frontmatter（已含编排者 2026-06-23 批准注释）。
571/572/573 编号最终收敛权威：settled/572 frontmatter `collision_note`（#74 Lead Option A 仲裁）= stopguard→571 / trinity→572 / process-population→573。

## A. nodes 段（dag.yaml 约 2967-3021）

按 frontmatter 权威，567-575 八条目改为：

| id | status | file | title 来源 |
|----|--------|------|-----------|
| 567 | 已结算 | settled/567-frozen-short-leg-unified-overrun-leverage-root.md | frozen short leg（不变） |
| 568 | 已结算 | settled/568-meta-rule-settlement-cognition-desync.md | settlement≠cognition（不变） |
| 569 | 已结算 | settled/569-meta-rule-role-boundary-collapse-rlhf-attractor.md | role-boundary collapse（不变） |
| 570 | 已结算 | settled/570-meta-rule-proxy-vs-ontology-self-correction-impossibility.md | proxy vs ontology（不变） |
| 571 | 已结算 | settled/571-meta-rule-stopguard-block-target-vs-responsibility.md | **Stop-Guard 阻断粒度⊥pending责任方**（从原572节点取） |
| 572 | 已结算 | settled/572-close-short-bullbear-chainbreak-trinity.md | **平空=牛熊切换=区间套链破坏三位一体**（从原571节点取） |
| 573 | 已结算 | settled/573-agent-process-population-compact-layer.md | agent 进程 population compact 层（不变） |
| 574 | 已结算 | settled/574-bsp-formal-solution-of-confirmation-lag.md | 买卖点=确认滞后形式化解（不变） |
| 575 | 已结算 | settled/575-clean-slate-rebuild-dual-generation-contention.md | clean slate 重建（不变） |

**删除 `id: TBD` 孤儿节点**（原 2988-2993，stopguard 临时占位，已被 571 取代）。

## B. edges/depends_on 段（dag.yaml 约 5991-6108）

权威 frontmatter depends_on：
- 567: 556, 563, 561, 572
- 568: 137, 562, 566
- 569: 137, 032, 187, 564, 565
- 570: 187, 010, 047
- 571(stopguard): 097, 548, 564
- 572(trinity): 567, 547, 561
- 573: 407, 562, 565, 566
- 574: 537, 540, 558, 573, 407, 565
- 575: 573, 407, 565

修复动作：
1. **删除所有 `from: ''` 空 id 边**（5999/6001/6003/6011-6022/6099/6101）——碰撞期残留，其 to 目标已被填好 id 的 568/569/570/573 边覆盖，是重复孤儿。
2. **`from: TBD` 边（6045-6050，to: 097/548/564）→ 改 `from: '571'`**（stopguard 真实依赖）。
3. **`from: '571'` 边（6051-6062）拆分**：保留 to: 097/548/564（=stopguard）；删除 to: 567/547/561（=trinity，归 572）。
4. **`from: '572'` 边（6071-6082）拆分**：保留 to: 567/547/561（=trinity）；删除 to: 097/548/564（=stopguard，已由 TBD→571 提供）。
5. **`from: '567'` 边 6083-6086**：`567→572` 保留（frontmatter 含 572）；`567→571` 删除（frontmatter depends_on 不含 571）。
6. 569 edges `from:569→137/032/187/564/565`（6029-6038）已对，保留。570/568/573/574/575 同理已对，仅删重复空 id 边。

## C. negates 段（dag.yaml 约 6282-6302）

权威：571 negation_form=expansion（hook 阻断对象维度）；572 negation_form=aufhebung（negates 开空门控补丁）。
1. **`from: '571'→开空门控补丁` 边（6287-6290）删除**——该 negates 属 572，6291/6299 已有正确的 `from:'572'` 边。
2. **新增 571 真实 negates 边**：571 expansion 扩展出"hook 阻断对象维度"（097/548/564 只覆盖注入内容维度，未覆盖阻断对象）。按 frontmatter 行 11 的 negation_form 描述补 negates 文本边，或登记为 expansion 型 topo_effect（split:097-hook-action-dimension）。
3. 6295-6298 `from:'567'→reading_b前提` 是 567 的真实 negates（separation），保留。

## D. block-topology 映射（任务第1项）——结构性阻塞，非本规格范围

566-575 入 block-topology 依赖 `map_genealogy_to_blocks.py`，被 **549 号 freeze**（relations.jsonl 未实体化 LFS 指针，须操作者 `git lfs pull` 解冻）。映射动作在管线解冻前阻塞。本规格仅修 dag.yaml 索引层，使解冻后映射能取到正确的 status/file/edges。

## E. meta.json（下游推论 impl-566a-2）

`last_mapped_genealogy: 530` 失真（id_mapping 实际到 565）。走 559 ceremony-scan-completeness skill 三问检测：有消费者→同步更新为 565；无消费者→删字段（090 声明膨胀）。
